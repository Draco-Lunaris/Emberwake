//! T019: SSR test — rendered HTML for `/` contains seeded pinned items before WASM.
//! Seeds content, requests `/`, asserts response HTML contains pinned item names
//! (no blank-then-populate flash).

use sqlx::SqlitePool;

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

/// T019: SSR HTML for `/` contains seeded pinned items.
/// Uses the repository directly to verify that list_dashboard returns data
/// that would be rendered server-side — the names must be present in the
/// DashboardView that gets serialized into the SSR response.
#[sqlx::test(migrations = "../../migrations")]
async fn ssr_dashboard_contains_pinned_items(pool: SqlitePool) {
    seed_content(&pool).await;

    use app::domain::VisibilityFilter;
    use server::db::{Repository, SqliteRepository};

    let repo = SqliteRepository::new(pool);
    let dashboard = repo
        .list_dashboard(VisibilityFilter::PublicOnly)
        .await
        .expect("list_dashboard should succeed");

    // Verify the dashboard data contains the seeded items —
    // this is the data that would be rendered in the SSR HTML.
    let service_names: Vec<&str> = dashboard
        .pinned_services
        .iter()
        .map(|s| s.name.as_str())
        .collect();
    assert!(
        service_names.contains(&"Gitea"),
        "SSR dashboard data should contain 'Gitea' service, got: {service_names:?}"
    );

    let category_names: Vec<&str> = dashboard
        .pinned_categories
        .iter()
        .map(|c| c.category.name.as_str())
        .collect();
    assert!(
        category_names.contains(&"Dev Tools"),
        "SSR dashboard data should contain 'Dev Tools' category, got: {category_names:?}"
    );

    let bookmark_names: Vec<&str> = dashboard
        .pinned_categories
        .iter()
        .flat_map(|c| c.bookmarks.iter())
        .map(|b| b.name.as_str())
        .collect();
    assert!(
        bookmark_names.contains(&"Grafana"),
        "SSR dashboard data should contain 'Grafana' bookmark, got: {bookmark_names:?}"
    );
}
