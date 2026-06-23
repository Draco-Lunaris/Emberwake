//! T044: API-token test: in-scope success, out-of-scope 403, post-revoke 401.
//! Tests extended_auth_queries directly using #[test] with tokio runtime.

#[path = "../common/mod.rs"]
mod common;

use common::test_pool;
use sqlx::SqlitePool;
use uuid::Uuid;

use app::domain::ApiTokenInput;
use app::error::AppError;
use app::server::auth_queries;
use app::server::extended_auth_queries;

const M_COST: u32 = 32 * 1024;
const T_COST: u32 = 3;
const P_COST: u32 = 1;
const SERVER_KEY: &[u8] = b"test-server-key";

async fn setup_admin(pool: &SqlitePool) -> Uuid {
    auth_queries::complete_setup_query(pool, "admin", "password123", None, M_COST, T_COST, P_COST)
        .await
        .expect("setup");
    let row: (String,) = sqlx::query_as("SELECT id FROM users WHERE username = 'admin'")
        .fetch_one(pool)
        .await
        .expect("get admin id");
    Uuid::parse_str(&row.0).expect("parse uuid")
}

#[test]
fn create_api_token_returns_secret() {
    let rt = tokio::runtime::Runtime::new().expect("runtime");
    rt.block_on(async {
        let pool = test_pool().await;
        let admin_id = setup_admin(&pool).await;
        let input = ApiTokenInput {
            name: "test-token".to_string(),
            scopes: vec!["services:read".to_string(), "export".to_string()],
            expires_at: None,
        };
        let secret = extended_auth_queries::create_api_token_query(
            &pool,
            &admin_id.to_string(),
            &input,
            SERVER_KEY,
        )
        .await
        .expect("create token");
        assert!(!secret.secret.is_empty());
        assert_eq!(secret.name, "test-token");
        assert_eq!(secret.scopes.len(), 2);
    });
}

#[test]
fn verify_api_token_in_scope() {
    let rt = tokio::runtime::Runtime::new().expect("runtime");
    rt.block_on(async {
        let pool = test_pool().await;
        let admin_id = setup_admin(&pool).await;
        let input = ApiTokenInput {
            name: "test-token".to_string(),
            scopes: vec!["services:read".to_string()],
            expires_at: None,
        };
        let secret = extended_auth_queries::create_api_token_query(
            &pool,
            &admin_id.to_string(),
            &input,
            SERVER_KEY,
        )
        .await
        .expect("create token");
        let verified = extended_auth_queries::verify_api_token(&pool, &secret.secret, SERVER_KEY)
            .await
            .expect("verify");
        assert!(verified.is_some());
        assert_eq!(verified.unwrap().scopes, vec!["services:read"]);
    });
}

#[test]
fn verify_api_token_out_of_scope() {
    let rt = tokio::runtime::Runtime::new().expect("runtime");
    rt.block_on(async {
        let pool = test_pool().await;
        let admin_id = setup_admin(&pool).await;
        let input = ApiTokenInput {
            name: "test-token".to_string(),
            scopes: vec!["services:read".to_string()],
            expires_at: None,
        };
        let secret = extended_auth_queries::create_api_token_query(
            &pool,
            &admin_id.to_string(),
            &input,
            SERVER_KEY,
        )
        .await
        .expect("create token");
        let verified = extended_auth_queries::verify_api_token(&pool, &secret.secret, SERVER_KEY)
            .await
            .expect("verify");
        assert!(verified.is_some());
        let v = verified.unwrap();
        assert!(v.scopes.contains(&"services:read".to_string()));
        assert!(!v.scopes.contains(&"services:write".to_string()));
    });
}

#[test]
fn verify_api_token_post_revoke() {
    let rt = tokio::runtime::Runtime::new().expect("runtime");
    rt.block_on(async {
        let pool = test_pool().await;
        let admin_id = setup_admin(&pool).await;
        let input = ApiTokenInput {
            name: "test-token".to_string(),
            scopes: vec!["services:read".to_string()],
            expires_at: None,
        };
        let secret = extended_auth_queries::create_api_token_query(
            &pool,
            &admin_id.to_string(),
            &input,
            SERVER_KEY,
        )
        .await
        .expect("create token");
        let verified = extended_auth_queries::verify_api_token(&pool, &secret.secret, SERVER_KEY)
            .await
            .expect("verify before revoke");
        assert!(verified.is_some());
        extended_auth_queries::revoke_api_token_query(&pool, secret.id, &admin_id.to_string())
            .await
            .expect("revoke");
        let verified = extended_auth_queries::verify_api_token(&pool, &secret.secret, SERVER_KEY)
            .await
            .expect("verify after revoke");
        assert!(verified.is_none(), "revoked token should not verify");
    });
}

#[test]
fn list_api_tokens_returns_own_tokens() {
    let rt = tokio::runtime::Runtime::new().expect("runtime");
    rt.block_on(async {
        let pool = test_pool().await;
        let admin_id = setup_admin(&pool).await;
        for name in ["token1", "token2"] {
            let input = ApiTokenInput {
                name: name.to_string(),
                scopes: vec!["services:read".to_string()],
                expires_at: None,
            };
            extended_auth_queries::create_api_token_query(
                &pool,
                &admin_id.to_string(),
                &input,
                SERVER_KEY,
            )
            .await
            .expect("create token");
        }
        let tokens = extended_auth_queries::list_api_tokens_query(&pool, &admin_id.to_string())
            .await
            .expect("list");
        assert_eq!(tokens.len(), 2);
    });
}

#[test]
fn invalid_scope_rejected() {
    let rt = tokio::runtime::Runtime::new().expect("runtime");
    rt.block_on(async {
        let pool = test_pool().await;
        let admin_id = setup_admin(&pool).await;
        let input = ApiTokenInput {
            name: "bad".to_string(),
            scopes: vec!["invalid:scope".to_string()],
            expires_at: None,
        };
        let result = extended_auth_queries::create_api_token_query(
            &pool,
            &admin_id.to_string(),
            &input,
            SERVER_KEY,
        )
        .await;
        assert!(matches!(result, Err(AppError::Validation(_))));
    });
}

// --- T044: HTTP bearer middleware tests through /api/v1/* endpoints ---

/// Helper: build the public API router with a test state.
fn build_api_router(pool: SqlitePool) -> axum::Router {
    let state = common::build_test_state(pool, std::str::from_utf8(SERVER_KEY).unwrap());
    server::public_api::public_api_routes().with_state(state)
}

/// T044: In-scope token → 200 OK.
#[tokio::test]
async fn http_bearer_in_scope_returns_200() {
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    let pool = test_pool().await;
    let admin_id = setup_admin(&pool).await;
    let input = ApiTokenInput {
        name: "http-in-scope".to_string(),
        scopes: vec!["services:read".to_string()],
        expires_at: None,
    };
    let secret = extended_auth_queries::create_api_token_query(
        &pool,
        &admin_id.to_string(),
        &input,
        SERVER_KEY,
    )
    .await
    .expect("create token");

    let app = build_api_router(pool);
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/services")
                .header("authorization", format!("Bearer {}", secret.secret))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("response");

    assert_eq!(
        response.status(),
        StatusCode::OK,
        "in-scope token should return 200"
    );
}

/// T044: Out-of-scope token → 403 Forbidden.
#[tokio::test]
async fn http_bearer_out_of_scope_returns_403() {
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    let pool = test_pool().await;
    let admin_id = setup_admin(&pool).await;
    let input = ApiTokenInput {
        name: "http-out-of-scope".to_string(),
        scopes: vec!["services:read".to_string()],
        expires_at: None,
    };
    let secret = extended_auth_queries::create_api_token_query(
        &pool,
        &admin_id.to_string(),
        &input,
        SERVER_KEY,
    )
    .await
    .expect("create token");

    let app = build_api_router(pool);
    // POST /api/v1/services requires services:write scope; token only has services:read
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/services")
                .header("authorization", format!("Bearer {}", secret.secret))
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({"name": "test", "url": "https://test.example.com"})
                        .to_string(),
                ))
                .unwrap(),
        )
        .await
        .expect("response");

    assert_eq!(
        response.status(),
        StatusCode::FORBIDDEN,
        "out-of-scope token should return 403"
    );
}

/// T044: Revoked token → 401 Unauthorized.
#[tokio::test]
async fn http_bearer_revoked_returns_401() {
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    let pool = test_pool().await;
    let admin_id = setup_admin(&pool).await;
    let input = ApiTokenInput {
        name: "http-revoked".to_string(),
        scopes: vec!["services:read".to_string()],
        expires_at: None,
    };
    let secret = extended_auth_queries::create_api_token_query(
        &pool,
        &admin_id.to_string(),
        &input,
        SERVER_KEY,
    )
    .await
    .expect("create token");

    extended_auth_queries::revoke_api_token_query(&pool, secret.id, &admin_id.to_string())
        .await
        .expect("revoke");

    let app = build_api_router(pool);
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/services")
                .header("authorization", format!("Bearer {}", secret.secret))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("response");

    assert_eq!(
        response.status(),
        StatusCode::UNAUTHORIZED,
        "revoked token should return 401"
    );
}

/// T044: Missing token → 401 Unauthorized.
#[tokio::test]
async fn http_bearer_missing_returns_401() {
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    let pool = test_pool().await;
    let _ = setup_admin(&pool).await;

    let app = build_api_router(pool);
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/services")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("response");

    assert_eq!(
        response.status(),
        StatusCode::UNAUTHORIZED,
        "missing token should return 401"
    );
}
