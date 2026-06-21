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
