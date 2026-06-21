//! T056/T056b: Monitor scheduler + uptime summary integration tests.
//! Tests: check records state/latency, history row written, state transition emits SSE event,
//! uptime summary computes correct percentage, retention pruning removes old rows.

use std::str::FromStr;

use app::domain::{MonitorState, VisibilityFilter};
use app::server::monitor_queries;
use server::monitor;
use server::sse::SseHub;
use sqlx::SqlitePool;
use uuid::Uuid;

/// Seed a monitored service into the database.
async fn seed_monitored_service(
    pool: &SqlitePool,
    id: &str,
    name: &str,
    monitor_kind: &str,
    monitor_target: &str,
    visibility: &str,
) {
    let now = "2026-01-01T00:00:00Z";
    sqlx::query(
        "INSERT INTO service (id, category_id, name, url, is_pinned, order_index, visibility, \
         monitor_enabled, monitor_kind, monitor_target, monitor_interval_s, created_at, updated_at) \
         VALUES (?, NULL, ?, ?, 0, 0, ?, 1, ?, ?, 5, ?, ?)",
    )
    .bind(id)
    .bind(name)
    .bind(monitor_target)
    .bind(visibility)
    .bind(monitor_kind)
    .bind(monitor_target)
    .bind(now)
    .bind(now)
    .execute(pool)
    .await
    .expect("insert monitored service");
}

/// Seed a disabled service (monitor_enabled=false) into the database.
async fn seed_disabled_service(pool: &SqlitePool, id: &str, name: &str) {
    let now = "2026-01-01T00:00:00Z";
    sqlx::query(
        "INSERT INTO service (id, category_id, name, url, is_pinned, order_index, visibility, \
         monitor_enabled, created_at, updated_at) \
         VALUES (?, NULL, ?, 'http://disabled.example.com', 0, 0, 'public', 0, ?, ?)",
    )
    .bind(id)
    .bind(name)
    .bind(now)
    .bind(now)
    .execute(pool)
    .await
    .expect("insert disabled service");
}

/// T056: Check records state/latency; transition emits an event; history row written.
/// Uses a stub TCP listener as the health check target.
#[sqlx::test(migrations = "../../migrations")]
async fn check_records_state_and_emits_event(pool: SqlitePool) {
    // Start a stub TCP listener that accepts connections (service is up).
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    let addr = listener.local_addr().expect("local addr");
    let target = format!("127.0.0.1:{}", addr.port());

    // Spawn a task to accept the connection so the check succeeds.
    tokio::spawn(async move {
        let _ = listener.accept().await;
    });

    let service_id = Uuid::from_str("0192a0a0-0000-7000-8000-000000000001").unwrap();
    seed_monitored_service(
        &pool,
        &service_id.to_string(),
        "Test TCP Service",
        "tcp",
        &target,
        "public",
    )
    .await;

    let sse_hub = SseHub::new(64);
    let mut rx = sse_hub.subscribe();

    // Get the monitored service from DB.
    let services = monitor_queries::list_monitored_services(&pool)
        .await
        .expect("list monitored");
    assert_eq!(services.len(), 1);
    let svc = &services[0];
    assert_eq!(svc.id, service_id);

    // Run the health check.
    monitor::check_service(&pool, &sse_hub, svc).await;

    // Assert StatusReading row updated with state/latency/checked_at.
    let reading = monitor_queries::get_status_reading(&pool, service_id)
        .await
        .expect("get reading");
    let reading = reading.expect("reading should exist");
    assert_eq!(reading.state, MonitorState::Up);
    assert!(reading.latency_ms.is_some(), "latency should be recorded");
    assert!(!reading.checked_at.is_empty(), "checked_at should be set");

    // Assert StatusHistory row written.
    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM status_history WHERE service_id = ?")
        .bind(service_id.to_string())
        .fetch_one(&pool)
        .await
        .expect("count history");
    assert_eq!(count.0, 1, "one history row should be written");

    // Assert SSE event emitted (first check = always emit).
    let event = tokio::time::timeout(std::time::Duration::from_secs(2), rx.recv())
        .await
        .expect("should receive SSE event within timeout");
    let event = event.expect("event should not be an error");
    match event {
        server::sse::SseEvent::Status(se) => {
            assert_eq!(se.service_id, service_id);
            assert_eq!(se.state, MonitorState::Up);
        }
        _ => panic!("expected status event"),
    }
}

/// T056: State transition (up→down) emits an event.
#[sqlx::test(migrations = "../../migrations")]
async fn state_transition_emits_event(pool: SqlitePool) {
    let service_id = Uuid::from_str("0192a0a0-0000-7000-8000-000000000002").unwrap();

    // Seed service with a target that will fail (port not listening = down).
    seed_monitored_service(
        &pool,
        &service_id.to_string(),
        "Test Down Service",
        "tcp",
        "127.0.0.1:1", // port 1 is reserved, connection will fail
        "public",
    )
    .await;

    let sse_hub = SseHub::new(64);

    // First, seed an existing "up" reading.
    monitor_queries::upsert_status_reading(
        &pool,
        service_id,
        MonitorState::Up,
        Some(10),
        None,
        "2026-01-01T00:00:00Z",
    )
    .await
    .expect("seed up reading");

    let mut rx = sse_hub.subscribe();

    let services = monitor_queries::list_monitored_services(&pool)
        .await
        .expect("list monitored");
    let svc = &services[0];

    // Run check — should detect down (transition up→down).
    monitor::check_service(&pool, &sse_hub, svc).await;

    // Assert SSE event emitted for transition.
    let event = tokio::time::timeout(std::time::Duration::from_secs(5), rx.recv())
        .await
        .expect("should receive SSE event on transition");
    let event = event.expect("event");
    match event {
        server::sse::SseEvent::Status(se) => {
            assert_eq!(se.service_id, service_id);
            assert_eq!(
                se.state,
                MonitorState::Down,
                "should be down after failed check"
            );
        }
        _ => panic!("expected status event"),
    }

    // Verify reading updated to down.
    let reading = monitor_queries::get_status_reading(&pool, service_id)
        .await
        .expect("get reading")
        .expect("reading exists");
    assert_eq!(reading.state, MonitorState::Down);
}

/// T056: Disabled services make no outbound calls — list_monitored_services excludes them.
#[sqlx::test(migrations = "../../migrations")]
async fn disabled_services_not_listed(pool: SqlitePool) {
    seed_disabled_service(&pool, "disabled-svc-001", "Disabled Service").await;
    seed_monitored_service(
        &pool,
        "enabled-svc-001",
        "Enabled Service",
        "tcp",
        "127.0.0.1:1",
        "public",
    )
    .await;

    let services = monitor_queries::list_monitored_services(&pool)
        .await
        .expect("list monitored");
    assert_eq!(services.len(), 1, "only enabled service should be listed");
    assert_eq!(services[0].name, "Enabled Service");
}

/// T056b: get_uptime_summary computes correct percentage from StatusHistory.
#[sqlx::test(migrations = "../../migrations")]
async fn uptime_summary_computes_percentage(pool: SqlitePool) {
    let service_id = Uuid::from_str("0192a0a0-0000-7000-8000-000000000010").unwrap();
    seed_monitored_service(
        &pool,
        &service_id.to_string(),
        "Uptime Test",
        "tcp",
        "127.0.0.1:1",
        "public",
    )
    .await;

    let now = chrono::Utc::now();

    // Insert 10 history rows: 8 up, 2 down within the last hour.
    for i in 0..10 {
        let state = if i < 8 {
            MonitorState::Up
        } else {
            MonitorState::Down
        };
        let ts = now - chrono::Duration::minutes(i);
        let id = Uuid::now_v7();
        monitor_queries::insert_status_history(
            &pool,
            id,
            service_id,
            state,
            Some(i * 10),
            None,
            &ts.to_rfc3339(),
        )
        .await
        .expect("insert history");
    }

    let summary = monitor_queries::compute_uptime_summary(&pool, service_id, 24)
        .await
        .expect("uptime summary");

    assert_eq!(summary.service_id, service_id);
    assert_eq!(summary.total_checks, 10);
    assert_eq!(summary.up_checks, 8);
    assert_eq!(summary.uptime_percent, 80.0);
}

/// T056b: Retention pruning removes old rows exceeding max-age and max-rows.
#[sqlx::test(migrations = "../../migrations")]
async fn retention_pruning_removes_old_rows(pool: SqlitePool) {
    let service_id = Uuid::from_str("0192a0a0-0000-7000-8000-000000000020").unwrap();
    seed_monitored_service(
        &pool,
        &service_id.to_string(),
        "Prune Test",
        "tcp",
        "127.0.0.1:1",
        "public",
    )
    .await;

    let now = chrono::Utc::now();

    // Insert 5 rows older than 30 days (should be pruned by max-age).
    for i in 0..5 {
        let ts = now - chrono::Duration::days(31 + i);
        let id = Uuid::now_v7();
        monitor_queries::insert_status_history(
            &pool,
            id,
            service_id,
            MonitorState::Up,
            Some(10),
            None,
            &ts.to_rfc3339(),
        )
        .await
        .expect("insert old history");
    }

    // Insert 5 recent rows (should be kept).
    for i in 0..5 {
        let ts = now - chrono::Duration::minutes(i);
        let id = Uuid::now_v7();
        monitor_queries::insert_status_history(
            &pool,
            id,
            service_id,
            MonitorState::Up,
            Some(10),
            None,
            &ts.to_rfc3339(),
        )
        .await
        .expect("insert recent history");
    }

    // Prune with max_age_days=30, max_rows=1000.
    let deleted = monitor_queries::prune_status_history(&pool, service_id, 1000, 30)
        .await
        .expect("prune");
    assert_eq!(deleted, 5, "5 old rows should be pruned");

    // Verify 5 recent rows remain.
    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM status_history WHERE service_id = ?")
        .bind(service_id.to_string())
        .fetch_one(&pool)
        .await
        .expect("count");
    assert_eq!(count.0, 5, "5 recent rows should remain");
}

/// T056b: Pruning by max-rows keeps the most recent rows.
#[sqlx::test(migrations = "../../migrations")]
async fn pruning_by_max_rows_keeps_recent(pool: SqlitePool) {
    let service_id = Uuid::from_str("0192a0a0-0000-7000-8000-000000000030").unwrap();
    seed_monitored_service(
        &pool,
        &service_id.to_string(),
        "Max Rows Test",
        "tcp",
        "127.0.0.1:1",
        "public",
    )
    .await;

    let now = chrono::Utc::now();

    // Insert 15 recent rows.
    for i in 0..15 {
        let ts = now - chrono::Duration::seconds(i);
        let id = Uuid::now_v7();
        monitor_queries::insert_status_history(
            &pool,
            id,
            service_id,
            MonitorState::Up,
            Some(10),
            None,
            &ts.to_rfc3339(),
        )
        .await
        .expect("insert history");
    }

    // Prune with max_rows=10, max_age_days=365 (age won't prune anything).
    let deleted = monitor_queries::prune_status_history(&pool, service_id, 10, 365)
        .await
        .expect("prune");
    assert_eq!(deleted, 5, "5 excess rows should be pruned");

    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM status_history WHERE service_id = ?")
        .bind(service_id.to_string())
        .fetch_one(&pool)
        .await
        .expect("count");
    assert_eq!(count.0, 10, "10 rows should remain after pruning");
}

/// Visibility filter: list_status_readings returns only public for PublicOnly.
#[sqlx::test(migrations = "../../migrations")]
async fn status_readings_visibility_filter(pool: SqlitePool) {
    let pub_id = Uuid::from_str("0192a0a0-0000-7000-8000-000000000040").unwrap();
    let priv_id = Uuid::from_str("0192a0a0-0000-7000-8000-000000000041").unwrap();

    seed_monitored_service(
        &pool,
        &pub_id.to_string(),
        "Public Svc",
        "tcp",
        "127.0.0.1:1",
        "public",
    )
    .await;
    seed_monitored_service(
        &pool,
        &priv_id.to_string(),
        "Private Svc",
        "tcp",
        "127.0.0.1:1",
        "private",
    )
    .await;

    // Insert status readings for both.
    monitor_queries::upsert_status_reading(
        &pool,
        pub_id,
        MonitorState::Up,
        Some(10),
        None,
        "2026-01-01T00:00:00Z",
    )
    .await
    .expect("upsert pub");
    monitor_queries::upsert_status_reading(
        &pool,
        priv_id,
        MonitorState::Down,
        None,
        Some("timeout"),
        "2026-01-01T00:00:00Z",
    )
    .await
    .expect("upsert priv");

    // PublicOnly: should return only the public service reading.
    let public_readings =
        monitor_queries::list_status_readings(&pool, VisibilityFilter::PublicOnly)
            .await
            .expect("list public");
    assert_eq!(public_readings.len(), 1);
    assert_eq!(public_readings[0].service_id, pub_id);

    // All: should return both readings.
    let all_readings = monitor_queries::list_status_readings(&pool, VisibilityFilter::All)
        .await
        .expect("list all");
    assert_eq!(all_readings.len(), 2);
}
