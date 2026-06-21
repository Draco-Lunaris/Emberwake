//! Health-check engine and scheduler for service monitoring.
//! HTTP checks use reqwest (rustls); TCP checks use tokio::net.
//! All checks run in background tasks — never block the request path.

pub mod scheduler;

use std::time::{Duration, Instant};

use app::domain::MonitorState;

/// Result of a single health check.
pub struct CheckResult {
    pub state: MonitorState,
    pub latency_ms: Option<i64>,
    pub reason: Option<String>,
}

/// Default per-check timeout (seconds).
const DEFAULT_TIMEOUT_S: u64 = 10;

/// Run an HTTP health check against the given target URL.
/// GET the URL; 2xx = up, 3xx/4xx = degraded, timeout/error = down.
pub async fn http_check(target: &str, timeout_s: u64) -> CheckResult {
    let timeout = Duration::from_secs(timeout_s.max(1));
    let client = match reqwest::Client::builder()
        .timeout(timeout)
        .connect_timeout(timeout)
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            return CheckResult {
                state: MonitorState::Down,
                latency_ms: None,
                reason: Some(format!("client build error: {e}")),
            };
        }
    };

    let start = Instant::now();
    match client.get(target).send().await {
        Ok(resp) => {
            let elapsed = start.elapsed().as_millis() as i64;
            let status = resp.status().as_u16();
            let state = if (200..300).contains(&status) {
                MonitorState::Up
            } else if (300..500).contains(&status) {
                MonitorState::Degraded
            } else {
                MonitorState::Down
            };
            let reason = if state == MonitorState::Up {
                None
            } else {
                Some(format!("HTTP {status}"))
            };
            CheckResult {
                state,
                latency_ms: Some(elapsed),
                reason,
            }
        }
        Err(e) => {
            let reason = if e.is_timeout() {
                "timeout".to_string()
            } else if e.is_connect() {
                "connection refused".to_string()
            } else {
                format!("{e}")
            };
            CheckResult {
                state: MonitorState::Down,
                latency_ms: None,
                reason: Some(reason),
            }
        }
    }
}

/// Run a TCP health check against the given host:port.
/// Connect succeeds = up; timeout/refused = down.
pub async fn tcp_check(target: &str, timeout_s: u64) -> CheckResult {
    let timeout = Duration::from_secs(timeout_s.max(1));
    let start = Instant::now();

    let connect = tokio::time::timeout(timeout, tokio::net::TcpStream::connect(target)).await;

    match connect {
        Ok(Ok(_stream)) => {
            let elapsed = start.elapsed().as_millis() as i64;
            CheckResult {
                state: MonitorState::Up,
                latency_ms: Some(elapsed),
                reason: None,
            }
        }
        Ok(Err(e)) => CheckResult {
            state: MonitorState::Down,
            latency_ms: None,
            reason: Some(format!("connect error: {e}")),
        },
        Err(_) => CheckResult {
            state: MonitorState::Down,
            latency_ms: None,
            reason: Some("timeout".to_string()),
        },
    }
}

/// Run a health check based on the monitor kind (http or tcp).
pub async fn run_check(kind: &str, target: &str, timeout_s: u64) -> CheckResult {
    match kind {
        "http" => http_check(target, timeout_s).await,
        "tcp" => tcp_check(target, timeout_s).await,
        _ => CheckResult {
            state: MonitorState::Down,
            latency_ms: None,
            reason: Some(format!("unknown monitor kind: {kind}")),
        },
    }
}

/// Run a single health check for a monitored service, update DB, emit SSE on state change.
pub async fn check_service(
    pool: &sqlx::SqlitePool,
    sse_hub: &crate::sse::SseHub,
    service: &app::server::monitor_queries::MonitoredService,
) {
    let timeout_s = service.monitor_interval_s.max(1) as u64;
    let timeout_s = timeout_s.min(DEFAULT_TIMEOUT_S * 3); // cap at 30s

    let result = run_check(&service.monitor_kind, &service.monitor_target, timeout_s).await;
    let now = chrono::Utc::now().to_rfc3339();

    // Get previous state to detect transition.
    let prev = app::server::monitor_queries::get_status_reading(pool, service.id)
        .await
        .ok()
        .flatten();

    // Upsert current reading.
    let _ = app::server::monitor_queries::upsert_status_reading(
        pool,
        service.id,
        result.state,
        result.latency_ms,
        result.reason.as_deref(),
        &now,
    )
    .await;

    // Insert history row.
    let history_id = uuid::Uuid::now_v7();
    let _ = app::server::monitor_queries::insert_status_history(
        pool,
        history_id,
        service.id,
        result.state,
        result.latency_ms,
        result.reason.as_deref(),
        &now,
    )
    .await;

    // Prune history.
    let _ = app::server::monitor_queries::prune_status_history(
        pool,
        service.id,
        app::server::monitor_queries::DEFAULT_MAX_ROWS,
        app::server::monitor_queries::DEFAULT_MAX_AGE_DAYS,
    )
    .await;

    // Emit SSE event on state change.
    let state_changed = match &prev {
        Some(p) => p.state != result.state,
        None => true, // first check = always emit
    };

    if state_changed {
        sse_hub.broadcast_status(
            service.id,
            result.state,
            result.latency_ms,
            service.visibility,
        );
    }
}
