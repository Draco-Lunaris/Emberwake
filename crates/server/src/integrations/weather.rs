//! Weather API client + scheduled refresh task (US7).
//! Fetches weather from a configurable API, caches to DB, emits SSE events.
//! Config-gated: inert when weather is not configured (no outbound calls, no errors).
//! On upstream error: retain last good cache state, log warning, don't emit event.
//! All fetches are server-side — never called from the browser.

use std::sync::Arc;
use std::time::Duration;

use sqlx::SqlitePool;
use tokio::time::interval;

use crate::sse::SseHub;

/// Default refresh interval if not configured (10 minutes).
const DEFAULT_REFRESH_INTERVAL_S: i64 = 600;

/// Scheduler tick: how often to check if weather is configured (seconds).
const SCHEDULER_TICK_S: u64 = 60;

/// Parsed weather data from an upstream API response.
struct WeatherData {
    temp: Option<f64>,
    condition: Option<String>,
    is_day: Option<bool>,
    cloud: Option<i64>,
    upstream_ts: Option<String>,
}

/// Fetch weather from the configured API.
/// Returns Ok(data) on success, Err on any failure (network, parse, status).
async fn fetch_weather(
    api_url: &str,
    api_key: &str,
    location: &str,
) -> Result<WeatherData, String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .connect_timeout(Duration::from_secs(10))
        .build()
        .map_err(|e| format!("client build error: {e}"))?;

    let url = format!("{api_url}?key={api_key}&q={location}");

    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("request error: {e}"))?
        .error_for_status()
        .map_err(|e| format!("HTTP error: {e}"))?
        .json::<serde_json::Value>()
        .await
        .map_err(|e| format!("parse error: {e}"))?;

    let temp = resp
        .get("current")
        .and_then(|c| c.get("temp_c"))
        .and_then(|v| v.as_f64());

    let condition = resp
        .get("current")
        .and_then(|c| c.get("condition"))
        .and_then(|c| c.get("text"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let is_day = resp
        .get("current")
        .and_then(|c| c.get("is_day"))
        .and_then(|v| v.as_i64())
        .map(|v| v == 1);

    let cloud = resp
        .get("current")
        .and_then(|c| c.get("cloud"))
        .and_then(|v| v.as_i64());

    let upstream_ts = resp
        .get("current")
        .and_then(|c| c.get("last_updated"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    Ok(WeatherData {
        temp,
        condition,
        is_day,
        cloud,
        upstream_ts,
    })
}

/// Run a weather refresh: fetch from upstream, cache to DB, emit SSE event.
/// On upstream error: log warning, retain last good cache, don't emit.
pub async fn refresh_weather(pool: &SqlitePool, sse_hub: &SseHub, server_key: &[u8]) {
    // Read weather settings from DB.
    let weather = match app::server::settings_queries::get_weather_typed(
        pool, server_key, true, // include secrets — server-side only
    )
    .await
    {
        Ok(w) => w,
        Err(e) => {
            tracing::warn!("weather refresh: failed to read settings: {e}");
            return;
        }
    };

    // Config-gated: inert when not configured or disabled.
    if !weather.enabled
        || weather.api_key.as_deref().unwrap_or_default().is_empty()
        || weather.location.as_deref().unwrap_or_default().is_empty()
    {
        return; // inert — no outbound calls, no errors
    }

    let api_url = weather
        .api_url
        .as_deref()
        .unwrap_or("https://api.weatherapi.com/v1/current.json");
    let api_key = weather.api_key.as_deref().unwrap_or_default();
    let location = weather.location.as_deref().unwrap_or_default();

    match fetch_weather(api_url, api_key, location).await {
        Ok(data) => {
            let now = chrono::Utc::now().to_rfc3339();

            // Cache the reading (upsert single-row cache).
            if let Err(e) = app::server::weather_queries::upsert_weather_reading(
                pool,
                data.temp,
                data.condition.as_deref(),
                data.is_day,
                data.cloud,
                data.upstream_ts.as_deref(),
                &now,
            )
            .await
            {
                tracing::warn!("weather refresh: failed to cache reading: {e}");
                return;
            }

            // Emit SSE weather event.
            let sse_data = serde_json::json!({
                "tempC": data.temp,
                "conditionCode": data.condition,
                "isDay": data.is_day,
                "cloud": data.cloud,
            });
            sse_hub.broadcast_weather(sse_data);

            tracing::debug!("weather refresh: cached new reading");
        }
        Err(e) => {
            // On upstream error: retain last good state, log, don't emit.
            tracing::warn!("weather refresh: upstream error, retaining last good cache: {e}");
        }
    }
}

/// Spawn the weather scheduler as a background task.
/// Checks if weather is configured on each tick; if so, fetches and caches.
/// If not configured: does nothing — inert, no outbound calls, no errors.
pub fn spawn_scheduler(pool: SqlitePool, sse_hub: Arc<SseHub>, server_key: Vec<u8>) {
    tokio::spawn(async move {
        let mut tick = interval(Duration::from_secs(SCHEDULER_TICK_S));

        loop {
            tick.tick().await;

            // Check if weather is configured to determine the effective interval.
            let weather =
                match app::server::settings_queries::get_weather_typed(&pool, &server_key, true)
                    .await
                {
                    Ok(w) => w,
                    Err(e) => {
                        tracing::warn!("weather scheduler: failed to read settings: {e}");
                        continue;
                    }
                };

            // Inert when not configured or disabled.
            if !weather.enabled
                || weather.api_key.as_deref().unwrap_or_default().is_empty()
                || weather.location.as_deref().unwrap_or_default().is_empty()
            {
                continue; // no outbound calls
            }

            // Use configured interval or default.
            let interval_s = weather
                .refresh_interval_s
                .unwrap_or(DEFAULT_REFRESH_INTERVAL_S)
                .max(60) as u64; // minimum 1 minute

            // Run the refresh.
            refresh_weather(&pool, &sse_hub, &server_key).await;

            // Reset the tick interval to match the configured refresh interval.
            tick = interval(Duration::from_secs(interval_s));
        }
    });
}
