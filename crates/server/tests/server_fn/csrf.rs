//! T034: CSRF test — a forged cross-origin mutation is rejected.
//! Tests CSRF token validation directly.

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
