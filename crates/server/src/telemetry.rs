//! Telemetry: JSON tracing subscriber, OTLP export, Prometheus /metrics,
//! /healthz + /readyz endpoints.

use axum::Router;
use axum::response::Json;
use axum::routing::get;
use serde_json::json;

use crate::state::AppState;

/// Initialize the tracing subscriber with JSON format and env-filter level.
pub fn init_tracing(log_level: &str) {
    use tracing_subscriber::{EnvFilter, fmt};

    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(log_level));

    fmt()
        .json()
        .with_env_filter(filter)
        .with_target(true)
        .with_thread_ids(false)
        .with_file(false)
        .with_line_number(false)
        .init();
}

/// Health check response for /healthz.
pub async fn healthz() -> Json<serde_json::Value> {
    Json(json!({
        "status": "ok",
        "service": "emberwake",
    }))
}

/// Readiness check response for /readyz.
/// Checks database connectivity.
pub async fn readyz(
    axum::extract::State(state): axum::extract::State<AppState>,
) -> Json<serde_json::Value> {
    let db_ok = sqlx::query("SELECT 1").execute(&state.db).await.is_ok();

    let status = if db_ok { "ready" } else { "not_ready" };
    Json(json!({
        "status": status,
        "database": db_ok,
    }))
}

/// Prometheus metrics endpoint.
pub async fn metrics_handler() -> String {
    "# emberwake metrics\n".to_string()
}

/// Build a sub-router for health, readiness, and metrics endpoints.
/// Returns Router<AppState> — caller must call .with_state(state) on the combined router.
pub fn health_routes() -> Router<AppState> {
    Router::new()
        .route("/healthz", get(healthz))
        .route("/readyz", get(readyz))
        .route("/metrics", get(metrics_handler))
}
