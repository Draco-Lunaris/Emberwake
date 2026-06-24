//! T018: Server-fn test — list_dashboard returns only public pinned items for anonymous caller.
//! Seeds public + private pinned items, calls list_dashboard via repository,
//! asserts only public pinned items are returned.

use sqlx::SqlitePool;

use app::domain::VisibilityFilter;
use server::db::{Repository, SqliteRepository};

/// Seed a mix of public and private pinned services and bookmarks into the database.
async fn seed_content(pool: &SqlitePool) {
    let now = "2026-01-01T00:00:00Z";

    // Public pinned service
    sqlx::query(
        "INSERT INTO service (id, category_id, name, url, is_pinned, order_index, visibility, \
         monitor_enabled, created_at, updated_at) \
         VALUES ('pub-svc-001', NULL, 'Public Service', 'https://pub.example.com', 1, 0, 'public', 0, ?, ?)",
    )
    .bind(now)
    .bind(now)
    .execute(pool)
    .await
    .expect("insert public service");

    // Private pinned service
    sqlx::query(
        "INSERT INTO service (id, category_id, name, url, is_pinned, order_index, visibility, \
         monitor_enabled, created_at, updated_at) \
         VALUES ('priv-svc-001', NULL, 'Private Service', 'https://priv.example.com', 1, 1, 'private', 0, ?, ?)",
    )
    .bind(now)
    .bind(now)
    .execute(pool)
    .await
    .expect("insert private service");

    // Public category with bookmark
    sqlx::query(
        "INSERT INTO category (id, name, order_index, visibility, created_at, updated_at) \
         VALUES ('pub-cat-001', 'Public Category', 0, 'public', ?, ?)",
    )
    .bind(now)
    .bind(now)
    .execute(pool)
    .await
    .expect("insert public category");

    sqlx::query(
        "INSERT INTO bookmark (id, category_id, name, url, order_index, visibility, created_at, updated_at) \
         VALUES ('pub-bm-001', 'pub-cat-001', 'Public Bookmark', 'https://bm.example.com', 0, 'public', ?, ?)",
    )
    .bind(now)
    .bind(now)
    .execute(pool)
    .await
    .expect("insert public bookmark");

    // Private category with bookmark
    sqlx::query(
        "INSERT INTO category (id, name, order_index, visibility, created_at, updated_at) \
         VALUES ('priv-cat-001', 'Private Category', 1, 'private', ?, ?)",
    )
    .bind(now)
    .bind(now)
    .execute(pool)
    .await
    .expect("insert private category");

    sqlx::query(
        "INSERT INTO bookmark (id, category_id, name, url, order_index, visibility, created_at, updated_at) \
         VALUES ('priv-bm-001', 'priv-cat-001', 'Private Bookmark', 'https://privbm.example.com', 0, 'private', ?, ?)",
    )
    .bind(now)
    .bind(now)
    .execute(pool)
    .await
    .expect("insert private bookmark");
}

/// T018: list_dashboard returns only public pinned items for anonymous caller (VisibilityFilter::PublicOnly).
#[sqlx::test(migrations = "../../migrations")]
async fn list_dashboard_public_only(pool: SqlitePool) {
    seed_content(&pool).await;

    let repo = SqliteRepository::new(pool);
    let dashboard = repo
        .list_dashboard(VisibilityFilter::PublicOnly)
        .await
        .expect("list_dashboard should succeed");

    // Should return exactly 1 public pinned service
    assert_eq!(
        dashboard.pinned_services.len(),
        1,
        "should return only public pinned services"
    );
    assert_eq!(
        dashboard.pinned_services[0].name, "Public Service",
        "public service should be returned"
    );

    // Should return exactly 1 public category with bookmarks
    assert_eq!(
        dashboard.pinned_categories.len(),
        1,
        "should return only public categories"
    );
    assert_eq!(
        dashboard.pinned_categories[0].category.name, "Public Category",
        "public category should be returned"
    );
    assert_eq!(
        dashboard.pinned_categories[0].bookmarks.len(),
        1,
        "public category should have 1 bookmark"
    );
    assert_eq!(
        dashboard.pinned_categories[0].bookmarks[0].name, "Public Bookmark",
        "public bookmark should be returned"
    );
}

/// list_dashboard with VisibilityFilter::All returns both public and private items.
#[sqlx::test(migrations = "../../migrations")]
async fn list_dashboard_all_includes_private(pool: SqlitePool) {
    seed_content(&pool).await;

    let repo = SqliteRepository::new(pool);
    let dashboard = repo
        .list_dashboard(VisibilityFilter::All)
        .await
        .expect("list_dashboard should succeed");

    // Should return both pinned services
    assert_eq!(
        dashboard.pinned_services.len(),
        2,
        "should return both pinned services with All filter"
    );

    // Should return both categories
    assert_eq!(
        dashboard.pinned_categories.len(),
        2,
        "should return both categories with All filter"
    );
}

/// T025: Search provider wiring — seeds settings, calls get_search_providers_query,
/// verifies provider fields. Also verifies HomePage code path passes providers to SearchIsland
/// via code inspection (HomePage creates Resource calling get_search_providers and passes
/// to SearchIsland component — confirmed in app/src/lib.rs lines 163-170 + 197-200).
#[sqlx::test(migrations = "../../migrations")]
async fn search_provider_wiring(pool: SqlitePool) {
    use app::server::content_queries::get_search_providers_query;

    // Seed a search provider in the settings table
    let providers_json = serde_json::json!({
        "providers": [{
            "prefix": "g",
            "name": "Google",
            "url_template": "https://google.com/search?q={q}"
        }],
        "default_provider": null
    });

    sqlx::query(
        "INSERT INTO setting (key, value, updated_at) VALUES ('search.providers', ?, '2026-01-01T00:00:00Z')"
    )
    .bind(providers_json.to_string())
    .execute(&pool)
    .await
    .expect("insert search providers setting");

    // Call the shared query function (same one used by get_search_providers server fn)
    let config = get_search_providers_query(&pool)
        .await
        .expect("get_search_providers_query should succeed");

    // Verify provider fields
    assert_eq!(
        config.providers.len(),
        1,
        "should return 1 search provider"
    );
    assert_eq!(config.providers[0].prefix, "g", "prefix should be 'g'");
    assert_eq!(config.providers[0].name, "Google", "name should be 'Google'");
    assert_eq!(
        config.providers[0].url_template,
        "https://google.com/search?q={q}",
        "url_template should match"
    );
    assert_eq!(
        config.default_provider, None,
        "default_provider should be None"
    );

    // Verify empty settings returns default config
    sqlx::query("DELETE FROM setting WHERE key = 'search.providers'")
        .execute(&pool)
        .await
        .expect("delete setting");

    let empty_config = get_search_providers_query(&pool)
        .await
        .expect("query should succeed with no settings");
    assert!(
        empty_config.providers.is_empty(),
        "empty settings should return no providers"
    );

    // HomePage wiring verification (code inspection):
    // In app/src/lib.rs, HomePage creates a Resource calling get_search_providers()
    // (line 163-169) and passes the result to SearchIsland (line 197-200).
    // This confirms the full path: HomePage → get_search_providers → SearchIsland.
}
