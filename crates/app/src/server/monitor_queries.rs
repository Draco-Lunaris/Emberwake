//! SQL query functions for service monitoring (status_reading, status_history, uptime).
//! All functions are ssr-only. Uses parameterized SQL with static string literals.

#![cfg(feature = "ssr")]

use std::str::FromStr;

use sqlx::{Row, SqlitePool};
use uuid::Uuid;

use crate::domain::{MonitorState, StatusReading, UptimeSummary, Visibility};
use crate::error::AppError;

/// Default max rows per service in status_history.
pub const DEFAULT_MAX_ROWS: i64 = 1000;
/// Default max age in days for status_history rows.
pub const DEFAULT_MAX_AGE_DAYS: i64 = 30;

/// A monitored service row from the service table.
pub struct MonitoredService {
    pub id: Uuid,
    pub name: String,
    pub url: String,
    pub visibility: Visibility,
    pub monitor_kind: String,
    pub monitor_target: String,
    pub monitor_interval_s: i64,
}

/// List all services with monitor_enabled=true.
pub async fn list_monitored_services(
    pool: &SqlitePool,
) -> Result<Vec<MonitoredService>, sqlx::Error> {
    let rows = sqlx::query(
        "SELECT id, name, url, visibility, monitor_kind, monitor_target, monitor_interval_s \
         FROM service WHERE monitor_enabled = 1 AND monitor_kind IS NOT NULL AND monitor_target IS NOT NULL",
    )
    .fetch_all(pool)
    .await?;

    let mut services = Vec::new();
    for row in rows {
        let id_str: String = row.get("id");
        let vis_str: String = row.get("visibility");
        let visibility = Visibility::from_str(&vis_str).unwrap_or(Visibility::Public);
        let kind: String = row.get("monitor_kind");
        let target: String = row.get("monitor_target");
        let interval: i64 = row.get("monitor_interval_s");
        services.push(MonitoredService {
            id: Uuid::parse_str(&id_str).unwrap_or_default(),
            name: row.get("name"),
            url: row.get("url"),
            visibility,
            monitor_kind: kind,
            monitor_target: target,
            monitor_interval_s: interval,
        });
    }
    Ok(services)
}

/// Get the current StatusReading for a service, if it exists.
pub async fn get_status_reading(
    pool: &SqlitePool,
    service_id: Uuid,
) -> Result<Option<StatusReading>, sqlx::Error> {
    let row = sqlx::query(
        "SELECT service_id, state, latency_ms, reason, checked_at \
         FROM status_reading WHERE service_id = ?",
    )
    .bind(service_id.to_string())
    .fetch_optional(pool)
    .await?;

    match row {
        Some(r) => {
            let state_str: String = r.get("state");
            let state = MonitorState::from_str(&state_str).unwrap_or(MonitorState::Down);
            Ok(Some(StatusReading {
                service_id,
                state,
                latency_ms: r.get("latency_ms"),
                reason: r.get("reason"),
                checked_at: r.get("checked_at"),
            }))
        }
        None => Ok(None),
    }
}

/// Upsert the current StatusReading for a service (one row per service).
pub async fn upsert_status_reading(
    pool: &SqlitePool,
    service_id: Uuid,
    state: MonitorState,
    latency_ms: Option<i64>,
    reason: Option<&str>,
    checked_at: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO status_reading (service_id, state, latency_ms, reason, checked_at) \
         VALUES (?, ?, ?, ?, ?) \
         ON CONFLICT(service_id) DO UPDATE SET state = excluded.state, \
         latency_ms = excluded.latency_ms, reason = excluded.reason, checked_at = excluded.checked_at",
    )
    .bind(service_id.to_string())
    .bind(state.to_string())
    .bind(latency_ms)
    .bind(reason)
    .bind(checked_at)
    .execute(pool)
    .await?;
    Ok(())
}

/// Insert a StatusHistory row.
pub async fn insert_status_history(
    pool: &SqlitePool,
    id: Uuid,
    service_id: Uuid,
    state: MonitorState,
    latency_ms: Option<i64>,
    reason: Option<&str>,
    checked_at: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO status_history (id, service_id, state, latency_ms, reason, checked_at) \
         VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(id.to_string())
    .bind(service_id.to_string())
    .bind(state.to_string())
    .bind(latency_ms)
    .bind(reason)
    .bind(checked_at)
    .execute(pool)
    .await?;
    Ok(())
}

/// Prune status_history rows exceeding max-rows and max-age for a service.
pub async fn prune_status_history(
    pool: &SqlitePool,
    service_id: Uuid,
    max_rows: i64,
    max_age_days: i64,
) -> Result<i64, sqlx::Error> {
    // Delete rows older than max_age_days.
    let cutoff = chrono::Utc::now() - chrono::Duration::days(max_age_days);
    let cutoff_str = cutoff.to_rfc3339();

    let age_result =
        sqlx::query("DELETE FROM status_history WHERE service_id = ? AND checked_at < ?")
            .bind(service_id.to_string())
            .bind(&cutoff_str)
            .execute(pool)
            .await?;

    let mut deleted = age_result.rows_affected();

    // Delete excess rows beyond max_rows (keep the most recent).
    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM status_history WHERE service_id = ?")
        .bind(service_id.to_string())
        .fetch_one(pool)
        .await?;

    if count.0 > max_rows {
        let excess = count.0 - max_rows;
        let excess_result = sqlx::query(
            "DELETE FROM status_history WHERE id IN ( \
             SELECT id FROM status_history WHERE service_id = ? ORDER BY checked_at ASC LIMIT ? \
             )",
        )
        .bind(service_id.to_string())
        .bind(excess)
        .execute(pool)
        .await?;
        deleted += excess_result.rows_affected();
    }

    Ok(deleted as i64)
}

/// List all current StatusReadings, filtered by visibility.
pub async fn list_status_readings(
    pool: &SqlitePool,
    filter: crate::domain::VisibilityFilter,
) -> Result<Vec<StatusReading>, sqlx::Error> {
    let query = match filter {
        crate::domain::VisibilityFilter::PublicOnly => sqlx::query(
            "SELECT sr.service_id, sr.state, sr.latency_ms, sr.reason, sr.checked_at \
                 FROM status_reading sr \
                 JOIN service s ON sr.service_id = s.id \
                 WHERE s.visibility = 'public'",
        ),
        crate::domain::VisibilityFilter::All => sqlx::query(
            "SELECT service_id, state, latency_ms, reason, checked_at FROM status_reading",
        ),
    };

    let rows = query.fetch_all(pool).await?;
    let mut readings = Vec::new();
    for row in rows {
        let id_str: String = row.get("service_id");
        let state_str: String = row.get("state");
        let state = MonitorState::from_str(&state_str).unwrap_or(MonitorState::Down);
        readings.push(StatusReading {
            service_id: Uuid::parse_str(&id_str).unwrap_or_default(),
            state,
            latency_ms: row.get("latency_ms"),
            reason: row.get("reason"),
            checked_at: row.get("checked_at"),
        });
    }
    Ok(readings)
}

/// Compute uptime summary from status_history over a time window.
pub async fn compute_uptime_summary(
    pool: &SqlitePool,
    service_id: Uuid,
    window_hours: u32,
) -> Result<UptimeSummary, AppError> {
    let cutoff = chrono::Utc::now() - chrono::Duration::hours(window_hours as i64);
    let cutoff_str = cutoff.to_rfc3339();

    let rows =
        sqlx::query("SELECT state FROM status_history WHERE service_id = ? AND checked_at >= ?")
            .bind(service_id.to_string())
            .bind(&cutoff_str)
            .fetch_all(pool)
            .await?;

    let total = rows.len() as u64;
    let up = rows
        .iter()
        .filter(|r| {
            let s: String = r.get("state");
            s == "up"
        })
        .count() as u64;

    let percent = if total > 0 {
        (up as f64 / total as f64) * 100.0
    } else {
        0.0
    };

    Ok(UptimeSummary {
        service_id,
        window_hours,
        total_checks: total,
        up_checks: up,
        uptime_percent: percent,
    })
}
