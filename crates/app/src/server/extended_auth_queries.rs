//! Extended auth query functions (US4): OIDC external identities, WebAuthn passkeys,
//! and scoped API tokens. All functions are ssr-only.
//! Uses parameterized SQL with static string literals.
//! Security-critical: API token secrets are never stored — only HMAC-SHA256 hashes.

#![cfg(feature = "ssr")]

use std::str::FromStr;

use base64::Engine;
use chrono::Utc;
use hmac::{Hmac, Mac};
use sha2::Sha256;
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

use crate::domain::{
    ApiTokenInput, ApiTokenSecret, ApiTokenSummary, ExternalIdentity, VALID_SCOPES,
};
use crate::error::AppError;

type HmacSha256 = Hmac<Sha256>;

// --- External Identity (OIDC) ---

/// Create an external identity linked to a user. New identities are unapproved
/// (admin-approve policy: requires admin approval before active session).
pub async fn create_external_identity(
    pool: &SqlitePool,
    user_id: &str,
    provider: &str,
    subject: &str,
) -> Result<ExternalIdentity, AppError> {
    let id = Uuid::now_v7().to_string();
    let now = Utc::now().to_rfc3339();

    sqlx::query(
        "INSERT INTO external_identity (id, user_id, provider, subject, created_at, approved) \
         VALUES (?, ?, ?, ?, ?, 0)",
    )
    .bind(&id)
    .bind(user_id)
    .bind(provider)
    .bind(subject)
    .bind(&now)
    .execute(pool)
    .await?;

    Ok(ExternalIdentity {
        id: Uuid::from_str(&id).unwrap_or_default(),
        user_id: Uuid::from_str(user_id).unwrap_or_default(),
        provider: provider.to_string(),
        subject: subject.to_string(),
        created_at: now,
    })
}

/// Check if an external identity is approved (admin-approve policy).
pub async fn is_external_identity_approved(
    pool: &SqlitePool,
    provider: &str,
    subject: &str,
) -> Result<bool, AppError> {
    let row: Option<(i64,)> =
        sqlx::query_as("SELECT approved FROM external_identity WHERE provider = ? AND subject = ?")
            .bind(provider)
            .bind(subject)
            .fetch_optional(pool)
            .await?;
    Ok(row.map(|(a,)| a != 0).unwrap_or(false))
}

/// Approve an external identity (admin action).
pub async fn approve_external_identity(pool: &SqlitePool, id: Uuid) -> Result<(), AppError> {
    let result = sqlx::query("UPDATE external_identity SET approved = 1 WHERE id = ?")
        .bind(id.to_string())
        .execute(pool)
        .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }
    Ok(())
}

/// Find an external identity by provider + subject.
pub async fn find_external_identity(
    pool: &SqlitePool,
    provider: &str,
    subject: &str,
) -> Result<Option<ExternalIdentity>, AppError> {
    let row = sqlx::query(
        "SELECT id, user_id, provider, subject, created_at \
         FROM external_identity WHERE provider = ? AND subject = ?",
    )
    .bind(provider)
    .bind(subject)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|r| ExternalIdentity {
        id: Uuid::from_str(r.get::<String, _>("id").as_str()).unwrap_or_default(),
        user_id: Uuid::from_str(r.get::<String, _>("user_id").as_str()).unwrap_or_default(),
        provider: r.get("provider"),
        subject: r.get("subject"),
        created_at: r.get("created_at"),
    }))
}

/// List external identities for a user.
pub async fn list_external_identities_query(
    pool: &SqlitePool,
    user_id: &str,
) -> Result<Vec<ExternalIdentity>, AppError> {
    let rows = sqlx::query(
        "SELECT id, user_id, provider, subject, created_at \
         FROM external_identity WHERE user_id = ? ORDER BY created_at ASC",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .iter()
        .map(|r| ExternalIdentity {
            id: Uuid::from_str(r.get::<String, _>("id").as_str()).unwrap_or_default(),
            user_id: Uuid::from_str(r.get::<String, _>("user_id").as_str()).unwrap_or_default(),
            provider: r.get("provider"),
            subject: r.get("subject"),
            created_at: r.get("created_at"),
        })
        .collect())
}

/// Unlink (delete) an external identity. Ownership is enforced.
pub async fn unlink_external_identity_query(
    pool: &SqlitePool,
    id: Uuid,
    user_id: &str,
) -> Result<(), AppError> {
    let result = sqlx::query("DELETE FROM external_identity WHERE id = ? AND user_id = ?")
        .bind(id.to_string())
        .bind(user_id)
        .execute(pool)
        .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }
    Ok(())
}

// --- Passkey Credential (WebAuthn) ---

/// A stored passkey credential record.
pub struct PasskeyRecord {
    pub id: String,
    pub user_id: String,
    pub credential_id: Vec<u8>,
    pub public_key: Vec<u8>,
    pub sign_count: i64,
}

/// Store a passkey credential for a user.
pub async fn store_passkey(
    pool: &SqlitePool,
    user_id: &str,
    credential_id: &[u8],
    public_key: &[u8],
    sign_count: i64,
) -> Result<(), AppError> {
    let id = Uuid::now_v7().to_string();
    let now = Utc::now().to_rfc3339();

    sqlx::query(
        "INSERT INTO passkey_credential (id, user_id, credential_id, public_key, sign_count, created_at) \
         VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(user_id)
    .bind(credential_id)
    .bind(public_key)
    .bind(sign_count)
    .bind(&now)
    .execute(pool)
    .await?;

    Ok(())
}

/// Find a passkey by credential_id.
pub async fn find_passkey(
    pool: &SqlitePool,
    credential_id: &[u8],
) -> Result<Option<PasskeyRecord>, AppError> {
    let row = sqlx::query(
        "SELECT id, user_id, credential_id, public_key, sign_count \
         FROM passkey_credential WHERE credential_id = ?",
    )
    .bind(credential_id)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|r| PasskeyRecord {
        id: r.get("id"),
        user_id: r.get("user_id"),
        credential_id: r.get("credential_id"),
        public_key: r.get("public_key"),
        sign_count: r.get("sign_count"),
    }))
}

/// Update the sign count for a passkey (clone detection).
pub async fn update_passkey_sign_count(
    pool: &SqlitePool,
    credential_id: &[u8],
    sign_count: i64,
) -> Result<(), AppError> {
    sqlx::query("UPDATE passkey_credential SET sign_count = ? WHERE credential_id = ?")
        .bind(sign_count)
        .bind(credential_id)
        .execute(pool)
        .await?;
    Ok(())
}

/// List all passkeys for a user.
pub async fn list_passkeys_for_user(
    pool: &SqlitePool,
    user_id: &str,
) -> Result<Vec<PasskeyRecord>, AppError> {
    let rows = sqlx::query(
        "SELECT id, user_id, credential_id, public_key, sign_count \
         FROM passkey_credential WHERE user_id = ? ORDER BY created_at ASC",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .iter()
        .map(|r| PasskeyRecord {
            id: r.get("id"),
            user_id: r.get("user_id"),
            credential_id: r.get("credential_id"),
            public_key: r.get("public_key"),
            sign_count: r.get("sign_count"),
        })
        .collect())
}

/// Delete a passkey. Ownership is enforced.
pub async fn delete_passkey(pool: &SqlitePool, id: &str, user_id: &str) -> Result<(), AppError> {
    let result = sqlx::query("DELETE FROM passkey_credential WHERE id = ? AND user_id = ?")
        .bind(id)
        .bind(user_id)
        .execute(pool)
        .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }
    Ok(())
}

// --- API Token (scoped automation credential) ---

/// A verified API token with its scopes.
pub struct VerifiedToken {
    pub id: Uuid,
    pub user_id: Uuid,
    pub scopes: Vec<String>,
}

/// Generate a random API token secret: ew_<base64(32 random bytes)>.
fn generate_token_secret() -> String {
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes).expect("getrandom failed");
    format!(
        "ew_{}",
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
    )
}

/// Compute HMAC-SHA256(server_key, token) as hex string.
fn hash_token(token: &str, server_key: &[u8]) -> Result<String, AppError> {
    let mut mac = HmacSha256::new_from_slice(server_key).map_err(|_| AppError::Internal)?;
    mac.update(token.as_bytes());
    Ok(hex_encode(&mac.finalize().into_bytes()))
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Validate that all scopes are in the valid set.
fn validate_scopes(scopes: &[String]) -> Result<(), AppError> {
    for scope in scopes {
        if !VALID_SCOPES.contains(&scope.as_str()) {
            return Err(AppError::Validation(format!("invalid scope: {scope}")));
        }
    }
    Ok(())
}

/// Create an API token. Returns the secret (shown once) + metadata.
/// Only the hash is stored.
pub async fn create_api_token_query(
    pool: &SqlitePool,
    user_id: &str,
    input: &ApiTokenInput,
    server_key: &[u8],
) -> Result<ApiTokenSecret, AppError> {
    if input.name.trim().is_empty() {
        return Err(AppError::Validation("token name must not be empty".into()));
    }
    validate_scopes(&input.scopes)?;

    let id = Uuid::now_v7();
    let secret = generate_token_secret();
    let token_hash = hash_token(&secret, server_key)?;
    let scopes_json = serde_json::to_string(&input.scopes).map_err(|_| AppError::Internal)?;
    let now = Utc::now().to_rfc3339();

    sqlx::query(
        "INSERT INTO api_token (id, user_id, name, token_hash, scopes, expires_at, revoked_at, created_at, last_used_at) \
         VALUES (?, ?, ?, ?, ?, ?, NULL, ?, NULL)",
    )
    .bind(id.to_string())
    .bind(user_id)
    .bind(&input.name)
    .bind(&token_hash)
    .bind(&scopes_json)
    .bind(&input.expires_at)
    .bind(&now)
    .execute(pool)
    .await?;

    Ok(ApiTokenSecret {
        id,
        secret,
        name: input.name.clone(),
        scopes: input.scopes.clone(),
        expires_at: input.expires_at.clone(),
    })
}

/// Verify an API token secret. Returns None if invalid, revoked, or expired.
pub async fn verify_api_token(
    pool: &SqlitePool,
    secret: &str,
    server_key: &[u8],
) -> Result<Option<VerifiedToken>, AppError> {
    let computed_hash = hash_token(secret, server_key)?;

    let row = sqlx::query(
        "SELECT id, user_id, scopes, expires_at, revoked_at \
         FROM api_token WHERE token_hash = ?",
    )
    .bind(&computed_hash)
    .fetch_optional(pool)
    .await?;

    let row = match row {
        Some(r) => r,
        None => return Ok(None),
    };

    // Check revoked
    let revoked_at: Option<String> = row.get("revoked_at");
    if revoked_at.is_some() {
        return Ok(None);
    }

    // Check expiry
    let expires_at: Option<String> = row.get("expires_at");
    if let Some(ref exp) = expires_at {
        let now = Utc::now().to_rfc3339();
        if now > *exp {
            return Ok(None);
        }
    }

    let id_str: String = row.get("id");
    let user_id_str: String = row.get("user_id");
    let scopes_json: String = row.get("scopes");
    let scopes: Vec<String> = serde_json::from_str(&scopes_json).unwrap_or_default();

    // Update last_used_at (best-effort)
    let now = Utc::now().to_rfc3339();
    let token_id = id_str.clone();
    let _ = sqlx::query("UPDATE api_token SET last_used_at = ? WHERE id = ?")
        .bind(&now)
        .bind(&token_id)
        .execute(pool)
        .await;

    Ok(Some(VerifiedToken {
        id: Uuid::from_str(&id_str).unwrap_or_default(),
        user_id: Uuid::from_str(&user_id_str).unwrap_or_default(),
        scopes,
    }))
}

/// Revoke an API token. Ownership is enforced (user can revoke own; admin can revoke any).
pub async fn revoke_api_token_query(
    pool: &SqlitePool,
    id: Uuid,
    user_id: &str,
) -> Result<(), AppError> {
    let now = Utc::now().to_rfc3339();
    let result = sqlx::query(
        "UPDATE api_token SET revoked_at = ? WHERE id = ? AND user_id = ? AND revoked_at IS NULL",
    )
    .bind(&now)
    .bind(id.to_string())
    .bind(user_id)
    .execute(pool)
    .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }
    Ok(())
}

/// List API tokens for a user (summary only — no secrets).
pub async fn list_api_tokens_query(
    pool: &SqlitePool,
    user_id: &str,
) -> Result<Vec<ApiTokenSummary>, AppError> {
    let rows = sqlx::query(
        "SELECT id, name, scopes, expires_at, revoked_at, created_at, last_used_at \
         FROM api_token WHERE user_id = ? ORDER BY created_at DESC",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .iter()
        .map(|r| {
            let scopes_json: String = r.get("scopes");
            let scopes: Vec<String> = serde_json::from_str(&scopes_json).unwrap_or_default();
            ApiTokenSummary {
                id: Uuid::from_str(r.get::<String, _>("id").as_str()).unwrap_or_default(),
                name: r.get("name"),
                scopes,
                expires_at: r.get("expires_at"),
                revoked_at: r.get("revoked_at"),
                created_at: r.get("created_at"),
                last_used_at: r.get("last_used_at"),
            }
        })
        .collect())
}
