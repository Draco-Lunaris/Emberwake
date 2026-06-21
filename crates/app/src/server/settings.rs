//! Settings and theme server functions (US5).
//! Mutations enforce auth + CSRF + authorization (admin-gated) and are audited.
//! Reads: list_themes and get_active_theme are public; get_settings is admin (secrets redacted for others).

use leptos::server_fn::ServerFnError;
use uuid::Uuid;

use crate::domain::{SettingsPatch, SettingsView, Theme, ThemeInput, ThemeSummary};
use crate::error::AppError;

/// Extract the server key from Axum Extension (same pattern as extended_auth).
#[cfg(feature = "ssr")]
async fn get_server_key() -> Vec<u8> {
    use axum::Extension;
    match leptos_axum::extract::<Extension<crate::server::extended_auth::ServerKey>>().await {
        Ok(sk) => sk.0.0,
        Err(_) => Vec::new(),
    }
}

/// Extract the SqlitePool from Axum Extension.
#[cfg(feature = "ssr")]
async fn get_pool() -> Result<sqlx::SqlitePool, AppError> {
    use axum::Extension;
    Ok(leptos_axum::extract::<Extension<sqlx::SqlitePool>>()
        .await
        .map_err(|_| AppError::Internal)?
        .0)
}

/// Require admin session + CSRF for mutating operations.
#[cfg(feature = "ssr")]
async fn require_admin_csrf(
    pool: &sqlx::SqlitePool,
) -> Result<crate::server::auth_queries::SessionInfo, AppError> {
    use crate::domain::Role;
    let info = crate::server::auth_helper::require_session_csrf(pool).await?;
    if info.role != Role::Admin {
        return Err(AppError::Forbidden);
    }
    Ok(info)
}

/// Write an audit event for a settings/theme mutation (best-effort).
#[cfg(feature = "ssr")]
async fn audit_setting(pool: &sqlx::SqlitePool, actor_id: Uuid, action: &str, target: &str) {
    crate::server::auth_queries::audit_write_query(
        pool,
        Some(actor_id),
        action,
        Some(target),
        None,
        None,
        "success",
    )
    .await;
}

/// Get all settings. Admins see full settings including secrets; non-admins get redacted secrets.
#[leptos::server]
pub async fn get_settings() -> Result<SettingsView, ServerFnError<AppError>> {
    #[cfg(feature = "ssr")]
    {
        let pool = get_pool().await?;
        let server_key = get_server_key().await;

        // Determine if caller is admin
        let include_secrets = match crate::server::auth_helper::require_session(&pool).await {
            Ok(info) => info.role == crate::domain::Role::Admin,
            Err(_) => false,
        };

        crate::server::settings_queries::get_settings_view(&pool, &server_key, include_secrets)
            .await
            .map_err(ServerFnError::from)
    }
    #[cfg(not(feature = "ssr"))]
    {
        Err(ServerFnError::from(AppError::Internal))
    }
}

/// Update settings. Admin-gated, CSRF-protected, audited.
#[leptos::server]
pub async fn update_settings(
    patch: SettingsPatch,
) -> Result<SettingsView, ServerFnError<AppError>> {
    #[cfg(feature = "ssr")]
    {
        let pool = get_pool().await?;
        let info = require_admin_csrf(&pool).await?;
        let server_key = get_server_key().await;

        crate::server::settings_queries::apply_settings_patch(&pool, &patch, &server_key).await?;

        audit_setting(&pool, info.user_id, "settings_update", "settings").await;

        // Return full view (caller is admin)
        crate::server::settings_queries::get_settings_view(&pool, &server_key, true)
            .await
            .map_err(ServerFnError::from)
    }
    #[cfg(not(feature = "ssr"))]
    {
        let _ = patch;
        Err(ServerFnError::from(AppError::Unauthorized))
    }
}

/// List all themes (summaries). Public — no auth required.
#[leptos::server]
pub async fn list_themes() -> Result<Vec<ThemeSummary>, ServerFnError<AppError>> {
    #[cfg(feature = "ssr")]
    {
        let pool = get_pool().await?;
        crate::server::settings_queries::list_themes_query(&pool)
            .await
            .map_err(ServerFnError::from)
    }
    #[cfg(not(feature = "ssr"))]
    {
        Err(ServerFnError::from(AppError::Internal))
    }
}

/// Get the active theme (full tokens + CSS). Public — applied during SSR.
#[leptos::server]
pub async fn get_active_theme() -> Result<Option<Theme>, ServerFnError<AppError>> {
    #[cfg(feature = "ssr")]
    {
        let pool = get_pool().await?;
        crate::server::settings_queries::get_active_theme_query(&pool)
            .await
            .map_err(ServerFnError::from)
    }
    #[cfg(not(feature = "ssr"))]
    {
        Err(ServerFnError::from(AppError::Internal))
    }
}

/// Save a new theme. Admin-gated, CSRF-protected, audited.
#[leptos::server]
pub async fn save_theme(input: ThemeInput) -> Result<Theme, ServerFnError<AppError>> {
    #[cfg(feature = "ssr")]
    {
        let pool = get_pool().await?;
        let info = require_admin_csrf(&pool).await?;

        let theme =
            crate::server::settings_queries::save_theme_query(&pool, &input, Some(info.user_id))
                .await?;

        audit_setting(
            &pool,
            info.user_id,
            "theme_save",
            &format!("theme:{}", theme.id),
        )
        .await;

        Ok(theme)
    }
    #[cfg(not(feature = "ssr"))]
    {
        let _ = input;
        Err(ServerFnError::from(AppError::Unauthorized))
    }
}

/// Set the active theme by id. Admin-gated, CSRF-protected, audited.
#[leptos::server]
pub async fn set_active_theme(id: Uuid) -> Result<(), ServerFnError<AppError>> {
    #[cfg(feature = "ssr")]
    {
        let pool = get_pool().await?;
        let info = require_admin_csrf(&pool).await?;
        let server_key = get_server_key().await;

        crate::server::settings_queries::set_active_theme_query(&pool, id, &server_key).await?;

        audit_setting(
            &pool,
            info.user_id,
            "theme_set_active",
            &format!("theme:{}", id),
        )
        .await;

        Ok(())
    }
    #[cfg(not(feature = "ssr"))]
    {
        let _ = id;
        Err(ServerFnError::from(AppError::Unauthorized))
    }
}
