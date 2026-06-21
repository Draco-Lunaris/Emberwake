//! Server functions for weather widget reads (US7).
//! get_weather serves cache only — no synchronous upstream call.

use leptos::server_fn::ServerFnError;

use crate::domain::WeatherReading;
use crate::error::AppError;

/// Get the cached weather reading. Public; serves cache only (no upstream call).
#[leptos::server]
pub async fn get_weather() -> Result<Option<WeatherReading>, ServerFnError<AppError>> {
    #[cfg(feature = "ssr")]
    {
        use axum::Extension;
        let pool = leptos_axum::extract::<Extension<sqlx::SqlitePool>>()
            .await
            .map_err(|_| AppError::Internal)?
            .0;
        crate::server::weather_queries::get_weather_reading(&pool)
            .await
            .map_err(AppError::from)
            .map_err(ServerFnError::from)
    }
    #[cfg(not(feature = "ssr"))]
    {
        Err(ServerFnError::from(AppError::Internal))
    }
}
