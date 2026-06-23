//! SQL query functions for auth (password hashing, sessions, CSRF, user CRUD).
//! All functions are ssr-only. Uses parameterized SQL with static string literals.
//! Security-critical: never log or return password hashes.

#![cfg(feature = "ssr")]

use std::str::FromStr;

use argon2::password_hash::SaltString;
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use chrono::{Duration, Utc};
use getrandom::fill;
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

use crate::domain::{
    LoginInput, NewUserInput, Role, SessionSummary, SetupState, UserPatch, UserSummary,
};
use crate::error::AppError;

// --- Token generation ---

/// Generate a cryptographically random token (32 bytes = 64 hex chars).
fn random_token() -> String {
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes).expect("getrandom failed");
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

// --- Password hashing (Argon2id) ---

/// Hash a password with Argon2id using config defaults: m=32 MiB, t=3, p=1.
/// Returns a PHC string. Never log or return the hash to clients.
pub fn hash_password(
    password: &str,
    m_cost: u32,
    t_cost: u32,
    p_cost: u32,
) -> Result<String, AppError> {
    let mut salt_bytes = [0u8; 16];
    fill(&mut salt_bytes).map_err(|_| AppError::Internal)?;
    let salt = SaltString::encode_b64(&salt_bytes).map_err(|_| AppError::Internal)?;
    let params =
        argon2::Params::new(m_cost, t_cost, p_cost, None).map_err(|_| AppError::Internal)?;
    let argon2 = Argon2::new(argon2::Algorithm::Argon2id, argon2::Version::V0x13, params);
    let hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|_| AppError::Internal)?;
    Ok(hash.to_string())
}

/// Verify a password against a PHC string. Returns true if match.
pub fn verify_password(password: &str, phc_string: &str) -> bool {
    let parsed = match PasswordHash::new(phc_string) {
        Ok(h) => h,
        Err(_) => return false,
    };
    Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .is_ok()
}

// --- Setup ---

/// Check if initial admin setup has been completed.
pub async fn setup_status_query(pool: &SqlitePool) -> Result<SetupState, AppError> {
    let row: Option<(String,)> =
        sqlx::query_as("SELECT value FROM setting WHERE key = 'setup_complete'")
            .fetch_optional(pool)
            .await?;
    Ok(if row.is_some() {
        SetupState::Complete
    } else {
        SetupState::Open
    })
}

/// Complete first-run setup: create admin user + set setup_complete in one transaction.
/// Race-safe via UNIQUE constraint on setup_complete key.
pub async fn complete_setup_query(
    pool: &SqlitePool,
    username: &str,
    password: &str,
    email: Option<&str>,
    m_cost: u32,
    t_cost: u32,
    p_cost: u32,
) -> Result<(), AppError> {
    if username.trim().is_empty() {
        return Err(AppError::Validation("username must not be empty".into()));
    }
    if username.len() > 64 {
        return Err(AppError::Validation(
            "username must be at most 64 characters".into(),
        ));
    }
    if password.len() < 8 {
        return Err(AppError::Validation(
            "password must be at least 8 characters".into(),
        ));
    }

    let mut tx = pool.begin().await?;

    let existing: Option<(String,)> =
        sqlx::query_as("SELECT value FROM setting WHERE key = 'setup_complete'")
            .fetch_optional(&mut *tx)
            .await?;
    if existing.is_some() {
        return Err(AppError::Conflict("setup already complete".into()));
    }

    let user_id = Uuid::now_v7().to_string();
    let now = Utc::now().to_rfc3339();
    let hash = hash_password(password, m_cost, t_cost, p_cost)?;

    sqlx::query(
        "INSERT INTO users (id, username, email, password_hash, role, is_active, created_at, updated_at) \
         VALUES (?, ?, ?, ?, 'admin', 1, ?, ?)",
    )
    .bind(&user_id)
    .bind(username)
    .bind(email)
    .bind(&hash)
    .bind(&now)
    .bind(&now)
    .execute(&mut *tx)
    .await?;

    sqlx::query("INSERT INTO setting (key, value, updated_at) VALUES ('setup_complete', '1', ?)")
        .bind(&now)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;
    Ok(())
}

// --- Session management ---

const SESSION_IDLE_TIMEOUT_MINUTES: i64 = 30;
const SESSION_ABSOLUTE_TIMEOUT_HOURS: i64 = 24;

/// Create a new session for a user. Returns (session_token, csrf_token).
pub async fn create_session(
    pool: &SqlitePool,
    user_id: &str,
    user_agent: Option<&str>,
    ip: Option<&str>,
) -> Result<(String, String), AppError> {
    let session_token = random_token();
    let csrf_token = random_token();
    let now = Utc::now();
    let expires_at = now + Duration::hours(SESSION_ABSOLUTE_TIMEOUT_HOURS);

    sqlx::query(
        "INSERT INTO sessions (id, user_id, created_at, expires_at, last_used_at, user_agent, ip, csrf_token) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&session_token)
    .bind(user_id)
    .bind(now.to_rfc3339())
    .bind(expires_at.to_rfc3339())
    .bind(now.to_rfc3339())
    .bind(user_agent)
    .bind(ip)
    .bind(&csrf_token)
    .execute(pool)
    .await?;

    Ok((session_token, csrf_token))
}

/// Session info returned from lookup.
pub struct SessionInfo {
    pub session_id: String,
    pub user_id: Uuid,
    pub role: Role,
    pub username: String,
    pub csrf_token: String,
}

/// Look up a session by token. Checks expiry and updates last_used_at.
pub async fn lookup_session(
    pool: &SqlitePool,
    token: &str,
) -> Result<Option<SessionInfo>, AppError> {
    let row: Option<sqlx::sqlite::SqliteRow> = sqlx::query(
        "SELECT s.id, s.user_id, s.csrf_token, s.expires_at, s.last_used_at, \
                u.role, u.username, u.is_active \
         FROM sessions s \
         JOIN users u ON s.user_id = u.id \
         WHERE s.id = ?",
    )
    .bind(token)
    .fetch_optional(pool)
    .await?;

    let row = match row {
        Some(r) => r,
        None => return Ok(None),
    };

    let is_active: i64 = row.get("is_active");
    if is_active == 0 {
        let _ = sqlx::query("DELETE FROM sessions WHERE id = ?")
            .bind(token)
            .execute(pool)
            .await;
        return Ok(None);
    }

    let expires_at: String = row.get("expires_at");
    let now = Utc::now().to_rfc3339();
    if now > expires_at {
        let _ = sqlx::query("DELETE FROM sessions WHERE id = ?")
            .bind(token)
            .execute(pool)
            .await;
        return Ok(None);
    }

    let last_used_at: String = row.get("last_used_at");
    let now_dt = Utc::now();
    let last_used = chrono::DateTime::parse_from_rfc3339(&last_used_at).unwrap_or(now_dt.into());
    if (now_dt - last_used.with_timezone(&Utc)).num_minutes() > SESSION_IDLE_TIMEOUT_MINUTES {
        let _ = sqlx::query("DELETE FROM sessions WHERE id = ?")
            .bind(token)
            .execute(pool)
            .await;
        return Ok(None);
    }

    let _ = sqlx::query("UPDATE sessions SET last_used_at = ? WHERE id = ?")
        .bind(&now)
        .bind(token)
        .execute(pool)
        .await;

    Ok(Some(SessionInfo {
        session_id: row.get("id"),
        user_id: Uuid::from_str(row.get::<String, _>("user_id").as_str()).unwrap_or_default(),
        role: row.get::<String, _>("role").parse().unwrap_or_default(),
        username: row.get("username"),
        csrf_token: row.get("csrf_token"),
    }))
}

/// Delete a session (revoke). Returns true if a row was deleted.
pub async fn revoke_session_query(pool: &SqlitePool, session_id: &str) -> Result<bool, AppError> {
    let result = sqlx::query("DELETE FROM sessions WHERE id = ?")
        .bind(session_id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected() > 0)
}

/// Revoke all sessions for a user except the specified one.
pub async fn revoke_all_other_sessions_query(
    pool: &SqlitePool,
    user_id: &str,
    keep_session_id: &str,
) -> Result<u64, AppError> {
    let result = sqlx::query("DELETE FROM sessions WHERE user_id = ? AND id != ?")
        .bind(user_id)
        .bind(keep_session_id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected())
}

/// Delete a session by token (used on logout).
pub async fn delete_session(pool: &SqlitePool, session_id: &str) -> Result<(), AppError> {
    sqlx::query("DELETE FROM sessions WHERE id = ?")
        .bind(session_id)
        .execute(pool)
        .await?;
    Ok(())
}

/// List sessions for a user.
pub async fn list_sessions_query(
    pool: &SqlitePool,
    user_id: &str,
) -> Result<Vec<SessionSummary>, AppError> {
    let rows = sqlx::query(
        "SELECT id, user_agent, ip, created_at, expires_at, last_used_at \
         FROM sessions WHERE user_id = ? ORDER BY created_at DESC",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .iter()
        .map(|r| SessionSummary {
            id: r.get("id"),
            user_agent: r.get("user_agent"),
            ip: r.get("ip"),
            created_at: r.get("created_at"),
            expires_at: r.get("expires_at"),
            last_used_at: r.get("last_used_at"),
        })
        .collect())
}

// --- Login ---

/// Attempt login. Returns (session_token, csrf_token, user_id) on success.
pub async fn login_query(
    pool: &SqlitePool,
    input: &LoginInput,
    user_agent: Option<&str>,
    ip: Option<&str>,
) -> Result<(String, String, Uuid), AppError> {
    let row: Option<sqlx::sqlite::SqliteRow> = sqlx::query(
        "SELECT id, password_hash, is_active FROM users WHERE username = ? COLLATE NOCASE",
    )
    .bind(&input.username)
    .fetch_optional(pool)
    .await?;

    let row = match row {
        Some(r) => r,
        None => return Err(AppError::Unauthorized),
    };

    let is_active: i64 = row.get("is_active");
    if is_active == 0 {
        return Err(AppError::Unauthorized);
    }

    let user_id_str: String = row.get("id");
    let password_hash: Option<String> = row.get("password_hash");

    let hash = match password_hash {
        Some(h) if !h.is_empty() => h,
        _ => return Err(AppError::Unauthorized),
    };

    if !verify_password(&input.password, &hash) {
        return Err(AppError::Unauthorized);
    }

    let now = Utc::now().to_rfc3339();
    let _ = sqlx::query("UPDATE users SET last_login_at = ? WHERE id = ?")
        .bind(&now)
        .bind(&user_id_str)
        .execute(pool)
        .await;

    let (session_token, csrf_token) = create_session(pool, &user_id_str, user_agent, ip).await?;
    Ok((
        session_token,
        csrf_token,
        Uuid::from_str(&user_id_str).unwrap_or_default(),
    ))
}

// --- User CRUD ---

fn row_to_user_summary(row: &sqlx::sqlite::SqliteRow) -> UserSummary {
    UserSummary {
        id: Uuid::from_str(row.get::<String, _>("id").as_str()).unwrap_or_default(),
        username: row.get("username"),
        email: row.get("email"),
        role: row.get::<String, _>("role").parse().unwrap_or_default(),
        is_active: row.get::<i64, _>("is_active") != 0,
        created_at: row.get("created_at"),
        last_login_at: row.get("last_login_at"),
    }
}

pub async fn list_users_query(pool: &SqlitePool) -> Result<Vec<UserSummary>, AppError> {
    let rows = sqlx::query(
        "SELECT id, username, email, role, is_active, created_at, last_login_at \
         FROM users ORDER BY created_at ASC",
    )
    .fetch_all(pool)
    .await?;
    Ok(rows.iter().map(row_to_user_summary).collect())
}

pub async fn create_user_query(
    pool: &SqlitePool,
    input: &NewUserInput,
    m_cost: u32,
    t_cost: u32,
    p_cost: u32,
) -> Result<UserSummary, AppError> {
    if input.username.trim().is_empty() {
        return Err(AppError::Validation("username must not be empty".into()));
    }
    if input.username.len() > 64 {
        return Err(AppError::Validation(
            "username must be at most 64 characters".into(),
        ));
    }
    if input.password.len() < 8 {
        return Err(AppError::Validation(
            "password must be at least 8 characters".into(),
        ));
    }

    let id = Uuid::now_v7().to_string();
    let now = Utc::now().to_rfc3339();
    let hash = hash_password(&input.password, m_cost, t_cost, p_cost)?;

    sqlx::query(
        "INSERT INTO users (id, username, email, password_hash, role, is_active, created_at, updated_at) \
         VALUES (?, ?, ?, ?, ?, 1, ?, ?)",
    )
    .bind(&id)
    .bind(&input.username)
    .bind(&input.email)
    .bind(&hash)
    .bind(input.role.to_string())
    .bind(&now)
    .bind(&now)
    .execute(pool)
    .await?;

    let row = sqlx::query(
        "SELECT id, username, email, role, is_active, created_at, last_login_at FROM users WHERE id = ?",
    )
    .bind(&id)
    .fetch_one(pool)
    .await?;

    Ok(row_to_user_summary(&row))
}

pub async fn update_user_query(
    pool: &SqlitePool,
    id: Uuid,
    patch: &UserPatch,
    m_cost: u32,
    t_cost: u32,
    p_cost: u32,
) -> Result<UserSummary, AppError> {
    let id_str = id.to_string();
    let now = Utc::now().to_rfc3339();

    if let Some(ref username) = patch.username {
        if username.trim().is_empty() {
            return Err(AppError::Validation("username must not be empty".into()));
        }
        sqlx::query("UPDATE users SET username = ?, updated_at = ? WHERE id = ?")
            .bind(username)
            .bind(&now)
            .bind(&id_str)
            .execute(pool)
            .await?;
    }
    if let Some(ref email) = patch.email {
        match email {
            Some(e) => {
                sqlx::query("UPDATE users SET email = ?, updated_at = ? WHERE id = ?")
                    .bind(e)
                    .bind(&now)
                    .bind(&id_str)
                    .execute(pool)
                    .await?;
            }
            None => {
                sqlx::query("UPDATE users SET email = NULL, updated_at = ? WHERE id = ?")
                    .bind(&now)
                    .bind(&id_str)
                    .execute(pool)
                    .await?;
            }
        }
    }
    if let Some(role) = patch.role {
        sqlx::query("UPDATE users SET role = ?, updated_at = ? WHERE id = ?")
            .bind(role.to_string())
            .bind(&now)
            .bind(&id_str)
            .execute(pool)
            .await?;
    }
    if let Some(is_active) = patch.is_active {
        sqlx::query("UPDATE users SET is_active = ?, updated_at = ? WHERE id = ?")
            .bind(if is_active { 1i64 } else { 0i64 })
            .bind(&now)
            .bind(&id_str)
            .execute(pool)
            .await?;
    }
    if let Some(ref password) = patch.password {
        if password.len() < 8 {
            return Err(AppError::Validation(
                "password must be at least 8 characters".into(),
            ));
        }
        let hash = hash_password(password, m_cost, t_cost, p_cost)?;
        sqlx::query("UPDATE users SET password_hash = ?, updated_at = ? WHERE id = ?")
            .bind(&hash)
            .bind(&now)
            .bind(&id_str)
            .execute(pool)
            .await?;
    }

    let row = sqlx::query(
        "SELECT id, username, email, role, is_active, created_at, last_login_at FROM users WHERE id = ?",
    ).bind(&id_str).fetch_optional(pool).await?;

    match row {
        Some(r) => Ok(row_to_user_summary(&r)),
        None => Err(AppError::NotFound),
    }
}

pub async fn deactivate_user_query(pool: &SqlitePool, id: Uuid) -> Result<(), AppError> {
    let id_str = id.to_string();
    let now = Utc::now().to_rfc3339();

    let result = sqlx::query("UPDATE users SET is_active = 0, updated_at = ? WHERE id = ?")
        .bind(&now)
        .bind(&id_str)
        .execute(pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }

    let _ = sqlx::query("DELETE FROM sessions WHERE user_id = ?")
        .bind(&id_str)
        .execute(pool)
        .await;

    Ok(())
}

pub async fn get_user_by_id(
    pool: &SqlitePool,
    user_id: Uuid,
) -> Result<Option<UserSummary>, AppError> {
    let row = sqlx::query(
        "SELECT id, username, email, role, is_active, created_at, last_login_at FROM users WHERE id = ?",
    )
    .bind(user_id.to_string())
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|r| row_to_user_summary(&r)))
}

// --- CSRF validation ---

pub fn validate_csrf(provided: &str, expected: &str) -> Result<(), AppError> {
    if provided.is_empty() || provided != expected {
        return Err(AppError::Forbidden);
    }
    Ok(())
}

// --- Audit writing (direct via pool, avoids server crate dependency) ---

pub async fn audit_write_query(
    pool: &SqlitePool,
    actor_id: Option<Uuid>,
    action: &str,
    target: Option<&str>,
    ip: Option<&str>,
    user_agent: Option<&str>,
    result: &str,
) {
    let id = Uuid::now_v7().to_string();
    let ts = Utc::now().to_rfc3339();
    let actor = actor_id.map(|u| u.to_string());

    let res = sqlx::query(
        "INSERT INTO audit_event (id, ts, actor_id, action, target, ip, user_agent, result) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(&ts)
    .bind(&actor)
    .bind(action)
    .bind(target)
    .bind(ip)
    .bind(user_agent)
    .bind(result)
    .execute(pool)
    .await;

    if let Err(e) = res {
        tracing::warn!("Failed to write audit event: {e}");
    }
}

// --- Cookie helpers ---

pub const SESSION_COOKIE_NAME: &str = "emberwake_session";
pub const CSRF_COOKIE_NAME: &str = "emberwake_csrf";

pub fn parse_session_cookie(cookie_header: Option<&str>) -> Option<String> {
    let header = cookie_header?;
    for cookie in header.split(';') {
        let cookie = cookie.trim();
        if let Some((name, value)) = cookie.split_once('=')
            && name.trim() == SESSION_COOKIE_NAME
        {
            return Some(value.trim().to_string());
        }
    }
    None
}

pub fn build_session_cookie(token: &str, secure: bool) -> String {
    let mut cookie = format!(
        "{}={}; Path=/; HttpOnly; SameSite=Lax",
        SESSION_COOKIE_NAME, token
    );
    if secure {
        cookie.push_str("; Secure");
    }
    cookie.push_str("; Max-Age=86400");
    cookie
}

pub fn build_clear_session_cookie(secure: bool) -> String {
    let mut cookie = format!(
        "{}=; Path=/; HttpOnly; SameSite=Lax; Max-Age=0",
        SESSION_COOKIE_NAME
    );
    if secure {
        cookie.push_str("; Secure");
    }
    cookie
}

pub fn parse_csrf_cookie(cookie_header: Option<&str>) -> String {
    let header = match cookie_header {
        Some(h) => h,
        None => return String::new(),
    };
    for cookie in header.split(';') {
        let cookie = cookie.trim();
        if let Some((name, value)) = cookie.split_once('=')
            && name.trim() == CSRF_COOKIE_NAME
        {
            return value.trim().to_string();
        }
    }
    String::new()
}

pub fn build_csrf_cookie(token: &str, secure: bool) -> String {
    let mut cookie = format!("{}={}; Path=/; SameSite=Lax", CSRF_COOKIE_NAME, token);
    if secure {
        cookie.push_str("; Secure");
    }
    cookie.push_str("; Max-Age=86400");
    cookie
}

pub fn build_clear_csrf_cookie(secure: bool) -> String {
    let mut cookie = format!("{}=; Path=/; SameSite=Lax; Max-Age=0", CSRF_COOKIE_NAME);
    if secure {
        cookie.push_str("; Secure");
    }
    cookie
}
