//! Shared label/annotation parser for Docker containers and K8s Ingress.
//! Parses `emberwake.*` labels/annotations into DiscoveredService entries.
//! Pure function — no I/O, easily testable.

use std::collections::HashMap;

use app::domain::{DiscoveredService, DiscoverySource};

/// Label/annotation key prefix for Emberwake discovery.
pub const LABEL_PREFIX: &str = "emberwake.";

/// Parse Docker container labels or K8s Ingress annotations into DiscoveredService entries.
///
/// Label keys:
/// - `emberwake.name` — required; service display name
/// - `emberwake.url` — required; service URL (comma-separated for multiple)
/// - `emberwake.icon` — optional; icon reference
/// - `emberwake.category` — optional; category name
/// - `emberwake.description` — optional; description text
///
/// If `emberwake.name` or `emberwake.url` is missing/empty, no services are discovered.
/// Multi-value URL syntax: comma-separated URLs create one DiscoveredService per URL.
pub fn parse_labels(
    labels: &HashMap<String, String>,
    source: DiscoverySource,
    source_id: &str,
) -> Vec<DiscoveredService> {
    let name = match labels.get("emberwake.name") {
        Some(n) if !n.is_empty() => n.clone(),
        _ => return Vec::new(),
    };

    let url_raw = match labels.get("emberwake.url") {
        Some(u) if !u.is_empty() => u.clone(),
        _ => return Vec::new(),
    };

    let icon = labels.get("emberwake.icon").cloned();
    let category = labels.get("emberwake.category").cloned();
    let description = labels.get("emberwake.description").cloned();

    url_raw
        .split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|u| DiscoveredService {
            name: name.clone(),
            url: u.to_string(),
            icon: icon.clone(),
            category: category.clone(),
            description: description.clone(),
            source,
            source_id: source_id.to_string(),
        })
        .collect()
}

/// Check if any emberwake labels are present.
pub fn has_emberwake_labels(labels: &HashMap<String, String>) -> bool {
    labels.keys().any(|k| k.starts_with(LABEL_PREFIX))
}
