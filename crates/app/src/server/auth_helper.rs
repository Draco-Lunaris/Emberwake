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

    let cookie_header = headers.get("cookie").and_then(|v| v.to_str().ok());
    let csrf_from_cookie = auth_queries::parse_csrf_cookie(cookie_header);
    let csrf_from_header = headers
        .get("x-csrf-token")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();
    let csrf_provided = if !csrf_from_cookie.is_empty() {
        csrf_from_cookie
    } else {
        csrf_from_header
    };

    auth_queries::validate_csrf(&csrf_provided, &info.csrf_token)?;

    Ok(info)
}

/// Extract session, validate CSRF, AND require admin role.
/// Returns the SessionInfo if session, CSRF, and admin role are all valid.
pub async fn require_admin_csrf(pool: &SqlitePool) -> Result<SessionInfo, AppError> {
    let info = require_session_csrf(pool).await?;
    if info.role != crate::domain::Role::Admin {
        return Err(AppError::Forbidden);
    }
    Ok(info)
}
