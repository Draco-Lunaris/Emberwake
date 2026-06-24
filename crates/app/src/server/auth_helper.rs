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

/// Validate Origin or Referer header against the server's expected origin.
/// Fail-closed: if both Origin and Referer are missing, reject with Forbidden.
/// If Origin is present, it must match. If Origin is missing but Referer is present,
/// Referer must match. This prevents cross-origin mutation attacks.
pub fn validate_origin(headers: &axum::http::HeaderMap) -> Result<(), AppError> {
    // Extract the expected origin from the Host header.
    let host = headers
        .get("host")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    // Check Origin header first (preferred per spec).
    if let Some(origin) = headers.get("origin").and_then(|v| v.to_str().ok()) {
        // Origin must contain the host. Accept http and https schemes.
        let origin_matches = origin.contains(host) || host.is_empty();
        if !origin_matches {
            tracing::warn!("CSRF: Origin mismatch: origin={origin}, host={host}");
            return Err(AppError::Forbidden);
        }
        return Ok(());
    }

    // Fall back to Referer header if Origin is missing.
    if let Some(referer) = headers.get("referer").and_then(|v| v.to_str().ok()) {
        let referer_matches = referer.contains(host) || host.is_empty();
        if !referer_matches {
            tracing::warn!("CSRF: Referer mismatch: referer={referer}, host={host}");
            return Err(AppError::Forbidden);
        }
        return Ok(());
    }

    // Both missing → fail-closed.
    tracing::warn!("CSRF: Both Origin and Referer missing — rejecting (fail-closed)");
    Err(AppError::Forbidden)
}

/// Extract session AND validate CSRF token for mutating operations.
/// Returns the SessionInfo if both session and CSRF are valid.
///
/// CSRF protection has three layers:
/// 1. Origin/Referer header validation (fail-closed if both missing)
/// 2. Per-session CSRF token comparison (cookie/header vs session)
/// 3. Cookie SameSite=Lax (not Strict — Strict breaks login redirect flow)
///
/// SameSite=Lax is used instead of Strict because the login POST redirect
/// would not send the session cookie under SameSite=Strict, breaking the
/// login flow. Lax still prevents cross-origin POST mutations while allowing
/// top-level navigations.
pub async fn require_session_csrf(pool: &SqlitePool) -> Result<SessionInfo, AppError> {
    use axum::http::HeaderMap;

    let info = require_session(pool).await?;

    let headers = leptos_axum::extract::<HeaderMap>()
        .await
        .map_err(|_| AppError::Internal)?;

    // Layer 1: Origin/Referer validation (fail-closed).
    validate_origin(&headers)?;

    // Layer 2: Per-session CSRF token validation.
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
