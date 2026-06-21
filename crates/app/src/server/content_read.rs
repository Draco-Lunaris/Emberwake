//! Server functions for content reads.
//! Public calls return only public rows; authenticated calls return public + private.

use leptos::server_fn::ServerFnError;
use uuid::Uuid;

use crate::domain::{
    Bookmark, CategoryWithItems, DashboardView, Service, ServiceFilter, VisibilityFilter,
};
use crate::error::AppError;

/// Determine visibility filter based on whether the caller has a valid session.
#[cfg(feature = "ssr")]
async fn visibility_for_caller(pool: &sqlx::SqlitePool) -> VisibilityFilter {
    match crate::server::auth_helper::require_session(pool).await {
        Ok(_) => VisibilityFilter::All,
        Err(_) => VisibilityFilter::PublicOnly,
    }
}

#[leptos::server]
pub async fn list_dashboard() -> Result<DashboardView, ServerFnError<AppError>> {
    #[cfg(feature = "ssr")]
    {
        use axum::Extension;
        let pool = leptos_axum::extract::<Extension<sqlx::SqlitePool>>()
            .await
            .map_err(|_| AppError::Internal)?
            .0;
        let filter = visibility_for_caller(&pool).await;
        crate::server::content_queries::list_dashboard_query(&pool, filter)
            .await
            .map_err(AppError::from)
            .map_err(ServerFnError::from)
    }
    #[cfg(not(feature = "ssr"))]
    {
        Err(ServerFnError::from(AppError::Internal))
    }
}

#[leptos::server]
pub async fn list_categories() -> Result<Vec<CategoryWithItems>, ServerFnError<AppError>> {
    #[cfg(feature = "ssr")]
    {
        use axum::Extension;
        let pool = leptos_axum::extract::<Extension<sqlx::SqlitePool>>()
            .await
            .map_err(|_| AppError::Internal)?
            .0;
        let filter = visibility_for_caller(&pool).await;
        crate::server::content_queries::list_categories_query(&pool, filter)
            .await
            .map_err(AppError::from)
            .map_err(ServerFnError::from)
    }
    #[cfg(not(feature = "ssr"))]
    {
        Err(ServerFnError::from(AppError::Internal))
    }
}

#[leptos::server]
pub async fn list_services(filter: ServiceFilter) -> Result<Vec<Service>, ServerFnError<AppError>> {
    #[cfg(feature = "ssr")]
    {
        use axum::Extension;
        let pool = leptos_axum::extract::<Extension<sqlx::SqlitePool>>()
            .await
            .map_err(|_| AppError::Internal)?
            .0;
        let vis_filter = visibility_for_caller(&pool).await;
        crate::server::content_queries::list_services_query(&pool, filter.category_id, vis_filter)
            .await
            .map_err(AppError::from)
            .map_err(ServerFnError::from)
    }
    #[cfg(not(feature = "ssr"))]
    {
        let _ = filter;
        Err(ServerFnError::from(AppError::Internal))
    }
}

#[leptos::server]
pub async fn list_bookmarks(
    category: Option<Uuid>,
) -> Result<Vec<Bookmark>, ServerFnError<AppError>> {
    #[cfg(feature = "ssr")]
    {
        use axum::Extension;
        let pool = leptos_axum::extract::<Extension<sqlx::SqlitePool>>()
            .await
            .map_err(|_| AppError::Internal)?
            .0;
        let vis_filter = visibility_for_caller(&pool).await;
        crate::server::content_queries::list_bookmarks_query(&pool, category, vis_filter)
            .await
            .map_err(AppError::from)
            .map_err(ServerFnError::from)
    }
    #[cfg(not(feature = "ssr"))]
    {
        let _ = category;
        Err(ServerFnError::from(AppError::Internal))
    }
}
