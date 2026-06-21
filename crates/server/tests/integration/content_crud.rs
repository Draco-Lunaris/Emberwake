//! T027: Integration test — CRUD lifecycle persists across a simulated restart.
//! Creates entities, closes/reopens the DB pool, and verifies entities persist.

use sqlx::SqlitePool;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use std::str::FromStr;

use app::domain::{BookmarkInput, CategoryInput, ServiceInput, Visibility, VisibilityFilter};
use server::db::{Repository, SqliteRepository};

/// Create a fresh SQLite pool at a temp path with migrations applied.
async fn create_pool(db_path: &str) -> SqlitePool {
    let options = SqliteConnectOptions::from_str(&format!("sqlite://{db_path}"))
        .expect("valid connect options")
        .create_if_missing(true)
        .foreign_keys(true);

    let pool = SqlitePoolOptions::new()
        .max_connections(8)
        .connect_with(options)
        .await
        .expect("connect pool");

    sqlx::migrate!("../../migrations")
        .run(&pool)
        .await
        .expect("migrations");

    pool
}

/// T027: CRUD lifecycle persists across a simulated restart.
#[tokio::test]
async fn crud_persists_across_restart() {
    let tmp_dir = std::env::temp_dir();
    let db_path = format!(
        "{}/emberwake_t027_{}.db",
        tmp_dir.display(),
        uuid::Uuid::now_v7()
    );

    // IDs captured in phase 1 and verified in phase 2
    let (cat_id, svc_id, bm_id);

    // Phase 1: Create entities with first pool
    {
        let pool = create_pool(&db_path).await;
        let repo = SqliteRepository::new(pool.clone());

        let cat = repo
            .create_category(CategoryInput {
                name: "Dev Tools".into(),
                icon: None,
                visibility: Visibility::Public,
            })
            .await
            .expect("create category");

        let svc = repo
            .create_service(ServiceInput {
                category_id: Some(cat.id),
                name: "Gitea".into(),
                url: "https://gitea.example.com".into(),
                icon: None,
                description: Some("Code hosting".into()),
                is_pinned: true,
                visibility: Visibility::Public,
                monitor_enabled: false,
                monitor_kind: None,
                monitor_target: None,
                monitor_interval_s: None,
            })
            .await
            .expect("create service");

        let bm = repo
            .create_bookmark(BookmarkInput {
                category_id: Some(cat.id),
                name: "Docs".into(),
                url: "https://docs.example.com".into(),
                icon: None,
                visibility: Visibility::Public,
            })
            .await
            .expect("create bookmark");

        // Pin the service
        repo.set_service_pinned(svc.id, true)
            .await
            .expect("pin service");

        cat_id = cat.id;
        svc_id = svc.id;
        bm_id = bm.id;

        // Close the pool (simulates server restart)
        pool.close().await;
    }

    // Phase 2: Reopen pool and verify persistence
    {
        let pool = create_pool(&db_path).await;
        let repo = SqliteRepository::new(pool);

        // Verify category persists
        let cats = repo
            .list_categories(VisibilityFilter::All)
            .await
            .expect("list categories");
        let cat = cats
            .iter()
            .find(|c| c.id == cat_id)
            .expect("category should persist");
        assert_eq!(cat.name, "Dev Tools");

        // Verify service persists with correct fields
        let services = repo
            .list_services(None, VisibilityFilter::All)
            .await
            .expect("list services");
        let svc = services
            .iter()
            .find(|s| s.id == svc_id)
            .expect("service should persist");
        assert_eq!(svc.name, "Gitea");
        assert_eq!(svc.url, "https://gitea.example.com");
        assert_eq!(svc.category_id, Some(cat_id));
        assert!(svc.is_pinned, "service should remain pinned");

        // Verify bookmark persists
        let bookmarks = repo
            .list_bookmarks(Some(cat_id), VisibilityFilter::All)
            .await
            .expect("list bookmarks");
        let bm = bookmarks
            .iter()
            .find(|b| b.id == bm_id)
            .expect("bookmark should persist");
        assert_eq!(bm.name, "Docs");
        assert_eq!(bm.url, "https://docs.example.com");
        assert_eq!(bm.category_id, Some(cat_id));
    }

    // Cleanup
    std::fs::remove_file(&db_path).ok();
}
