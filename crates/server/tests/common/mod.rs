//! Common test helpers for integration and server_fn tests.

use sqlx::SqlitePool;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use std::str::FromStr;

/// Create an in-memory SQLite pool with migrations applied.
pub async fn test_pool() -> SqlitePool {
    let options = SqliteConnectOptions::from_str("sqlite::memory:")
        .expect("valid connect options")
        .foreign_keys(true)
        .create_if_missing(true);

    let pool = SqlitePoolOptions::new()
        .max_connections(8)
        .connect_with(options)
        .await
        .expect("connect pool");

    sqlx::migrate!("../../migrations")
        .run(&pool)
        .await
        .expect("migrations");

    pool
}

/// Build a test AppState suitable for Axum router tests.
#[allow(dead_code)]
pub fn build_test_state(pool: SqlitePool, server_key: &str) -> server::state::AppState {
    use std::sync::Arc;

    use leptos::prelude::LeptosOptions;
    use server::audit::AuditWriter;
    use server::config::Config;
    use server::sse::SseHub;

    let config = Config {
        db_path: "test".to_string(),
        bind_addr: "0.0.0.0:0".to_string(),
        server_key: server_key.to_string(),
        ..Default::default()
    };

    server::state::AppState {
        leptos_options: LeptosOptions::builder()
            .output_name("emberwake")
            .site_addr("0.0.0.0:0".parse::<std::net::SocketAddr>().unwrap())
            .site_root("target/site")
            .build(),
        db: pool.clone(),
        config: Arc::new(config),
        audit: Arc::new(AuditWriter::new(pool)),
        sse_hub: SseHub::new(256),
    }
}

/// Build a test AppState with a custom Config (for OIDC-enabled tests, etc.).
#[allow(dead_code)]
pub fn build_test_state_with_config(
    pool: SqlitePool,
    config: server::config::Config,
) -> server::state::AppState {
    use std::sync::Arc;

    use leptos::prelude::LeptosOptions;
    use server::audit::AuditWriter;
    use server::sse::SseHub;

    server::state::AppState {
        leptos_options: LeptosOptions::builder()
            .output_name("emberwake")
            .site_addr("0.0.0.0:0".parse::<std::net::SocketAddr>().unwrap())
            .site_root("target/site")
            .build(),
        db: pool.clone(),
        config: Arc::new(config),
        audit: Arc::new(AuditWriter::new(pool)),
        sse_hub: SseHub::new(256),
    }
}
