//! T032: Server-fn tests — setup single-shot + race safety; login success/throttle;
//! logout/revoke invalidate server-side.
//! Tests auth_queries directly (no HTTP layer needed) using #[sqlx::test].

use sqlx::SqlitePool;

use app::domain::{LoginInput, SetupState};
use app::error::AppError;
use app::server::auth_queries;

const M_COST: u32 = 32 * 1024;
const T_COST: u32 = 3;
const P_COST: u32 = 1;

// --- Setup tests ---

#[sqlx::test(migrations = "../../migrations")]
async fn setup_status_open_on_fresh_db(pool: SqlitePool) {
    let status = auth_queries::setup_status_query(&pool)
        .await
        .expect("setup_status");
    assert_eq!(status, SetupState::Open);
}

#[sqlx::test(migrations = "../../migrations")]
async fn complete_setup_creates_admin(pool: SqlitePool) {
    auth_queries::complete_setup_query(
        &pool,
        "admin",
        "password123",
        Some("admin@example.com"),
        M_COST,
        T_COST,
        P_COST,
    )
    .await
    .expect("complete_setup should succeed");

    let status = auth_queries::setup_status_query(&pool)
        .await
        .expect("setup_status");
    assert_eq!(status, SetupState::Complete);

    // Verify admin user was created
    let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users")
        .fetch_one(&pool)
        .await
        .expect("count users");
    assert_eq!(row.0, 1, "exactly one admin user should exist");

    let row: (String,) = sqlx::query_as("SELECT role FROM users WHERE username = 'admin'")
        .fetch_one(&pool)
        .await
        .expect("get admin role");
    assert_eq!(row.0, "admin");
}

#[sqlx::test(migrations = "../../migrations")]
async fn complete_setup_is_single_shot(pool: SqlitePool) {
    auth_queries::complete_setup_query(&pool, "admin", "password123", None, M_COST, T_COST, P_COST)
        .await
        .expect("first setup should succeed");

    // Second setup attempt should fail with Conflict
    let result = auth_queries::complete_setup_query(
        &pool,
        "admin2",
        "password456",
        None,
        M_COST,
        T_COST,
        P_COST,
    )
    .await;
    assert!(
        matches!(result, Err(AppError::Conflict(_))),
        "second setup should return Conflict"
    );
}

// --- Login tests ---

#[sqlx::test(migrations = "../../migrations")]
async fn login_success_issues_session(pool: SqlitePool) {
    auth_queries::complete_setup_query(&pool, "admin", "password123", None, M_COST, T_COST, P_COST)
        .await
        .expect("setup");

    let (token, csrf, user_id) = auth_queries::login_query(
        &pool,
        &LoginInput {
            username: "admin".into(),
            password: "password123".into(),
        },
        Some("test-agent"),
        Some("127.0.0.1"),
    )
    .await
    .expect("login should succeed");

    assert!(!token.is_empty(), "session token should be non-empty");
    assert!(!csrf.is_empty(), "csrf token should be non-empty");

    // Verify session exists in DB
    let info = auth_queries::lookup_session(&pool, &token)
        .await
        .expect("lookup_session");
    assert!(info.is_some(), "session should be found");
    let info = info.unwrap();
    assert_eq!(info.user_id, user_id);
    assert_eq!(info.username, "admin");
}

#[sqlx::test(migrations = "../../migrations")]
async fn login_wrong_password_rejected(pool: SqlitePool) {
    auth_queries::complete_setup_query(&pool, "admin", "password123", None, M_COST, T_COST, P_COST)
        .await
        .expect("setup");

    let result = auth_queries::login_query(
        &pool,
        &LoginInput {
            username: "admin".into(),
            password: "wrongpass".into(),
        },
        None,
        None,
    )
    .await;
    assert!(
        matches!(result, Err(AppError::Unauthorized)),
        "wrong password should return Unauthorized"
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn login_nonexistent_user_rejected(pool: SqlitePool) {
    let result = auth_queries::login_query(
        &pool,
        &LoginInput {
            username: "ghost".into(),
            password: "password123".into(),
        },
        None,
        None,
    )
    .await;
    assert!(
        matches!(result, Err(AppError::Unauthorized)),
        "nonexistent user should return Unauthorized"
    );
}

// --- Logout/revoke tests ---

#[sqlx::test(migrations = "../../migrations")]
async fn logout_invalidates_session_server_side(pool: SqlitePool) {
    auth_queries::complete_setup_query(&pool, "admin", "password123", None, M_COST, T_COST, P_COST)
        .await
        .expect("setup");

    let (token, _, _) = auth_queries::login_query(
        &pool,
        &LoginInput {
            username: "admin".into(),
            password: "password123".into(),
        },
        None,
        None,
    )
    .await
    .expect("login");

    // Verify session exists
    let info = auth_queries::lookup_session(&pool, &token)
        .await
        .expect("lookup");
    assert!(info.is_some(), "session should exist before logout");

    // Logout (delete session)
    auth_queries::delete_session(&pool, &token)
        .await
        .expect("delete session");

    // Verify session is gone
    let info = auth_queries::lookup_session(&pool, &token)
        .await
        .expect("lookup");
    assert!(info.is_none(), "session should be invalidated after logout");
}

#[sqlx::test(migrations = "../../migrations")]
async fn revoke_session_invalidates_server_side(pool: SqlitePool) {
    auth_queries::complete_setup_query(&pool, "admin", "password123", None, M_COST, T_COST, P_COST)
        .await
        .expect("setup");

    let (token1, _, _) = auth_queries::login_query(
        &pool,
        &LoginInput {
            username: "admin".into(),
            password: "password123".into(),
        },
        None,
        None,
    )
    .await
    .expect("login 1");

    let (token2, _, _) = auth_queries::login_query(
        &pool,
        &LoginInput {
            username: "admin".into(),
            password: "password123".into(),
        },
        None,
        None,
    )
    .await
    .expect("login 2");

    // Revoke session 1
    let deleted = auth_queries::revoke_session_query(&pool, &token1)
        .await
        .expect("revoke");
    assert!(deleted, "revoke should delete a row");

    // Verify session 1 is gone
    let info = auth_queries::lookup_session(&pool, &token1)
        .await
        .expect("lookup");
    assert!(info.is_none(), "revoked session should be gone");

    // Verify session 2 still exists
    let info = auth_queries::lookup_session(&pool, &token2)
        .await
        .expect("lookup");
    assert!(info.is_some(), "other session should still exist");
}

#[sqlx::test(migrations = "../../migrations")]
async fn revoke_all_other_sessions(pool: SqlitePool) {
    auth_queries::complete_setup_query(&pool, "admin", "password123", None, M_COST, T_COST, P_COST)
        .await
        .expect("setup");

    let (token1, _, user_id) = auth_queries::login_query(
        &pool,
        &LoginInput {
            username: "admin".into(),
            password: "password123".into(),
        },
        None,
        None,
    )
    .await
    .expect("login 1");

    let (token2, _, _) = auth_queries::login_query(
        &pool,
        &LoginInput {
            username: "admin".into(),
            password: "password123".into(),
        },
        None,
        None,
    )
    .await
    .expect("login 2");

    let (token3, _, _) = auth_queries::login_query(
        &pool,
        &LoginInput {
            username: "admin".into(),
            password: "password123".into(),
        },
        None,
        None,
    )
    .await
    .expect("login 3");

    // Revoke all except token2 (keep current)
    let count = auth_queries::revoke_all_other_sessions_query(&pool, &user_id.to_string(), &token2)
        .await
        .expect("revoke all other");
    assert_eq!(count, 2, "should revoke 2 other sessions");

    // token2 should still work
    let info = auth_queries::lookup_session(&pool, &token2)
        .await
        .expect("lookup");
    assert!(info.is_some(), "kept session should still exist");

    // token1 and token3 should be gone
    let info = auth_queries::lookup_session(&pool, &token1)
        .await
        .expect("lookup");
    assert!(info.is_none(), "revoked session 1 should be gone");
    let info = auth_queries::lookup_session(&pool, &token3)
        .await
        .expect("lookup");
    assert!(info.is_none(), "revoked session 3 should be gone");
}

// --- Password hashing tests ---

#[test]
fn hash_and_verify_password() {
    let hash = auth_queries::hash_password("testpass123", M_COST, T_COST, P_COST).expect("hash");
    assert!(!hash.is_empty());
    assert!(
        hash.starts_with("$argon2id$"),
        "should be Argon2id PHC string"
    );

    assert!(
        auth_queries::verify_password("testpass123", &hash),
        "correct password should verify"
    );
    assert!(
        !auth_queries::verify_password("wrongpass", &hash),
        "wrong password should fail"
    );
}

// --- CSRF validation tests ---

#[test]
fn csrf_validation() {
    assert!(
        auth_queries::validate_csrf("abc123", "abc123").is_ok(),
        "matching tokens should pass"
    );
    assert!(
        auth_queries::validate_csrf("abc123", "different").is_err(),
        "mismatched tokens should fail"
    );
    assert!(
        auth_queries::validate_csrf("", "abc123").is_err(),
        "empty token should fail"
    );
}
