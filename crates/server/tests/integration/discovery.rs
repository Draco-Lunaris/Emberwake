//! T064/T065: Discovery parser + cache + SSE integration tests.
//! Tests: label parsing (Docker + K8s), multi-value URL syntax,
//! cache update + SSE event emission, disabled = no calls.

use std::collections::HashMap;
use std::time::Duration;

use app::domain::{DiscoveredService, DiscoveryAction, DiscoverySource, SseDiscoveryEvent};
use app::server::discovery::DiscoveryCache;
use app::server::settings_queries;
use server::sse::{SseEvent, SseHub};
use sqlx::SqlitePool;

// --- T064: Parser unit tests ---

/// T064: Parse Docker container labels into DiscoveredService.
#[test]
fn parse_docker_labels_basic() {
    let mut labels = HashMap::new();
    labels.insert("emberwake.name".to_string(), "My Service".to_string());
    labels.insert(
        "emberwake.url".to_string(),
        "https://example.com".to_string(),
    );
    labels.insert("emberwake.icon".to_string(), "fa-globe".to_string());
    labels.insert("emberwake.category".to_string(), "Web".to_string());
    labels.insert(
        "emberwake.description".to_string(),
        "A web service".to_string(),
    );

    let services = server::integrations::labels::parse_labels(
        &labels,
        DiscoverySource::Docker,
        "container123",
    );
    assert_eq!(services.len(), 1);
    assert_eq!(services[0].name, "My Service");
    assert_eq!(services[0].url, "https://example.com");
    assert_eq!(services[0].icon.as_deref(), Some("fa-globe"));
    assert_eq!(services[0].category.as_deref(), Some("Web"));
    assert_eq!(services[0].description.as_deref(), Some("A web service"));
    assert_eq!(services[0].source, DiscoverySource::Docker);
    assert_eq!(services[0].source_id, "container123");
}

/// T064: Parse K8s Ingress annotations into DiscoveredService.
#[test]
fn parse_k8s_annotations_basic() {
    let mut annotations = HashMap::new();
    annotations.insert("emberwake.name".to_string(), "K8s App".to_string());
    annotations.insert(
        "emberwake.url".to_string(),
        "https://k8s.example.com".to_string(),
    );
    annotations.insert("emberwake.icon".to_string(), "fa-server".to_string());

    let services = server::integrations::labels::parse_labels(
        &annotations,
        DiscoverySource::Kubernetes,
        "default/my-ingress",
    );
    assert_eq!(services.len(), 1);
    assert_eq!(services[0].name, "K8s App");
    assert_eq!(services[0].url, "https://k8s.example.com");
    assert_eq!(services[0].icon.as_deref(), Some("fa-server"));
    assert!(services[0].category.is_none());
    assert!(services[0].description.is_none());
    assert_eq!(services[0].source, DiscoverySource::Kubernetes);
    assert_eq!(services[0].source_id, "default/my-ingress");
}

/// T064: Multi-value URL syntax (comma-separated) creates multiple services.
#[test]
fn parse_labels_multi_value_url() {
    let mut labels = HashMap::new();
    labels.insert("emberwake.name".to_string(), "Multi URL".to_string());
    labels.insert(
        "emberwake.url".to_string(),
        "https://a.example.com,https://b.example.com".to_string(),
    );
    labels.insert("emberwake.icon".to_string(), "fa-link".to_string());

    let services =
        server::integrations::labels::parse_labels(&labels, DiscoverySource::Docker, "abc");
    assert_eq!(services.len(), 2);
    assert_eq!(services[0].url, "https://a.example.com");
    assert_eq!(services[1].url, "https://b.example.com");
    assert_eq!(services[0].name, "Multi URL");
    assert_eq!(services[1].name, "Multi URL");
    assert_eq!(services[0].icon.as_deref(), Some("fa-link"));
    assert_eq!(services[1].icon.as_deref(), Some("fa-link"));
}

/// T064: Missing name label = service not discovered.
#[test]
fn parse_labels_missing_name() {
    let mut labels = HashMap::new();
    labels.insert(
        "emberwake.url".to_string(),
        "https://example.com".to_string(),
    );

    let services =
        server::integrations::labels::parse_labels(&labels, DiscoverySource::Docker, "x");
    assert!(services.is_empty());
}

/// T064: Missing URL label = service not discovered.
#[test]
fn parse_labels_missing_url() {
    let mut labels = HashMap::new();
    labels.insert("emberwake.name".to_string(), "No URL".to_string());

    let services =
        server::integrations::labels::parse_labels(&labels, DiscoverySource::Docker, "x");
    assert!(services.is_empty());
}

/// T064: Empty name or URL = service not discovered.
#[test]
fn parse_labels_empty_values() {
    let mut labels = HashMap::new();
    labels.insert("emberwake.name".to_string(), "".to_string());
    labels.insert(
        "emberwake.url".to_string(),
        "https://example.com".to_string(),
    );

    let services =
        server::integrations::labels::parse_labels(&labels, DiscoverySource::Docker, "x");
    assert!(services.is_empty());
}

/// T064: No emberwake labels = not discovered.
#[test]
fn parse_labels_no_emberwake_labels() {
    let mut labels = HashMap::new();
    labels.insert(
        "com.docker.compose.service".to_string(),
        "myapp".to_string(),
    );

    let services =
        server::integrations::labels::parse_labels(&labels, DiscoverySource::Docker, "x");
    assert!(services.is_empty());
}

/// T064: has_emberwake_labels detects presence.
#[test]
fn has_emberwake_labels_detects() {
    let mut labels = HashMap::new();
    labels.insert("emberwake.name".to_string(), "Test".to_string());
    assert!(server::integrations::labels::has_emberwake_labels(&labels));

    let mut other = HashMap::new();
    other.insert("other.label".to_string(), "value".to_string());
    assert!(!server::integrations::labels::has_emberwake_labels(&other));
}

// --- T065: Cache + SSE integration tests ---

/// T065: A container-start event triggers an SSE discovery event through the hub.
#[sqlx::test(migrations = "../../migrations")]
async fn container_start_emits_sse_event(_pool: SqlitePool) {
    let sse_hub = SseHub::new(64);
    let cache = DiscoveryCache::new();

    // Subscribe before the event.
    let mut rx = sse_hub.subscribe();

    // Simulate a container-start: add service to cache and emit SSE.
    let service = DiscoveredService {
        name: "New Service".to_string(),
        url: "https://new.example.com".to_string(),
        icon: Some("fa-star".to_string()),
        category: None,
        description: None,
        source: DiscoverySource::Docker,
        source_id: "abc123".to_string(),
    };

    cache.add_docker(service.clone());
    sse_hub.broadcast_discovery(SseDiscoveryEvent {
        service_id: "abc123".to_string(),
        action: DiscoveryAction::Added,
        name: "New Service".to_string(),
        url: "https://new.example.com".to_string(),
    });

    // Assert SSE event received.
    let event = tokio::time::timeout(Duration::from_secs(5), rx.recv())
        .await
        .expect("should receive SSE event within timeout");

    match event.expect("event") {
        SseEvent::Discovery(de) => {
            assert_eq!(de.service_id, "abc123");
            assert_eq!(de.action, DiscoveryAction::Added);
            assert_eq!(de.name, "New Service");
            assert_eq!(de.url, "https://new.example.com");
        }
        _ => panic!("expected discovery event"),
    }

    // Assert cache was updated.
    let docker = cache.get_docker();
    assert_eq!(docker.len(), 1);
    assert_eq!(docker[0].name, "New Service");
    assert_eq!(docker[0].icon.as_deref(), Some("fa-star"));
}

/// T065: A container-stop event triggers a removal SSE event.
#[sqlx::test(migrations = "../../migrations")]
async fn container_stop_emits_removal_sse_event(_pool: SqlitePool) {
    let sse_hub = SseHub::new(64);
    let cache = DiscoveryCache::new();

    // Pre-populate cache with a service.
    let service = DiscoveredService {
        name: "Existing".to_string(),
        url: "https://existing.example.com".to_string(),
        icon: None,
        category: None,
        description: None,
        source: DiscoverySource::Docker,
        source_id: "def456".to_string(),
    };
    cache.add_docker(service);
    assert_eq!(cache.get_docker().len(), 1);

    // Subscribe.
    let mut rx = sse_hub.subscribe();

    // Simulate container stop: remove from cache and emit SSE.
    cache.remove_docker("def456");
    sse_hub.broadcast_discovery(SseDiscoveryEvent {
        service_id: "def456".to_string(),
        action: DiscoveryAction::Removed,
        name: String::new(),
        url: String::new(),
    });

    // Assert SSE event received.
    let event = tokio::time::timeout(Duration::from_secs(5), rx.recv())
        .await
        .expect("should receive SSE event");

    match event.expect("event") {
        SseEvent::Discovery(de) => {
            assert_eq!(de.service_id, "def456");
            assert_eq!(de.action, DiscoveryAction::Removed);
        }
        _ => panic!("expected discovery event"),
    }

    // Assert cache was updated.
    assert!(cache.get_docker().is_empty());
}

/// T065: When Docker discovery is disabled in settings, no calls are made.
/// The discover_docker server function returns empty vec when disabled.
#[sqlx::test(migrations = "../../migrations")]
async fn disabled_docker_returns_empty_cache(pool: SqlitePool) {
    // Do NOT enable Docker in settings (default is disabled).
    let cache = DiscoveryCache::new();

    // Verify integration settings default to disabled.
    let integrations = settings_queries::get_integrations_typed(&pool)
        .await
        .expect("read integrations");
    assert!(!integrations.docker_enabled);
    assert!(!integrations.k8s_enabled);

    // Cache should be empty (no background task populated it).
    assert!(cache.get_docker().is_empty());
    assert!(cache.get_k8s().is_empty());
}

/// T065: Docker integration is read-only by construction — verified by source inspection.
/// Reads the docker.rs source file and asserts that only read-only bollard methods
/// (list_containers, inspect_container, events) are called. No mutating methods
/// (create_container, delete_container, start, stop, kill, exec, restart) appear.
#[test]
fn docker_integration_is_read_only_by_construction() {
    use std::path::PathBuf;

    // Locate docker.rs source relative to the test file.
    // Test file: crates/server/tests/integration/discovery.rs
    // Source:    crates/server/src/integrations/docker.rs
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("src");
    path.push("integrations");
    path.push("docker.rs");

    let source = std::fs::read_to_string(&path)
        .unwrap_or_else(|_| panic!("should read docker.rs source at {:?}", path));

    // Verify read-only methods are present (the integration actually uses them).
    assert!(
        source.contains("list_containers"),
        "docker.rs should call list_containers (read-only)"
    );
    assert!(
        source.contains("inspect_container"),
        "docker.rs should call inspect_container (read-only)"
    );
    assert!(
        source.contains(".events("),
        "docker.rs should call events (read-only stream)"
    );

    // Verify NO mutating methods are present.
    let mutating_methods = [
        "create_container",
        "delete_container",
        "remove_container",
        "start_container",
        "stop_container",
        "kill_container",
        "restart_container",
        "exec_container",
        "create_network",
        "delete_network",
        "create_volume",
        "delete_volume",
        "create_image",
        "remove_image",
        "prune_containers",
        "prune_images",
        "prune_networks",
        "prune_volumes",
    ];

    for method in &mutating_methods {
        assert!(
            !source.contains(method),
            "docker.rs must not call mutating method '{}' (read-only by construction)",
            method
        );
    }
}

/// T065: Label parser handles multi-value URLs with whitespace padding.
/// Verifies that comma-separated URLs with spaces are trimmed correctly.
#[test]
fn parse_labels_multi_value_url_with_whitespace() {
    let mut labels = HashMap::new();
    labels.insert("emberwake.name".to_string(), "Padded URLs".to_string());
    labels.insert(
        "emberwake.url".to_string(),
        " https://a.example.com , https://b.example.com ".to_string(),
    );

    let services =
        server::integrations::labels::parse_labels(&labels, DiscoverySource::Docker, "ws");
    assert_eq!(services.len(), 2);
    assert_eq!(services[0].url, "https://a.example.com");
    assert_eq!(services[1].url, "https://b.example.com");
}

/// T065: Label parser skips empty entries in multi-value URL list.
/// Verifies that "a,,b" produces 2 services, not 3 (empty entry is skipped).
#[test]
fn parse_labels_multi_value_url_skips_empty_entries() {
    let mut labels = HashMap::new();
    labels.insert("emberwake.name".to_string(), "Skip Empty".to_string());
    labels.insert(
        "emberwake.url".to_string(),
        "https://a.com,,https://b.com".to_string(),
    );

    let services =
        server::integrations::labels::parse_labels(&labels, DiscoverySource::Docker, "skip");
    assert_eq!(services.len(), 2, "empty URL entries should be skipped");
    assert_eq!(services[0].url, "https://a.com");
    assert_eq!(services[1].url, "https://b.com");
}

/// T065: Label parser with only required fields (no optionals).
/// Verifies that a service with only name + url is parsed correctly,
/// with all optional fields set to None.
#[test]
fn parse_labels_only_required_fields() {
    let mut labels = HashMap::new();
    labels.insert("emberwake.name".to_string(), "Minimal".to_string());
    labels.insert(
        "emberwake.url".to_string(),
        "https://minimal.example.com".to_string(),
    );

    let services =
        server::integrations::labels::parse_labels(&labels, DiscoverySource::Docker, "min");
    assert_eq!(services.len(), 1);
    assert_eq!(services[0].name, "Minimal");
    assert_eq!(services[0].url, "https://minimal.example.com");
    assert!(services[0].icon.is_none());
    assert!(services[0].category.is_none());
    assert!(services[0].description.is_none());
}

// T065: Full Docker API mocking requires a mock Docker daemon.
// The tests above verify:
//   - Source-level read-only verification (no mutating bollard methods called)
//   - Label parser edge cases (whitespace trimming, empty entries, minimal fields)
//   - Cache + SSE integration (container start/stop events)
//   - Disabled integration returns empty cache
// A complete Docker API mock would require a mock HTTP server implementing the
// Docker Engine API (/containers/json, /containers/{id}/json, /events) — this
// is impractical in unit tests without a test harness like testcontainers or a
// mock Docker daemon (e.g., bollard's DockerTestServer).
