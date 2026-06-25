//! T086: Tests for per-section dashboard settings (enable/disable + column counts).

use sqlx::SqlitePool;
use app::server::settings_queries;

const SERVER_KEY: &[u8] = b"test-server-key";

#[sqlx::test(migrations = "../../migrations")]
async fn dashboard_settings_defaults_when_unset(pool: SqlitePool) {
    let svc_en = settings_queries::get_setting_raw(&pool, "dashboard.services.enabled").await.unwrap();
    assert!(svc_en.is_none(), "services.enabled should be unset by default");
    
    let app_cols = settings_queries::get_setting_raw(&pool, "dashboard.applications.columns").await.unwrap();
    assert!(app_cols.is_none(), "applications.columns should be unset by default");
}

#[sqlx::test(migrations = "../../migrations")]
async fn dashboard_settings_persist_after_update(pool: SqlitePool) {
    settings_queries::set_setting_raw(&pool, "dashboard.services.enabled", "false", SERVER_KEY).await.unwrap();
    settings_queries::set_setting_raw(&pool, "dashboard.services.columns", "6", SERVER_KEY).await.unwrap();
    settings_queries::set_setting_raw(&pool, "dashboard.applications.enabled", "false", SERVER_KEY).await.unwrap();
    settings_queries::set_setting_raw(&pool, "dashboard.applications.columns", "2", SERVER_KEY).await.unwrap();
    settings_queries::set_setting_raw(&pool, "dashboard.bookmarks.enabled", "true", SERVER_KEY).await.unwrap();
    settings_queries::set_setting_raw(&pool, "dashboard.bookmarks.columns", "5", SERVER_KEY).await.unwrap();
    
    assert_eq!(settings_queries::get_setting_raw(&pool, "dashboard.services.enabled").await.unwrap().unwrap(), "false");
    assert_eq!(settings_queries::get_setting_raw(&pool, "dashboard.services.columns").await.unwrap().unwrap(), "6");
    assert_eq!(settings_queries::get_setting_raw(&pool, "dashboard.applications.enabled").await.unwrap().unwrap(), "false");
    assert_eq!(settings_queries::get_setting_raw(&pool, "dashboard.applications.columns").await.unwrap().unwrap(), "2");
    assert_eq!(settings_queries::get_setting_raw(&pool, "dashboard.bookmarks.enabled").await.unwrap().unwrap(), "true");
    assert_eq!(settings_queries::get_setting_raw(&pool, "dashboard.bookmarks.columns").await.unwrap().unwrap(), "5");
}

#[sqlx::test(migrations = "../../migrations")]
async fn dashboard_settings_overwrite_existing(pool: SqlitePool) {
    settings_queries::set_setting_raw(&pool, "dashboard.services.columns", "3", SERVER_KEY).await.unwrap();
    settings_queries::set_setting_raw(&pool, "dashboard.services.columns", "8", SERVER_KEY).await.unwrap();
    
    assert_eq!(settings_queries::get_setting_raw(&pool, "dashboard.services.columns").await.unwrap().unwrap(), "8");
}
