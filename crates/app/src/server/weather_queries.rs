//! SQL query functions for weather reading cache (US7).
//! All functions are ssr-only. Uses parameterized SQL with static string literals.
//! The weather_reading table is a single-row cache (id=1, CHECK constraint).

#![cfg(feature = "ssr")]

use sqlx::{Row, SqlitePool};

use crate::domain::WeatherReading;
use crate::error::AppError;

/// Get the cached weather reading (single-row, id=1).
pub async fn get_weather_reading(pool: &SqlitePool) -> Result<Option<WeatherReading>, sqlx::Error> {
    let row = sqlx::query(
        "SELECT temp, condition, is_day, cloud, upstream_ts, fetched_at \
         FROM weather_reading WHERE id = 1",
    )
    .fetch_optional(pool)
    .await?;

    match row {
        Some(r) => {
            let temp: Option<f64> = r.get("temp");
            let condition: Option<String> = r.get("condition");
            let is_day_int: Option<i64> = r.get("is_day");
            let is_day = is_day_int.map(|v| v != 0);
            let cloud: Option<i64> = r.get("cloud");
            let upstream_ts: Option<String> = r.get("upstream_ts");
            let fetched_at: String = r.get("fetched_at");
            Ok(Some(WeatherReading {
                temp,
                condition,
                is_day,
                cloud,
                upstream_ts,
                fetched_at,
            }))
        }
        None => Ok(None),
    }
}

/// Upsert the weather reading cache (single-row, id=1).
/// On upstream error, the caller should NOT call this — retain last good state.
pub async fn upsert_weather_reading(
    pool: &SqlitePool,
    temp: Option<f64>,
    condition: Option<&str>,
    is_day: Option<bool>,
    cloud: Option<i64>,
    upstream_ts: Option<&str>,
    fetched_at: &str,
) -> Result<(), AppError> {
    let is_day_int = is_day.map(|v| if v { 1 } else { 0 });

    sqlx::query(
        "INSERT INTO weather_reading (id, temp, condition, is_day, cloud, upstream_ts, fetched_at) \
         VALUES (1, ?, ?, ?, ?, ?, ?) \
         ON CONFLICT(id) DO UPDATE SET \
           temp = excluded.temp, \
           condition = excluded.condition, \
           is_day = excluded.is_day, \
           cloud = excluded.cloud, \
           upstream_ts = excluded.upstream_ts, \
           fetched_at = excluded.fetched_at",
    )
    .bind(temp)
    .bind(condition)
    .bind(is_day_int)
    .bind(cloud)
    .bind(upstream_ts)
    .bind(fetched_at)
    .execute(pool)
    .await?;
    Ok(())
}
