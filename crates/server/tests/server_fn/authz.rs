//! T033: Authz test — private rows excluded for anon/unauthorized in read fns.
//! Seeds public + private items; verifies visibility filter behavior.

use sqlx::SqlitePool;

use app::domain::VisibilityFilter;
use server::db::{Repository, SqliteRepository};

async fn seed_public_private(pool: &SqlitePool) {
    let now = "2026-01-01T00:00:00Z";

    sqlx::query(
        "INSERT INTO service (id, category_id, name, url, is_pinned, order_index, visibility, \
         monitor_enabled, created_at, updated_at) \
         VALUES ('pub-svc', NULL, 'Public Service', 'https://pub.example.com', 1, 0, 'public', 0, ?, ?)",
    )
    .bind(now).bind(now).execute(pool).await.expect("insert public service");

    sqlx::query(
        "INSERT INTO service (id, category_id, name, url, is_pinned, order_index, visibility, \
         monitor_enabled, created_at, updated_at) \
         VALUES ('priv-svc', NULL, 'Private Service', 'https://priv.example.com', 1, 1, 'private', 0, ?, ?)",
    )
    .bind(now).bind(now).execute(pool).await.expect("insert private service");

    sqlx::query(
        "INSERT INTO category (id, name, order_index, visibility, created_at, updated_at) \
         VALUES ('pub-cat', 'Public Cat', 0, 'public', ?, ?)",
    )
    .bind(now)
    .bind(now)
    .execute(pool)
    .await
    .expect("insert public category");

    sqlx::query(
        "INSERT INTO category (id, name, order_index, visibility, created_at, updated_at) \
         VALUES ('priv-cat', 'Private Cat', 1, 'private', ?, ?)",
    )
    .bind(now)
    .bind(now)
    .execute(pool)
    .await
    .expect("insert private category");

    sqlx::query(
        "INSERT INTO bookmark (id, category_id, name, url, order_index, visibility, created_at, updated_at) \
         VALUES ('pub-bm', 'pub-cat', 'Public BM', 'https://bm.example.com', 0, 'public', ?, ?)",
    )
    .bind(now).bind(now).execute(pool).await.expect("insert public bookmark");

    sqlx::query(
        "INSERT INTO bookmark (id, category_id, name, url, order_index, visibility, created_at, updated_at) \
         VALUES ('priv-bm', 'priv-cat', 'Private BM', 'https://privbm.example.com', 0, 'private', ?, ?)",
    )
    .bind(now).bind(now).execute(pool).await.expect("insert private bookmark");
}

#[sqlx::test(migrations = "../../migrations")]
async fn anonymous_sees_only_public(pool: SqlitePool) {
    seed_public_private(&pool).await;

    let repo = SqliteRepository::new(pool);

    // Dashboard: only public
    let dashboard = repo
        .list_dashboard(VisibilityFilter::PublicOnly)
        .await
        .expect("dashboard");
    assert_eq!(
        dashboard.pinned_services.len(),
        1,
        "anon should see 1 public pinned service"
    );
    assert_eq!(dashboard.pinned_services[0].name, "Public Service");

    // Categories: only public
    let cats = repo
        .list_categories(VisibilityFilter::PublicOnly)
        .await
        .expect("categories");
    assert_eq!(cats.len(), 1, "anon should see 1 public category");
    assert_eq!(cats[0].name, "Public Cat");

    // Services: only public
    let services = repo
        .list_services(None, VisibilityFilter::PublicOnly)
        .await
        .expect("services");
    assert_eq!(services.len(), 1, "anon should see 1 public service");

    // Bookmarks: only public
    let bookmarks = repo
        .list_bookmarks(None, VisibilityFilter::PublicOnly)
        .await
        .expect("bookmarks");
    assert_eq!(bookmarks.len(), 1, "anon should see 1 public bookmark");
}

#[sqlx::test(migrations = "../../migrations")]
async fn authenticated_sees_all(pool: SqlitePool) {
    seed_public_private(&pool).await;

    let repo = SqliteRepository::new(pool);

    // Dashboard: both public and private
    let dashboard = repo
        .list_dashboard(VisibilityFilter::All)
        .await
        .expect("dashboard");
    assert_eq!(
        dashboard.pinned_services.len(),
        2,
        "auth should see both pinned services"
    );

    // Categories: both
    let cats = repo
        .list_categories(VisibilityFilter::All)
        .await
        .expect("categories");
    assert_eq!(cats.len(), 2, "auth should see both categories");

    // Services: both
    let services = repo
        .list_services(None, VisibilityFilter::All)
        .await
        .expect("services");
    assert_eq!(services.len(), 2, "auth should see both services");

    // Bookmarks: both
    let bookmarks = repo
        .list_bookmarks(None, VisibilityFilter::All)
        .await
        .expect("bookmarks");
    assert_eq!(bookmarks.len(), 2, "auth should see both bookmarks");
}
