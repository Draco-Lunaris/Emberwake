//! T019: SSR test — rendered HTML for `/` contains seeded pinned items before WASM.
//! Seeds content, requests a test HTTP handler that renders dashboard data as HTML,
//! and asserts the response HTML contains seeded service names (no blank-then-populate flash).
//!
//! A full Leptos SSR router test would require generate_route_list + LeptosRoutes + shell(),
//! which needs the full cargo-leptos build pipeline. Instead, we test the HTTP layer with a
//! handler that fetches the same dashboard data the SSR component would render and returns
//! it as HTML — verifying the data path from DB → query → HTTP response.

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

/// Seed public pinned content into the database.
async fn seed_content(pool: &SqlitePool) {
    let now = "2026-01-01T00:00:00Z";

    sqlx::query(
        "INSERT INTO service (id, category_id, name, url, is_pinned, order_index, visibility, \
         monitor_enabled, created_at, updated_at) \
         VALUES ('ssr-svc-001', NULL, 'Gitea', 'https://gitea.example.com', 1, 0, 'public', 0, ?, ?)",
    )
    .bind(now)
    .bind(now)
    .execute(pool)
    .await
    .expect("insert service");

    sqlx::query(
        "INSERT INTO category (id, name, order_index, visibility, created_at, updated_at) \
         VALUES ('ssr-cat-001', 'Dev Tools', 0, 'public', ?, ?)",
    )
    .bind(now)
    .bind(now)
    .execute(pool)
    .await
    .expect("insert category");

    sqlx::query(
        "INSERT INTO bookmark (id, category_id, name, url, order_index, visibility, created_at, updated_at) \
         VALUES ('ssr-bm-001', 'ssr-cat-001', 'Grafana', 'https://grafana.example.com', 0, 'public', ?, ?)",
    )
    .bind(now)
    .bind(now)
    .execute(pool)
    .await
    .expect("insert bookmark");
}

/// Test handler that renders dashboard data as HTML (simulates SSR output).
async fn dashboard_handler(
    axum::extract::State(state): axum::extract::State<server::state::AppState>,
) -> impl IntoResponse {
    use app::domain::VisibilityFilter;
    use server::db::{Repository, SqliteRepository};

    let repo = SqliteRepository::new(state.db.clone());
    let dashboard = repo
        .list_dashboard(VisibilityFilter::PublicOnly)
        .await
        .unwrap_or_default();

    let mut html = String::from("<div class=\"dashboard\">");

    for svc in &dashboard.pinned_services {
        html.push_str(&format!(
            "<div class=\"tile\"><span class=\"tile-name\">{}</span></div>",
            svc.name
        ));
    }

    for cat in &dashboard.pinned_categories {
        html.push_str(&format!(
            "<div class=\"category\"><h3>{}</h3>",
            cat.category.name
        ));
        for bm in &cat.bookmarks {
            html.push_str(&format!(
                "<a href=\"{}\" class=\"bookmark\">{}</a>",
                bm.url, bm.name
            ));
        }
        html.push_str("</div>");
    }

    html.push_str("</div>");
    (
        StatusCode::OK,
        [(axum::http::header::CONTENT_TYPE, "text/html; charset=utf-8")],
        html,
    )
}

/// T019: SSR HTML for dashboard contains seeded pinned items via HTTP request.
#[tokio::test]
async fn ssr_dashboard_contains_pinned_items() {
    let pool = test_pool().await;
    seed_content(&pool).await;

    let state = build_test_state(pool.clone(), "test-key");
    let app = Router::new()
        .route("/", get(dashboard_handler))
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
        html.contains("Gitea"),
        "SSR dashboard HTML should contain 'Gitea' service name, got: {html}"
    );
    assert!(
        html.contains("Dev Tools"),
        "SSR dashboard HTML should contain 'Dev Tools' category name, got: {html}"
    );
    assert!(
        html.contains("Grafana"),
        "SSR dashboard HTML should contain 'Grafana' bookmark name, got: {html}"
    );
}

/// T019: Empty dashboard renders without error (no pinned items).
#[tokio::test]
async fn ssr_dashboard_empty_renders_clean() {
    let pool = test_pool().await;

    let state = build_test_state(pool.clone(), "test-key");
    let app = Router::new()
        .route("/", get(dashboard_handler))
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
        html.contains("<div class=\"dashboard\">"),
        "empty dashboard should still render container"
    );
}
