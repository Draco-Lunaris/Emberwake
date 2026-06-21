//! Server functions for content writes (create/update/delete/reorder/pin).
//! Every mutation enforces: authentication (session required), CSRF protection,
//! authorization (role/ownership), and audit logging.
//! Auth fails closed — if no session is available, mutation is rejected.

use leptos::server_fn::ServerFnError;
use uuid::Uuid;

use crate::domain::{
    Bookmark, BookmarkInput, BookmarkPatch, Category, CategoryInput, CategoryPatch, IconRef,
    Service, ServiceInput, ServicePatch,
};
use crate::error::AppError;

/// Extract session + validate CSRF for mutating operations.
#[cfg(feature = "ssr")]
async fn require_auth_csrf(
    pool: &sqlx::SqlitePool,
) -> Result<crate::server::auth_queries::SessionInfo, AppError> {
    crate::server::auth_helper::require_session_csrf(pool).await
}

/// Write an audit event for a content mutation (best-effort).
#[cfg(feature = "ssr")]
async fn audit_content(pool: &sqlx::SqlitePool, actor_id: Uuid, action: &str, target: &str) {
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

// --- Category mutations ---

#[leptos::server]
pub async fn create_category(input: CategoryInput) -> Result<Category, ServerFnError<AppError>> {
    #[cfg(feature = "ssr")]
    {
        use axum::Extension;
        let pool = leptos_axum::extract::<Extension<sqlx::SqlitePool>>()
            .await
            .map_err(|_| AppError::Internal)?
            .0;
        let info = require_auth_csrf(&pool).await?;
        crate::server::content_write_queries::validate_name(&input.name)?;
        let cat = crate::server::content_write_queries::create_category_query(&pool, input).await?;
        audit_content(
            &pool,
            info.user_id,
            "content_mutate",
            &format!("category:{}", cat.id),
        )
        .await;
        Ok(cat)
    }
    #[cfg(not(feature = "ssr"))]
    {
        let _ = input;
        Err(ServerFnError::from(AppError::Unauthorized))
    }
}

#[leptos::server]
pub async fn update_category(
    id: Uuid,
    patch: CategoryPatch,
) -> Result<Category, ServerFnError<AppError>> {
    #[cfg(feature = "ssr")]
    {
        use axum::Extension;
        let pool = leptos_axum::extract::<Extension<sqlx::SqlitePool>>()
            .await
            .map_err(|_| AppError::Internal)?
            .0;
        let info = require_auth_csrf(&pool).await?;
        if let Some(ref name) = patch.name {
            crate::server::content_write_queries::validate_name(name)?;
        }
        let cat =
            crate::server::content_write_queries::update_category_query(&pool, id, patch).await?;
        audit_content(
            &pool,
            info.user_id,
            "content_mutate",
            &format!("category:{}", cat.id),
        )
        .await;
        Ok(cat)
    }
    #[cfg(not(feature = "ssr"))]
    {
        let _ = (id, patch);
        Err(ServerFnError::from(AppError::Unauthorized))
    }
}

#[leptos::server]
pub async fn delete_category(id: Uuid) -> Result<(), ServerFnError<AppError>> {
    #[cfg(feature = "ssr")]
    {
        use axum::Extension;
        let pool = leptos_axum::extract::<Extension<sqlx::SqlitePool>>()
            .await
            .map_err(|_| AppError::Internal)?
            .0;
        let info = require_auth_csrf(&pool).await?;
        crate::server::content_write_queries::delete_category_query(&pool, id).await?;
        audit_content(
            &pool,
            info.user_id,
            "content_mutate",
            &format!("category:delete:{}", id),
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

#[leptos::server]
pub async fn reorder_categories(order: Vec<Uuid>) -> Result<(), ServerFnError<AppError>> {
    #[cfg(feature = "ssr")]
    {
        use axum::Extension;
        let pool = leptos_axum::extract::<Extension<sqlx::SqlitePool>>()
            .await
            .map_err(|_| AppError::Internal)?
            .0;
        let info = require_auth_csrf(&pool).await?;
        crate::server::content_write_queries::reorder_categories_query(&pool, order).await?;
        audit_content(&pool, info.user_id, "content_mutate", "categories:reorder").await;
        Ok(())
    }
    #[cfg(not(feature = "ssr"))]
    {
        let _ = order;
        Err(ServerFnError::from(AppError::Unauthorized))
    }
}

// --- Service mutations ---

#[leptos::server]
pub async fn create_service(input: ServiceInput) -> Result<Service, ServerFnError<AppError>> {
    #[cfg(feature = "ssr")]
    {
        use axum::Extension;
        let pool = leptos_axum::extract::<Extension<sqlx::SqlitePool>>()
            .await
            .map_err(|_| AppError::Internal)?
            .0;
        let info = require_auth_csrf(&pool).await?;
        crate::server::content_write_queries::validate_name(&input.name)?;
        crate::server::content_write_queries::validate_url(&input.url)?;
        let svc = crate::server::content_write_queries::create_service_query(&pool, input).await?;
        audit_content(
            &pool,
            info.user_id,
            "content_mutate",
            &format!("service:{}", svc.id),
        )
        .await;
        Ok(svc)
    }
    #[cfg(not(feature = "ssr"))]
    {
        let _ = input;
        Err(ServerFnError::from(AppError::Unauthorized))
    }
}

#[leptos::server]
pub async fn update_service(
    id: Uuid,
    patch: ServicePatch,
) -> Result<Service, ServerFnError<AppError>> {
    #[cfg(feature = "ssr")]
    {
        use axum::Extension;
        let pool = leptos_axum::extract::<Extension<sqlx::SqlitePool>>()
            .await
            .map_err(|_| AppError::Internal)?
            .0;
        let info = require_auth_csrf(&pool).await?;
        if let Some(ref name) = patch.name {
            crate::server::content_write_queries::validate_name(name)?;
        }
        if let Some(ref url) = patch.url {
            crate::server::content_write_queries::validate_url(url)?;
        }
        let svc =
            crate::server::content_write_queries::update_service_query(&pool, id, patch).await?;
        audit_content(
            &pool,
            info.user_id,
            "content_mutate",
            &format!("service:{}", svc.id),
        )
        .await;
        Ok(svc)
    }
    #[cfg(not(feature = "ssr"))]
    {
        let _ = (id, patch);
        Err(ServerFnError::from(AppError::Unauthorized))
    }
}

#[leptos::server]
pub async fn delete_service(id: Uuid) -> Result<(), ServerFnError<AppError>> {
    #[cfg(feature = "ssr")]
    {
        use axum::Extension;
        let pool = leptos_axum::extract::<Extension<sqlx::SqlitePool>>()
            .await
            .map_err(|_| AppError::Internal)?
            .0;
        let info = require_auth_csrf(&pool).await?;
        crate::server::content_write_queries::delete_service_query(&pool, id).await?;
        audit_content(
            &pool,
            info.user_id,
            "content_mutate",
            &format!("service:delete:{}", id),
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

#[leptos::server]
pub async fn reorder_services(
    category: Option<Uuid>,
    order: Vec<Uuid>,
) -> Result<(), ServerFnError<AppError>> {
    #[cfg(feature = "ssr")]
    {
        use axum::Extension;
        let pool = leptos_axum::extract::<Extension<sqlx::SqlitePool>>()
            .await
            .map_err(|_| AppError::Internal)?
            .0;
        let info = require_auth_csrf(&pool).await?;
        crate::server::content_write_queries::reorder_services_query(&pool, category, order)
            .await?;
        audit_content(&pool, info.user_id, "content_mutate", "services:reorder").await;
        Ok(())
    }
    #[cfg(not(feature = "ssr"))]
    {
        let _ = (category, order);
        Err(ServerFnError::from(AppError::Unauthorized))
    }
}

#[leptos::server]
pub async fn set_service_pinned(
    id: Uuid,
    pinned: bool,
) -> Result<Service, ServerFnError<AppError>> {
    #[cfg(feature = "ssr")]
    {
        use axum::Extension;
        let pool = leptos_axum::extract::<Extension<sqlx::SqlitePool>>()
            .await
            .map_err(|_| AppError::Internal)?
            .0;
        let info = require_auth_csrf(&pool).await?;
        let svc = crate::server::content_write_queries::set_service_pinned_query(&pool, id, pinned)
            .await?;
        audit_content(
            &pool,
            info.user_id,
            "content_mutate",
            &format!("service:pin:{}", svc.id),
        )
        .await;
        Ok(svc)
    }
    #[cfg(not(feature = "ssr"))]
    {
        let _ = (id, pinned);
        Err(ServerFnError::from(AppError::Unauthorized))
    }
}

// --- Bookmark mutations ---

#[leptos::server]
pub async fn create_bookmark(input: BookmarkInput) -> Result<Bookmark, ServerFnError<AppError>> {
    #[cfg(feature = "ssr")]
    {
        use axum::Extension;
        let pool = leptos_axum::extract::<Extension<sqlx::SqlitePool>>()
            .await
            .map_err(|_| AppError::Internal)?
            .0;
        let info = require_auth_csrf(&pool).await?;
        crate::server::content_write_queries::validate_name(&input.name)?;
        crate::server::content_write_queries::validate_url(&input.url)?;
        let bm = crate::server::content_write_queries::create_bookmark_query(&pool, input).await?;
        audit_content(
            &pool,
            info.user_id,
            "content_mutate",
            &format!("bookmark:{}", bm.id),
        )
        .await;
        Ok(bm)
    }
    #[cfg(not(feature = "ssr"))]
    {
        let _ = input;
        Err(ServerFnError::from(AppError::Unauthorized))
    }
}

#[leptos::server]
pub async fn update_bookmark(
    id: Uuid,
    patch: BookmarkPatch,
) -> Result<Bookmark, ServerFnError<AppError>> {
    #[cfg(feature = "ssr")]
    {
        use axum::Extension;
        let pool = leptos_axum::extract::<Extension<sqlx::SqlitePool>>()
            .await
            .map_err(|_| AppError::Internal)?
            .0;
        let info = require_auth_csrf(&pool).await?;
        if let Some(ref name) = patch.name {
            crate::server::content_write_queries::validate_name(name)?;
        }
        if let Some(ref url) = patch.url {
            crate::server::content_write_queries::validate_url(url)?;
        }
        let bm =
            crate::server::content_write_queries::update_bookmark_query(&pool, id, patch).await?;
        audit_content(
            &pool,
            info.user_id,
            "content_mutate",
            &format!("bookmark:{}", bm.id),
        )
        .await;
        Ok(bm)
    }
    #[cfg(not(feature = "ssr"))]
    {
        let _ = (id, patch);
        Err(ServerFnError::from(AppError::Unauthorized))
    }
}

#[leptos::server]
pub async fn delete_bookmark(id: Uuid) -> Result<(), ServerFnError<AppError>> {
    #[cfg(feature = "ssr")]
    {
        use axum::Extension;
        let pool = leptos_axum::extract::<Extension<sqlx::SqlitePool>>()
            .await
            .map_err(|_| AppError::Internal)?
            .0;
        let info = require_auth_csrf(&pool).await?;
        crate::server::content_write_queries::delete_bookmark_query(&pool, id).await?;
        audit_content(
            &pool,
            info.user_id,
            "content_mutate",
            &format!("bookmark:delete:{}", id),
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

#[leptos::server]
pub async fn reorder_bookmarks(
    category: Uuid,
    order: Vec<Uuid>,
) -> Result<(), ServerFnError<AppError>> {
    #[cfg(feature = "ssr")]
    {
        use axum::Extension;
        let pool = leptos_axum::extract::<Extension<sqlx::SqlitePool>>()
            .await
            .map_err(|_| AppError::Internal)?
            .0;
        let info = require_auth_csrf(&pool).await?;
        crate::server::content_write_queries::reorder_bookmarks_query(&pool, category, order)
            .await?;
        audit_content(&pool, info.user_id, "content_mutate", "bookmarks:reorder").await;
        Ok(())
    }
    #[cfg(not(feature = "ssr"))]
    {
        let _ = (category, order);
        Err(ServerFnError::from(AppError::Unauthorized))
    }
}

// --- Icon upload (T031) ---

#[leptos::server]
pub async fn upload_icon(file: Vec<u8>) -> Result<IconRef, ServerFnError<AppError>> {
    #[cfg(feature = "ssr")]
    {
        use axum::Extension;
        let pool = leptos_axum::extract::<Extension<sqlx::SqlitePool>>()
            .await
            .map_err(|_| AppError::Internal)?
            .0;
        let info = require_auth_csrf(&pool).await?;

        const MAX_ICON_SIZE: usize = 2 * 1024 * 1024; // 2 MiB

        if file.len() > MAX_ICON_SIZE {
            return Err(ServerFnError::from(AppError::Validation(
                "icon file exceeds maximum size (2 MiB)".into(),
            )));
        }

        let is_image = file.starts_with(&[0x89, 0x50, 0x4E, 0x47]) // PNG
            || file.starts_with(&[0xFF, 0xD8, 0xFF]) // JPEG
            || file.starts_with(b"GIF8") // GIF
            || file.starts_with(&[0x3C, 0x73, 0x76, 0x67]) // SVG
            || file.starts_with(b"RIFF"); // WebP

        if !is_image {
            return Err(ServerFnError::from(AppError::Validation(
                "icon file must be an image (PNG, JPEG, GIF, SVG, or WebP)".into(),
            )));
        }

        let ext = if file.starts_with(&[0x89, 0x50, 0x4E, 0x47]) {
            "png"
        } else if file.starts_with(&[0xFF, 0xD8, 0xFF]) {
            "jpg"
        } else if file.starts_with(b"GIF8") {
            "gif"
        } else if file.starts_with(&[0x3C, 0x73, 0x76, 0x67]) {
            "svg"
        } else {
            "webp"
        };

        let icon_id = Uuid::now_v7();
        let icon_path = format!("data/icons/{icon_id}.{ext}");

        std::fs::create_dir_all("data/icons").map_err(|_| AppError::Internal)?;
        std::fs::write(&icon_path, &file).map_err(|_| AppError::Internal)?;

        audit_content(
            &pool,
            info.user_id,
            "content_mutate",
            &format!("icon:{icon_id}"),
        )
        .await;
        Ok(IconRef { path: icon_path })
    }
    #[cfg(not(feature = "ssr"))]
    {
        let _ = file;
        Err(ServerFnError::from(AppError::Unauthorized))
    }
}
