//! Server functions for monitoring reads.
//! Public calls return only public-service statuses; authenticated calls return all.

use leptos::server_fn::ServerFnError;
use uuid::Uuid;

#[cfg(feature = "ssr")]
use crate::domain::VisibilityFilter;
use crate::domain::{StatusReading, UptimeSummary};
use crate::error::AppError;

/// Determine visibility filter based on whether the caller has a valid session.
#[cfg(feature = "ssr")]
async fn visibility_for_caller(pool: &sqlx::SqlitePool) -> VisibilityFilter {
    match crate::server::auth_helper::require_session(pool).await {
        Ok(_) => VisibilityFilter::All,
        Err(_) => VisibilityFilter::PublicOnly,
    }
}

/// Get current service statuses for all monitored services.
/// Public callers see only public-service statuses; authenticated callers see all.
#[leptos::server]
pub async fn get_service_statuses() -> Result<Vec<StatusReading>, ServerFnError<AppError>> {
    #[cfg(feature = "ssr")]
    {
        use axum::Extension;
        let pool = leptos_axum::extract::<Extension<sqlx::SqlitePool>>()
            .await
            .map_err(|_| AppError::Internal)?
            .0;
        let filter = visibility_for_caller(&pool).await;
        crate::server::monitor_queries::list_status_readings(&pool, filter)
            .await
            .map_err(AppError::from)
            .map_err(ServerFnError::from)
    }
    #[cfg(not(feature = "ssr"))]
    {
        Err(ServerFnError::from(AppError::Internal))
    }
}

/// Get uptime summary for a specific service over a time window.
/// Public callers can only query public services; authenticated callers can query all.
#[leptos::server]
pub async fn get_uptime_summary(
    service: Uuid,
    window_hours: u32,
) -> Result<UptimeSummary, ServerFnError<AppError>> {
    #[cfg(feature = "ssr")]
    {
        use axum::Extension;
        let pool = leptos_axum::extract::<Extension<sqlx::SqlitePool>>()
            .await
            .map_err(|_| AppError::Internal)?
            .0;

        // Verify the service is visible to this caller.
        let filter = visibility_for_caller(&pool).await;
        if filter == VisibilityFilter::PublicOnly {
            let row: Option<(String,)> =
                sqlx::query_as("SELECT visibility FROM service WHERE id = ?")
                    .bind(service.to_string())
                    .fetch_optional(&pool)
                    .await
                    .map_err(AppError::from)?;
            match row {
                Some((vis,)) if vis == "public" => {}
                _ => return Err(ServerFnError::from(AppError::NotFound)),
            }
        }

        crate::server::monitor_queries::compute_uptime_summary(&pool, service, window_hours)
            .await
            .map_err(ServerFnError::from)
    }
    #[cfg(not(feature = "ssr"))]
    {
        let _ = service;
        let _ = window_hours;
        Err(ServerFnError::from(AppError::Internal))
    }
}
