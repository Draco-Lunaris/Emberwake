//! Auth helpers for server functions: session extraction, CSRF validation.
//! These are ssr-only helpers used by #[server] functions in app crate.

#![cfg(feature = "ssr")]

use sqlx::SqlitePool;

use crate::error::AppError;
use crate::server::auth_queries::{self, SessionInfo};

/// Extract and validate the session from the current request.
/// Returns Err(AppError::Unauthorized) if no valid session.
pub async fn require_session(pool: &SqlitePool) -> Result<SessionInfo, AppError> {
    use axum::http::HeaderMap;

    let headers = leptos_axum::extract::<HeaderMap>()
        .await
        .map_err(|_| AppError::Internal)?;

    let cookie_header = headers.get("cookie").and_then(|v| v.to_str().ok());

    let token = auth_queries::parse_session_cookie(cookie_header);

    match token {
        Some(t) => {
            let info = auth_queries::lookup_session(pool, &t).await?;
            match info {
                Some(si) => Ok(si),
                None => Err(AppError::Unauthorized),
            }
        }
        None => Err(AppError::Unauthorized),
    }
}

/// Extract session AND validate CSRF token for mutating operations.
/// Returns the SessionInfo if both session and CSRF are valid.
pub async fn require_session_csrf(pool: &SqlitePool) -> Result<SessionInfo, AppError> {
    use axum::http::HeaderMap;

    let info = require_session(pool).await?;

    let headers = leptos_axum::extract::<HeaderMap>()
        .await
        .map_err(|_| AppError::Internal)?;

    let csrf_header = headers
        .get("x-csrf-token")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    auth_queries::validate_csrf(csrf_header, &info.csrf_token)?;

    Ok(info)
}
