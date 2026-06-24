//! T034: CSRF test — a forged cross-origin mutation is rejected.
//! Tests CSRF token validation directly, plus session-integrated CSRF enforcement
//! that simulates the server function dispatch path (require_session_csrf).

#[path = "../common/mod.rs"]
mod common;

use app::error::AppError;
use app::server::auth_queries;

#[test]
fn csrf_valid_token_accepted() {
    let token = "valid_csrf_token_abc123";
    let result = auth_queries::validate_csrf(token, token);
    assert!(result.is_ok(), "valid CSRF token should be accepted");
}

#[test]
fn csrf_missing_token_rejected() {
    let result = auth_queries::validate_csrf("", "valid_csrf_token_abc123");
    assert!(
        matches!(result, Err(AppError::Forbidden)),
        "missing CSRF token should be rejected with Forbidden"
    );
}

#[test]
fn csrf_invalid_token_rejected() {
    let result = auth_queries::validate_csrf("forged_token", "valid_csrf_token_abc123");
    assert!(
        matches!(result, Err(AppError::Forbidden)),
        "invalid CSRF token should be rejected with Forbidden"
    );
}

#[test]
fn csrf_empty_expected_rejected() {
    // If session has empty csrf_token (shouldn't happen normally), any non-empty provided token fails
    let result = auth_queries::validate_csrf("some_token", "");
    assert!(
        matches!(result, Err(AppError::Forbidden)),
        "mismatched tokens should be rejected"
    );
}

/// T034: Session-integrated CSRF enforcement — simulates the server function dispatch path.
/// Creates a real session, then validates CSRF as require_session_csrf does:
/// session lookup → CSRF token comparison. A mutation without the correct CSRF
/// token must be rejected with Forbidden.
#[test]
fn csrf_session_integrated_missing_token_rejected() {
    let rt = tokio::runtime::Runtime::new().expect("runtime");
    rt.block_on(async {
        let pool = common::test_pool().await;

        // Setup admin user
        auth_queries::complete_setup_query(&pool, "admin", "password123", None, 32 * 1024, 3, 1)
            .await
            .expect("setup");

        let row: (String,) = sqlx::query_as("SELECT id FROM users WHERE username = 'admin'")
            .fetch_one(&pool)
            .await
            .expect("get admin id");
        let user_id = row.0;

        // Create a real session (as login would)
        let (token, csrf_token) = auth_queries::create_session(&pool, &user_id, None, None)
            .await
            .expect("create session");

        // Lookup session (as require_session does)
        let info = auth_queries::lookup_session(&pool, &token)
            .await
            .expect("lookup")
            .expect("session should exist");

        // Simulate server function dispatch without CSRF token → Forbidden
        let result = auth_queries::validate_csrf("", &info.csrf_token);
        assert!(
            matches!(result, Err(AppError::Forbidden)),
            "missing CSRF token should be rejected with Forbidden (simulates server fn dispatch)"
        );

        // Simulate server function dispatch with wrong CSRF token → Forbidden
        let result = auth_queries::validate_csrf("wrong_token", &info.csrf_token);
        assert!(
            matches!(result, Err(AppError::Forbidden)),
            "wrong CSRF token should be rejected with Forbidden"
        );

        // Simulate server function dispatch with correct CSRF token → Ok
        let result = auth_queries::validate_csrf(&info.csrf_token, &info.csrf_token);
        assert!(result.is_ok(), "correct CSRF token should be accepted");

        // Verify the csrf_token from session matches what we stored
        assert_eq!(
            info.csrf_token, csrf_token,
            "session CSRF token should match"
        );
    });
}

/// T037: Origin/Referer validation for CSRF prevention.
/// Tests validate_origin directly — the function used by require_session_csrf.
use axum::http::HeaderMap;

#[test]
fn origin_validation_correct_origin_accepted() {
    let mut headers = HeaderMap::new();
    headers.insert("host", "localhost:5005".parse().unwrap());
    headers.insert("origin", "http://localhost:5005".parse().unwrap());
    let result = app::server::auth_helper::validate_origin(&headers);
    assert!(result.is_ok(), "same-origin request should be accepted");
}

#[test]
fn origin_validation_wrong_origin_rejected() {
    let mut headers = HeaderMap::new();
    headers.insert("host", "localhost:5005".parse().unwrap());
    headers.insert("origin", "http://evil.example.com".parse().unwrap());
    let result = app::server::auth_helper::validate_origin(&headers);
    assert!(
        matches!(result, Err(app::error::AppError::Forbidden)),
        "cross-origin request should be rejected with Forbidden"
    );
}

#[test]
fn origin_validation_both_missing_rejected() {
    let mut headers = HeaderMap::new();
    headers.insert("host", "localhost:5005".parse().unwrap());
    let result = app::server::auth_helper::validate_origin(&headers);
    assert!(
        matches!(result, Err(app::error::AppError::Forbidden)),
        "both Origin and Referer missing should be rejected (fail-closed)"
    );
}

#[test]
fn origin_validation_referer_fallback_accepted() {
    let mut headers = HeaderMap::new();
    headers.insert("host", "localhost:5005".parse().unwrap());
    headers.insert("referer", "http://localhost:5005/settings".parse().unwrap());
    let result = app::server::auth_helper::validate_origin(&headers);
    assert!(
        result.is_ok(),
        "Referer fallback with matching host should be accepted"
    );
}

#[test]
fn origin_validation_referer_fallback_wrong_host_rejected() {
    let mut headers = HeaderMap::new();
    headers.insert("host", "localhost:5005".parse().unwrap());
    headers.insert("referer", "http://evil.example.com/attack".parse().unwrap());
    let result = app::server::auth_helper::validate_origin(&headers);
    assert!(
        matches!(result, Err(app::error::AppError::Forbidden)),
        "Referer with wrong host should be rejected"
    );
}

#[test]
fn origin_validation_origin_takes_precedence_over_referer() {
    let mut headers = HeaderMap::new();
    headers.insert("host", "localhost:5005".parse().unwrap());
    headers.insert("origin", "http://localhost:5005".parse().unwrap());
    headers.insert("referer", "http://evil.example.com".parse().unwrap());
    let result = app::server::auth_helper::validate_origin(&headers);
    assert!(
        result.is_ok(),
        "correct Origin should take precedence over wrong Referer"
    );
}
