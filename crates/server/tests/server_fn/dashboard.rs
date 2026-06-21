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
