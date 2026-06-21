//! Settings-backed search provider config read.

use leptos::server_fn::ServerFnError;

use crate::domain::SearchProviderConfig;
use crate::error::AppError;

#[leptos::server]
pub async fn get_search_providers() -> Result<SearchProviderConfig, ServerFnError<AppError>> {
    #[cfg(feature = "ssr")]
    {
        use axum::Extension;
        let pool = leptos_axum::extract::<Extension<sqlx::SqlitePool>>()
            .await
            .map_err(|_| AppError::Internal)?
            .0;
        crate::server::content_queries::get_search_providers_query(&pool)
            .await
            .map_err(AppError::from)
            .map_err(ServerFnError::from)
    }
    #[cfg(not(feature = "ssr"))]
    {
        Err(ServerFnError::from(AppError::Internal))
    }
}
