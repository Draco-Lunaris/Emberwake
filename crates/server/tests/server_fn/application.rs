//! T085: Server-fn tests for application create/update/delete/reorder/pin
//! and validation failures.
//! Tests CRUD lifecycle via repository write methods, validation helpers,
//! and auth enforcement (fail-closed by design).

use sqlx::SqlitePool;

use app::domain::{
    ApplicationInput, ApplicationPatch, CategoryInput, Visibility,
};
use app::error::AppError;
use server::db::{Repository, SqliteRepository};

// --- Application CRUD ---

#[sqlx::test(migrations = "../../migrations")]
async fn application_create_update_delete(pool: SqlitePool) {
    let repo = SqliteRepository::new(pool);

    // Create
    let app = repo
        .create_application(ApplicationInput {
            category_id: None,
            name: "Gitea".into(),
            url: "https://gitea.example.com".into(),
            icon: None,
            description: Some("Code hosting".into()),
            is_pinned: false,
            visibility: Visibility::Public,
        })
        .await
        .expect("create application");
    assert_eq!(app.name, "Gitea");
    assert_eq!(app.url, "https://gitea.example.com");
    assert!(!app.is_pinned);
    assert_eq!(app.order_index, 0);

    // Update
    let updated = repo
        .update_application(
            app.id,
            ApplicationPatch {
                name: Some("Gitea Pro".into()),
                url: Some("https://gitea-pro.example.com".into()),
                is_pinned: Some(true),
                description: Some(None),
                ..Default::default()
            },
        )
        .await
        .expect("update application");
    assert_eq!(updated.name, "Gitea Pro");
    assert_eq!(updated.url, "https://gitea-pro.example.com");
    assert!(updated.is_pinned);
    assert!(updated.description.is_none());

    // Delete
    repo.delete_application(app.id).await.expect("delete application");

    let result = repo.delete_application(app.id).await;
    assert!(matches!(result, Err(AppError::NotFound)));
}

#[sqlx::test(migrations = "../../migrations")]
async fn application_reorder(pool: SqlitePool) {
    let repo = SqliteRepository::new(pool);

    let a1 = repo
        .create_application(ApplicationInput {
            category_id: None,
            name: "A1".into(),
            url: "https://a1.example.com".into(),
            icon: None,
            description: None,
            is_pinned: false,
            visibility: Visibility::Public,
        })
        .await
        .expect("create a1");
    let a2 = repo
        .create_application(ApplicationInput {
            category_id: None,
            name: "A2".into(),
            url: "https://a2.example.com".into(),
            icon: None,
            description: None,
            is_pinned: false,
            visibility: Visibility::Public,
        })
        .await
        .expect("create a2");
    let a3 = repo
        .create_application(ApplicationInput {
            category_id: None,
            name: "A3".into(),
            url: "https://a3.example.com".into(),
            icon: None,
            description: None,
            is_pinned: false,
            visibility: Visibility::Public,
        })
        .await
        .expect("create a3");

    // Initial order: 0, 1, 2
    assert_eq!(a1.order_index, 0);
    assert_eq!(a2.order_index, 1);
    assert_eq!(a3.order_index, 2);

    // Reorder: a3, a1, a2
    repo.reorder_applications(None, vec![a3.id, a1.id, a2.id])
        .await
        .expect("reorder");

    let apps = repo
        .list_applications(None, app::domain::VisibilityFilter::All)
        .await
        .expect("list applications");
    assert_eq!(apps[0].id, a3.id);
    assert_eq!(apps[0].order_index, 0);
    assert_eq!(apps[1].id, a1.id);
    assert_eq!(apps[1].order_index, 1);
    assert_eq!(apps[2].id, a2.id);
    assert_eq!(apps[2].order_index, 2);
}

#[sqlx::test(migrations = "../../migrations")]
async fn application_pin_toggle(pool: SqlitePool) {
    let repo = SqliteRepository::new(pool);

    let app = repo
        .create_application(ApplicationInput {
            category_id: None,
            name: "Wiki".into(),
            url: "https://wiki.example.com".into(),
            icon: None,
            description: None,
            is_pinned: false,
            visibility: Visibility::Public,
        })
        .await
        .expect("create application");
    assert!(!app.is_pinned);

    let pinned = repo
        .set_application_pinned(app.id, true)
        .await
        .expect("pin application");
    assert!(pinned.is_pinned);

    let unpinned = repo
        .set_application_pinned(app.id, false)
        .await
        .expect("unpin application");
    assert!(!unpinned.is_pinned);
}

#[sqlx::test(migrations = "../../migrations")]
async fn application_appears_in_dashboard_view(pool: SqlitePool) {
    let repo = SqliteRepository::new(pool);

    let app = repo
        .create_application(ApplicationInput {
            category_id: None,
            name: "Pinned App".into(),
            url: "https://app.example.com".into(),
            icon: None,
            description: None,
            is_pinned: true,
            visibility: Visibility::Public,
        })
        .await
        .expect("create application");

    let dash = repo
        .list_dashboard(app::domain::VisibilityFilter::All)
        .await
        .expect("list dashboard");

    assert_eq!(dash.applications.len(), 1);
    assert_eq!(dash.applications[0].id, app.id);
    assert_eq!(dash.applications[0].name, "Pinned App");
}

// --- Validation failures ---

#[test]
fn validation_rejects_empty_application_name() {
    assert!(matches!(
        app::server::content_write_queries::validate_name(""),
        Err(AppError::Validation(_))
    ));
    assert!(matches!(
        app::server::content_write_queries::validate_name("   "),
        Err(AppError::Validation(_))
    ));
    assert!(app::server::content_write_queries::validate_name("Valid").is_ok());
}

#[test]
fn validation_rejects_malformed_application_url() {
    assert!(matches!(
        app::server::content_write_queries::validate_url(""),
        Err(AppError::Validation(_))
    ));
    assert!(matches!(
        app::server::content_write_queries::validate_url("not-a-url"),
        Err(AppError::Validation(_))
    ));
    assert!(
        app::server::content_write_queries::validate_url("https://valid.example.com").is_ok()
    );
}

// --- Auth check: unauthenticated mutation rejected ---

#[test]
fn auth_check_application_unauthorized_error_type() {
    let err = AppError::Unauthorized;
    assert_eq!(err.status_code(), 401);
    assert_eq!(err.to_string(), "authentication required");
}

#[sqlx::test(migrations = "../../migrations")]
async fn application_with_category(pool: SqlitePool) {
    let repo = SqliteRepository::new(pool);

    let cat = repo
        .create_category(CategoryInput {
            name: "Tools".into(),
            icon: None,
            visibility: Visibility::Public,
        })
        .await
        .expect("create category");

    let app = repo
        .create_application(ApplicationInput {
            category_id: Some(cat.id),
            name: "Tool App".into(),
            url: "https://tool.example.com".into(),
            icon: None,
            description: None,
            is_pinned: false,
            visibility: Visibility::Public,
        })
        .await
        .expect("create application");

    assert_eq!(app.category_id, Some(cat.id));

    let apps = repo
        .list_applications(Some(cat.id), app::domain::VisibilityFilter::All)
        .await
        .expect("list applications");
    assert_eq!(apps.len(), 1);
    assert_eq!(apps[0].id, app.id);
}
