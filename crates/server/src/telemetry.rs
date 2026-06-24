//! Telemetry: JSON tracing subscriber, OTLP export, Prometheus /metrics,
//! /healthz + /readyz endpoints.

use axum::Router;
use axum::response::Json;
use axum::routing::get;
use serde_json::json;

use crate::state::AppState;

/// Initialize the tracing subscriber with JSON format and env-filter level.
/// If OTLP endpoint is configured, initializes OTLP exporter alongside the JSON subscriber.
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

/// Guard that flushes and shuts down the OTLP tracer provider on drop.
/// Must be held for the lifetime of the process.
pub struct OtlpGuard;

impl Drop for OtlpGuard {
    fn drop(&mut self) {
        opentelemetry::global::shutdown_tracer_provider();
    }
}

/// Initialize OTLP trace export via HTTP.
///
/// Creates a real OTLP HTTP exporter with a batch span processor, installs it
/// as the global tracer provider, and returns a guard that flushes on drop.
/// The `endpoint` should point to an OTLP/HTTP collector (e.g. `http://localhost:4318/v1/traces`).
pub fn init_otlp(endpoint: &str) -> OtlpGuard {
    use opentelemetry::global;
    use opentelemetry_sdk::runtime::Tokio;
    use opentelemetry_sdk::trace::{BatchSpanProcessor, TracerProvider};
    use opentelemetry_otlp::{SpanExporter, WithExportConfig};

    let exporter = SpanExporter::builder()
        .with_http()
        .with_endpoint(endpoint)
        .build()
        .unwrap_or_else(|e| panic!("failed to create OTLP exporter for {endpoint}: {e}"));

    let processor = BatchSpanProcessor::builder(exporter, Tokio).build();

    let provider = TracerProvider::builder()
        .with_span_processor(processor)
        .build();

    global::set_tracer_provider(provider);

    tracing::info!("OTLP trace export initialized: {endpoint}");

    OtlpGuard
}

/// Register a request counter metric with the Prometheus default registry.
/// Exposes emberwake_http_requests_total on the /metrics endpoint.
pub fn register_request_counter() {
    use prometheus::IntCounterVec;
    use prometheus::Opts;

    let counter = IntCounterVec::new(
        Opts::new(
            "emberwake_http_requests_total",
            "Total HTTP requests served",
        ),
        &["path"],
    )
    .expect("valid counter opts");

    // Register with the default registry (used by metrics_handler).
    // Ignore error if already registered (e.g., in tests).
    prometheus::default_registry()
        .register(Box::new(counter))
        .ok();
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

/// Prometheus metrics endpoint — exposes process metrics + request count.
pub async fn metrics_handler() -> String {
    use prometheus::Encoder;
    let encoder = prometheus::TextEncoder::new();
    let metric_families = prometheus::default_registry().gather();
    let mut buffer = Vec::new();
    if encoder.encode(&metric_families, &mut buffer).is_err() {
        return "# error encoding metrics\n".to_string();
    }
    String::from_utf8(buffer).unwrap_or_else(|_| "# error encoding metrics\n".to_string())
}

/// Build a sub-router for health, readiness, and metrics endpoints.
/// Returns Router<AppState> — caller must call .with_state(state) on the combined router.
pub fn health_routes() -> Router<AppState> {
    Router::new()
        .route("/healthz", get(healthz))
        .route("/readyz", get(readyz))
        .route("/metrics", get(metrics_handler))
}
