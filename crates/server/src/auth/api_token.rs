//! API token bearer authentication for /api/v1/* routes.
//! Verifies token hash, checks scopes, checks expiry, checks revoked_at, audits every use.

use axum::http::HeaderMap;
use axum::response::Json;
use serde_json::json;
use sqlx::SqlitePool;

use app::server::extended_auth_queries::{self, VerifiedToken};

/// Extract and verify a bearer API token from the Authorization header.
/// Returns the verified token or an error response.
pub async fn verify_bearer(
    headers: &HeaderMap,
    pool: &SqlitePool,
    server_key: &[u8],
) -> Result<VerifiedToken, (axum::http::StatusCode, Json<serde_json::Value>)> {
    let auth_header = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| {
            (
                axum::http::StatusCode::UNAUTHORIZED,
                Json(json!({"error": "missing Authorization header"})),
            )
        })?;

    let token = auth_header.strip_prefix("Bearer ").ok_or_else(|| {
        (
            axum::http::StatusCode::UNAUTHORIZED,
            Json(json!({"error": "invalid Authorization scheme"})),
        )
    })?;

    let verified = extended_auth_queries::verify_api_token(pool, token, server_key)
        .await
        .map_err(|_| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "internal error"})),
            )
        })?;

    let verified = verified.ok_or_else(|| {
        (
            axum::http::StatusCode::UNAUTHORIZED,
            Json(json!({"error": "invalid, revoked, or expired token"})),
        )
    })?;

    // Audit token use (best-effort)
    app::server::auth_queries::audit_write_query(
        pool,
        Some(verified.user_id),
        "api_token_use",
        Some(&verified.id.to_string()),
        None,
        None,
        "success",
    )
    .await;

    Ok(verified)
}

/// Check that the verified token has the required scope.
pub fn require_scope(
    verified: &VerifiedToken,
    scope: &str,
) -> Result<(), (axum::http::StatusCode, Json<serde_json::Value>)> {
    if verified.scopes.iter().any(|s| s == scope) {
        Ok(())
    } else {
        Err((
            axum::http::StatusCode::FORBIDDEN,
            Json(json!({"error": format!("token lacks required scope: {scope}")})),
        ))
    }
}
