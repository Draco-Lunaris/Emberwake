//! T057: SSE test — a connected client receives a status event on up→down flip.
//! Tests the SSE hub broadcast mechanism for status event delivery.

use std::str::FromStr;

use app::domain::{MonitorState, Visibility};
use app::server::monitor_queries;
use server::monitor;
use server::sse::{SseEvent, SseHub};
use sqlx::SqlitePool;
use uuid::Uuid;

/// Seed a monitored service into the database.
async fn seed_service(pool: &SqlitePool, id: &str, target: &str) {
    let now = "2026-01-01T00:00:00Z";
    sqlx::query(
        "INSERT INTO service (id, category_id, name, url, is_pinned, order_index, visibility, \
         monitor_enabled, monitor_kind, monitor_target, monitor_interval_s, created_at, updated_at) \
         VALUES (?, NULL, 'SSE Test', ?, 0, 0, 'public', 1, 'tcp', ?, 5, ?, ?)",
    )
    .bind(id)
    .bind(target)
    .bind(target)
    .bind(now)
    .bind(now)
    .execute(pool)
    .await
    .expect("insert service");
}

/// T057: A connected client receives a status event on up→down flip.
#[sqlx::test(migrations = "../../migrations")]
async fn sse_client_receives_status_event_on_flip(pool: SqlitePool) {
    let service_id = Uuid::from_str("0192a0a0-0000-7000-8000-000000000050").unwrap();

    // Seed service targeting a port that won't accept connections (down).
    seed_service(&pool, &service_id.to_string(), "127.0.0.1:1").await;

    // Pre-seed an "up" reading so the check will detect a transition (up→down).
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

    let sse_hub = SseHub::new(64);

    // Subscribe BEFORE running the check so we receive the event.
    let mut rx = sse_hub.subscribe();

    // Get the monitored service and run the check.
    let services = monitor_queries::list_monitored_services(&pool)
        .await
        .expect("list monitored");
    assert_eq!(services.len(), 1);

    monitor::check_service(&pool, &sse_hub, &services[0]).await;

    // Assert the client receives a status event with the new state (down).
    let event = tokio::time::timeout(std::time::Duration::from_secs(5), rx.recv())
        .await
        .expect("should receive SSE event within timeout");

    match event.expect("event") {
        SseEvent::Status(se) => {
            assert_eq!(se.service_id, service_id);
            assert_eq!(
                se.state,
                MonitorState::Down,
                "should be down after failed TCP check"
            );
            assert_eq!(se.visibility, Visibility::Public);
        }
        SseEvent::Weather(_) => panic!("expected status event, got weather"),
        SseEvent::Discovery(_) => panic!("expected status event, got discovery"),
    }
}

/// T057: No SSE event emitted when state does not change.
#[sqlx::test(migrations = "../../migrations")]
async fn no_sse_event_on_same_state(pool: SqlitePool) {
    let service_id = Uuid::from_str("0192a0a0-0000-7000-8000-000000000051").unwrap();

    // Start a stub TCP listener that accepts connections (service is up).
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    let addr = listener.local_addr().expect("local addr");
    let target = format!("127.0.0.1:{}", addr.port());

    tokio::spawn(async move {
        let _ = listener.accept().await;
    });

    seed_service(&pool, &service_id.to_string(), &target).await;

    // Pre-seed an "up" reading so the check should NOT emit (same state).
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

    let sse_hub = SseHub::new(64);
    let mut rx = sse_hub.subscribe();

    let services = monitor_queries::list_monitored_services(&pool)
        .await
        .expect("list monitored");

    monitor::check_service(&pool, &sse_hub, &services[0]).await;

    // Assert NO event is received (state stayed up, no transition).
    let result = tokio::time::timeout(std::time::Duration::from_millis(500), rx.recv()).await;
    assert!(
        result.is_err(),
        "should NOT receive SSE event when state does not change"
    );
}
