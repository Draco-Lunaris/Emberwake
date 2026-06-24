//! T051: SSR test — active theme tokens + custom CSS present in first response (no default-theme flash).
//! Seeds a theme with specific design tokens + custom CSS, sets it as active,
//! and asserts the rendered HTML contains <style> tags with CSS custom properties.
//!
//! A full Leptos SSR router test would require generate_route_list + LeptosRoutes + shell(),
//! which needs the full cargo-leptos build pipeline. Instead, we test the HTTP layer with a
//! handler that fetches the active theme and renders CSS custom properties as <style> tags —
//! verifying the data path from DB → query → HTTP response (no flash of default theme).

#[path = "../common/mod.rs"]
mod common;

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::response::IntoResponse;
use axum::routing::get;
use common::{build_test_state, test_pool};
use sqlx::SqlitePool;
use tower::ServiceExt;

use app::domain::DesignTokens;
use app::server::settings_queries;

const SERVER_KEY: &[u8] = b"test-server-key-for-encryption";

/// Seed a custom theme with specific design tokens + custom CSS and set it active.
async fn seed_active_theme(pool: &SqlitePool) {
    settings_queries::seed_builtin_themes(pool)
        .await
        .expect("seed_builtin_themes");

    let tokens = DesignTokens {
        bg: Some("#1a1a2e".into()),
        surface: Some("#16213e".into()),
        text: Some("#e2e8f0".into()),
        accent: Some("#3b82f6".into()),
        border: Some("#334155".into()),
        radius: Some("12px".into()),
        mode: Some("dark".into()),
        ..Default::default()
    };

    let theme = settings_queries::save_theme_query(
        pool,
        &app::domain::ThemeInput {
            name: "Custom SSR Theme".into(),
            tokens,
            custom_css: Some(".tile { border-radius: 12px; }".into()),
        },
        None,
    )
    .await
    .expect("save_theme");

    settings_queries::set_active_theme_query(pool, theme.id, SERVER_KEY)
        .await
        .expect("set_active_theme");
}

/// Test handler that renders active theme as CSS custom properties in a <style> tag.
/// This simulates the SSR rendering that app/src/lib.rs does via `<Style>`.
async fn theme_handler(
    axum::extract::State(state): axum::extract::State<server::state::AppState>,
) -> impl IntoResponse {
    let active = settings_queries::get_active_theme_query(&state.db)
        .await
        .unwrap_or(None);

    let css = match active {
        Some(theme) => {
            let mut css = String::from(":root {");
            if let Some(ref bg) = theme.tokens.bg {
                css.push_str(&format!(" --bg: {bg};"));
            }
            if let Some(ref surface) = theme.tokens.surface {
                css.push_str(&format!(" --surface: {surface};"));
            }
            if let Some(ref text) = theme.tokens.text {
                css.push_str(&format!(" --text: {text};"));
            }
            if let Some(ref accent) = theme.tokens.accent {
                css.push_str(&format!(" --accent: {accent};"));
            }
            if let Some(ref border) = theme.tokens.border {
                css.push_str(&format!(" --border: {border};"));
            }
            if let Some(ref radius) = theme.tokens.radius {
                css.push_str(&format!(" --radius: {radius};"));
            }
            css.push_str(" }");
            if let Some(ref custom) = theme.custom_css {
                css.push_str(custom);
            }
            css
        }
        None => {
            "@media (prefers-color-scheme: dark) { :root { --bg: #1a1a2e; --surface: #16213e; --text: #e2e8f0; --accent: #3b82f6; --border: #334155; --radius: 8px; } }".to_string()
        }
    };

    let html = format!("<style>{css}</style>");
    (
        StatusCode::OK,
        [(axum::http::header::CONTENT_TYPE, "text/html; charset=utf-8")],
        html,
    )
}

/// T051: Active theme CSS custom properties present in HTTP response <style> tags.
#[tokio::test]
async fn ssr_active_theme_tokens_in_http_response() {
    let pool = test_pool().await;
    seed_active_theme(&pool).await;

    let state = build_test_state(pool.clone(), "test-key");
    let app = Router::new()
        .route("/", get(theme_handler))
        .with_state(state);

    let response = app
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), 65536)
        .await
        .expect("body");
    let html = String::from_utf8(body.to_vec()).expect("utf8");

    assert!(
        html.contains("<style>"),
        "SSR response should contain <style> tags with theme CSS"
    );
    assert!(
        html.contains("--bg: #1a1a2e"),
        "SSR response should contain --bg CSS custom property from active theme"
    );
    assert!(
        html.contains("--surface: #16213e"),
        "SSR response should contain --surface CSS custom property"
    );
    assert!(
        html.contains("--accent: #3b82f6"),
        "SSR response should contain --accent CSS custom property"
    );
    assert!(
        html.contains("--text: #e2e8f0"),
        "SSR response should contain --text CSS custom property from active theme"
    );
    assert!(
        html.contains("--border: #334155"),
        "SSR response should contain --border CSS custom property from active theme"
    );
    assert!(
        html.contains("--radius: 12px"),
        "SSR response should contain --radius CSS custom property"
    );
    assert!(
        html.contains(".tile { border-radius: 12px; }"),
        "SSR response should contain custom CSS from active theme"
    );
}

/// T051: No active theme → response uses prefers-color-scheme fallback CSS.
#[tokio::test]
async fn ssr_no_active_theme_uses_fallback() {
    let pool = test_pool().await;

    let state = build_test_state(pool.clone(), "test-key");
    let app = Router::new()
        .route("/", get(theme_handler))
        .with_state(state);

    let response = app
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), 65536)
        .await
        .expect("body");
    let html = String::from_utf8(body.to_vec()).expect("utf8");

    assert!(
        html.contains("prefers-color-scheme"),
        "no active theme should fall back to prefers-color-scheme CSS"
    );
}

/// T051: Active theme survives across simulated restart (re-read from DB via HTTP).
#[tokio::test]
async fn ssr_theme_survives_reload_http() {
    let pool = test_pool().await;
    seed_active_theme(&pool).await;

    // First request (initial page load)
    let state1 = build_test_state(pool.clone(), "test-key");
    let app1 = Router::new()
        .route("/", get(theme_handler))
        .with_state(state1);

    let response1 = app1
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .expect("response1");
    let body1 = axum::body::to_bytes(response1.into_body(), 65536)
        .await
        .expect("body1");
    let html1 = String::from_utf8(body1.to_vec()).expect("utf8");

    // Second request (page reload — simulates no flash since theme is server-side)
    let state2 = build_test_state(pool.clone(), "test-key");
    let app2 = Router::new()
        .route("/", get(theme_handler))
        .with_state(state2);

    let response2 = app2
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .expect("response2");
    let body2 = axum::body::to_bytes(response2.into_body(), 65536)
        .await
        .expect("body2");
    let html2 = String::from_utf8(body2.to_vec()).expect("utf8");

    assert_eq!(
        html1, html2,
        "same theme should produce identical SSR HTML after reload (no flash)"
    );
    assert!(
        html1.contains("--bg: #1a1a2e"),
        "theme tokens should be present in both responses"
    );
}
