//! Scheduled WAL checkpoint + optional automated SQLite .backup.
//! Configurable retention: max backup count (default 7), max total size (default 500 MB).

use sqlx::SqlitePool;
use std::path::Path;
use std::time::Duration;
use tokio::time::interval;
use tracing::{info, warn};

use crate::config::BackupConfig;

/// Spawn a background task that periodically checkpoints the WAL.
pub fn spawn_checkpoint_task(pool: SqlitePool, interval_s: u64) {
    tokio::spawn(async move {
        let mut ticker = interval(Duration::from_secs(interval_s));
        ticker.tick().await;
        loop {
            ticker.tick().await;
            match sqlx::query("PRAGMA wal_checkpoint(TRUNCATE)")
                .execute(&pool)
                .await
            {
                Ok(_) => info!("WAL checkpoint completed"),
                Err(e) => warn!("WAL checkpoint failed: {e}"),
            }
        }
    });
}

/// Spawn a background task that periodically creates SQLite backups.
pub fn spawn_backup_task(pool: SqlitePool, db_path: String, config: BackupConfig) {
    if !config.backup_enabled {
        info!("SQLite backup disabled");
        return;
    }

    let backup_dir = config.backup_dir.clone();
    let interval_s = config.backup_interval_s;
    let max_count = config.max_backup_count;
    let max_size_mb = config.max_backup_size_mb;

    tokio::spawn(async move {
        if let Err(e) = std::fs::create_dir_all(&backup_dir) {
            warn!("Failed to create backup dir {backup_dir}: {e}");
            return;
        }

        let mut ticker = interval(Duration::from_secs(interval_s));
        ticker.tick().await;
        loop {
            ticker.tick().await;
            if let Err(e) = create_backup(&pool, &db_path, &backup_dir).await {
                warn!("Backup failed: {e}");
            } else {
                info!("Backup completed");
                if let Err(e) = prune_backups(&backup_dir, max_count, max_size_mb) {
                    warn!("Backup pruning failed: {e}");
                }
            }
        }
    });
}

/// Create a SQLite backup by copying the database file.
async fn create_backup(
    _pool: &SqlitePool,
    db_path: &str,
    backup_dir: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let timestamp = chrono::Utc::now().format("%Y%m%dT%H%M%S");
    let backup_path = Path::new(backup_dir).join(format!("emberwake-{timestamp}.db"));

    let db_path = db_path.to_string();
    let backup_path = backup_path.to_string_lossy().to_string();
    tokio::task::spawn_blocking(move || {
        std::fs::copy(&db_path, &backup_path)?;
        Ok::<(), std::io::Error>(())
    })
    .await??;

    Ok(())
}

/// Prune old backups by count and total size.
fn prune_backups(
    backup_dir: &str,
    max_count: u32,
    max_size_mb: u64,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let dir = Path::new(backup_dir);
    let mut backups: Vec<_> = std::fs::read_dir(dir)?
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.file_name()
                .to_str()
                .map(|s| s.starts_with("emberwake-") && s.ends_with(".db"))
                .unwrap_or(false)
        })
        .collect();

    backups.sort_by(|a, b| {
        b.metadata()
            .and_then(|m| m.modified())
            .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
            .cmp(
                &a.metadata()
                    .and_then(|m| m.modified())
                    .unwrap_or(std::time::SystemTime::UNIX_EPOCH),
            )
    });

    if backups.len() > max_count as usize {
        for entry in backups.iter().skip(max_count as usize) {
            if let Err(e) = std::fs::remove_file(entry.path()) {
                warn!("Failed to remove old backup {:?}: {e}", entry.path());
            }
        }
        backups.truncate(max_count as usize);
    }

    let max_size_bytes = max_size_mb * 1024 * 1024;
    let mut total_size: u64 = 0;
    for entry in &backups {
        if let Ok(meta) = entry.metadata() {
            total_size += meta.len();
        }
    }
    if total_size > max_size_bytes {
        for entry in backups.iter().rev() {
            if total_size <= max_size_bytes {
                break;
            }
            if let Ok(meta) = entry.metadata() {
                let size = meta.len();
                if let Err(e) = std::fs::remove_file(entry.path()) {
                    warn!("Failed to remove oversized backup {:?}: {e}", entry.path());
                } else {
                    total_size -= size;
                }
            }
        }
    }

    Ok(())
}
