//! SQLite connection pool initialization with WAL mode and foreign keys enabled.

use sqlx::SqlitePool;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use std::str::FromStr;
use tracing::info;

/// Initialize a SQLite connection pool with WAL mode and foreign keys ON.
/// Runs migrations from the `migrations/` directory on startup (idempotent).
pub async fn init_pool(db_path: &str) -> Result<SqlitePool, sqlx::Error> {
    let options = SqliteConnectOptions::from_str(&format!("sqlite://{db_path}"))?
        .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
        .foreign_keys(true)
        .create_if_missing(true);

    let pool = SqlitePoolOptions::new()
        .max_connections(8)
        .connect_with(options)
        .await?;

    info!("SQLite pool initialized: {}", db_path);

    sqlx::migrate!("../../migrations").run(&pool).await?;

    info!("Migrations applied");
    Ok(pool)
}
