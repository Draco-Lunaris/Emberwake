//! T061: Weather widget integration tests.
//! Tests: scheduled fetch caches a reading; widget reads cache; SSE push on refresh;
//! missing config = inert (no fetch, no error, no event).

use app::domain::WeatherSettings;
use app::server::settings_queries;
use app::server::weather_queries;
use server::integrations::weather;
use server::sse::{SseEvent, SseHub};
use sqlx::SqlitePool;

/// Seed weather settings into the Setting table.
async fn seed_weather_settings(
    pool: &SqlitePool,
    api_key: &str,
    location: &str,
    api_url: &str,
    enabled: bool,
    server_key: &[u8],
) {
    let ws = WeatherSettings {
        api_key: Some(api_key.to_string()),
        api_url: Some(api_url.to_string()),
        location: Some(location.to_string()),
        refresh_interval_s: Some(60),
        enabled,
    };
    let json = serde_json::to_string(&ws).unwrap();
    settings_queries::set_setting_raw(pool, settings_queries::keys::WEATHER, &json, server_key)
        .await
        .expect("seed weather settings");
}

/// Start a stub HTTP server that returns a weather API JSON response.
/// Returns the base URL (e.g. "http://127.0.0.1:PORT").
async fn start_stub_weather_server() -> String {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    let addr = listener.local_addr().expect("local addr");
    let port = addr.port();

    tokio::spawn(async move {
        loop {
            let (mut stream, _) = match listener.accept().await {
                Ok(s) => s,
                Err(_) => return,
            };

            // Read the request (we don't care about content, just consume it).
            let mut buf = [0u8; 1024];
            let _ = tokio::io::AsyncReadExt::read(&mut stream, &mut buf).await;

            // Write a JSON response matching WeatherAPI format.
            let body = r#"{"current":{"temp_c":21.4,"condition":{"text":"Sunny"},"is_day":1,"cloud":0,"last_updated":"2026-06-20T12:00:00Z"}}"#;
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            let _ = tokio::io::AsyncWriteExt::write_all(&mut stream, response.as_bytes()).await;
            let _ = tokio::io::AsyncWriteExt::shutdown(&mut stream).await;
        }
    });

    format!("http://127.0.0.1:{port}")
}

/// T061: Scheduled fetch caches a reading; get_weather reads cache; SSE push on refresh.
#[sqlx::test(migrations = "../../migrations")]
async fn weather_fetch_caches_and_pushes_sse(pool: SqlitePool) {
    let server_key = b"test-server-key-32-bytes-long-aaaa";
    let stub_url = start_stub_weather_server().await;

    // Seed weather settings pointing to the stub server.
    seed_weather_settings(
        &pool,
        "test-api-key",
        "London",
        &format!("{stub_url}/v1/current.json"),
        true,
        server_key,
    )
    .await;

    let sse_hub = SseHub::new(64);
    let mut rx = sse_hub.subscribe();

    // Run a weather refresh (simulates scheduled fetch).
    weather::refresh_weather(&pool, &sse_hub, server_key).await;

    // Assert WeatherReading row is written to DB (cache).
    let reading = weather_queries::get_weather_reading(&pool)
        .await
        .expect("get weather reading");
    let reading = reading.expect("reading should exist after fetch");
    assert_eq!(reading.temp, Some(21.4));
    assert_eq!(reading.condition.as_deref(), Some("Sunny"));
    assert_eq!(reading.is_day, Some(true));
    assert_eq!(reading.cloud, Some(0));
    assert!(!reading.fetched_at.is_empty(), "fetched_at should be set");

    // Assert get_weather reads from cache (no synchronous upstream call).
    // We verify by checking the cached values match what was fetched.
    let cached = weather_queries::get_weather_reading(&pool)
        .await
        .expect("get weather reading (cache read)")
        .expect("cache should have reading");
    assert_eq!(cached.temp, Some(21.4));
    assert_eq!(cached.condition.as_deref(), Some("Sunny"));

    // Assert SSE weather event was emitted.
    let event = tokio::time::timeout(std::time::Duration::from_secs(2), rx.recv())
        .await
        .expect("should receive SSE weather event within timeout");
    let event = event.expect("event should not be an error");
    match event {
        SseEvent::Weather(data) => {
            assert_eq!(data.get("tempC").and_then(|v| v.as_f64()), Some(21.4));
            assert_eq!(
                data.get("conditionCode").and_then(|v| v.as_str()),
                Some("Sunny")
            );
            assert_eq!(data.get("isDay").and_then(|v| v.as_bool()), Some(true));
        }
        _ => panic!("expected weather event"),
    }
}

/// T061: Missing config = inert (no fetch, no error, no event).
#[sqlx::test(migrations = "../../migrations")]
async fn weather_inert_when_unconfigured(pool: SqlitePool) {
    let server_key = b"test-server-key-32-bytes-long-aaaa";
    let sse_hub = SseHub::new(64);
    let mut rx = sse_hub.subscribe();

    // No weather settings seeded — should be inert.
    weather::refresh_weather(&pool, &sse_hub, server_key).await;

    // Assert no WeatherReading row was written.
    let reading = weather_queries::get_weather_reading(&pool)
        .await
        .expect("get weather reading");
    assert!(
        reading.is_none(),
        "no reading should exist when unconfigured"
    );

    // Assert no SSE event was emitted.
    let event = tokio::time::timeout(std::time::Duration::from_millis(100), rx.recv()).await;
    assert!(
        event.is_err() || event.unwrap().is_err(),
        "no SSE event should be emitted when unconfigured"
    );
}

/// T061: Disabled weather = inert (no fetch, no error, no event).
#[sqlx::test(migrations = "../../migrations")]
async fn weather_inert_when_disabled(pool: SqlitePool) {
    let server_key = b"test-server-key-32-bytes-long-aaaa";
    let stub_url = start_stub_weather_server().await;

    // Seed weather settings but with enabled=false.
    seed_weather_settings(
        &pool,
        "test-api-key",
        "London",
        &format!("{stub_url}/v1/current.json"),
        false, // disabled
        server_key,
    )
    .await;

    let sse_hub = SseHub::new(64);
    let mut rx = sse_hub.subscribe();

    // Run a weather refresh — should be inert (disabled).
    weather::refresh_weather(&pool, &sse_hub, server_key).await;

    // Assert no WeatherReading row was written.
    let reading = weather_queries::get_weather_reading(&pool)
        .await
        .expect("get weather reading");
    assert!(reading.is_none(), "no reading should exist when disabled");

    // Assert no SSE event was emitted.
    let event = tokio::time::timeout(std::time::Duration::from_millis(100), rx.recv()).await;
    assert!(
        event.is_err() || event.unwrap().is_err(),
        "no SSE event should be emitted when disabled"
    );
}

/// T061: Upstream error retains last good cache state; no SSE event on error.
#[sqlx::test(migrations = "../../migrations")]
async fn weather_upstream_error_retains_cache(pool: SqlitePool) {
    let server_key = b"test-server-key-32-bytes-long-aaaa";

    // Seed an initial good reading into the cache.
    weather_queries::upsert_weather_reading(
        &pool,
        Some(15.0),
        Some("Cloudy"),
        Some(false),
        Some(50),
        Some("2026-06-20T10:00:00Z"),
        "2026-06-20T10:00:00Z",
    )
    .await
    .expect("seed initial reading");

    // Seed weather settings pointing to a non-existent server (will fail).
    seed_weather_settings(
        &pool,
        "test-api-key",
        "London",
        "http://127.0.0.1:1/v1/current.json", // port 1 = connection refused
        true,
        server_key,
    )
    .await;

    let sse_hub = SseHub::new(64);
    let mut rx = sse_hub.subscribe();

    // Run a weather refresh — should fail but retain last good cache.
    weather::refresh_weather(&pool, &sse_hub, server_key).await;

    // Assert the cache still has the old good reading.
    let reading = weather_queries::get_weather_reading(&pool)
        .await
        .expect("get weather reading");
    let reading = reading.expect("cache should still have last good reading");
    assert_eq!(reading.temp, Some(15.0));
    assert_eq!(reading.condition.as_deref(), Some("Cloudy"));
    assert_eq!(reading.is_day, Some(false));
    assert_eq!(reading.cloud, Some(50));

    // Assert no SSE event was emitted (upstream error = no event).
    let event = tokio::time::timeout(std::time::Duration::from_millis(100), rx.recv()).await;
    assert!(
        event.is_err() || event.unwrap().is_err(),
        "no SSE event should be emitted on upstream error"
    );
}
