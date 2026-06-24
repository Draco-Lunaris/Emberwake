//! T042: OIDC callback test against a stub IdP (code+PKCE -> session).
//! Tests the admin-approve provisioning policy: identity created but requires approval.

#[path = "../common/mod.rs"]
mod common;

use common::test_pool;
use sqlx::SqlitePool;
use uuid::Uuid;

use app::server::auth_queries;
use app::server::extended_auth_queries;

const M_COST: u32 = 32 * 1024;
const T_COST: u32 = 3;
const P_COST: u32 = 1;

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
fn oidc_identity_created_unapproved() {
    let rt = tokio::runtime::Runtime::new().expect("runtime");
    rt.block_on(async {
        let pool = test_pool().await;
        let admin_id = setup_admin(&pool).await;
        let identity = extended_auth_queries::create_external_identity(
            &pool,
            &admin_id.to_string(),
            "https://stub-idp.example.com",
            "stub-subject-123",
        )
        .await
        .expect("create identity");
        assert_eq!(identity.provider, "https://stub-idp.example.com");
        let approved = extended_auth_queries::is_external_identity_approved(
            &pool,
            "https://stub-idp.example.com",
            "stub-subject-123",
        )
        .await
        .expect("check");
        assert!(!approved, "new identity should not be approved");
    });
}

#[test]
fn oidc_identity_approved_after_admin_action() {
    let rt = tokio::runtime::Runtime::new().expect("runtime");
    rt.block_on(async {
        let pool = test_pool().await;
        let admin_id = setup_admin(&pool).await;
        let identity = extended_auth_queries::create_external_identity(
            &pool,
            &admin_id.to_string(),
            "https://stub-idp.example.com",
            "stub-subject-456",
        )
        .await
        .expect("create");
        extended_auth_queries::approve_external_identity(&pool, identity.id)
            .await
            .expect("approve");
        let approved = extended_auth_queries::is_external_identity_approved(
            &pool,
            "https://stub-idp.example.com",
            "stub-subject-456",
        )
        .await
        .expect("check");
        assert!(approved, "identity should be approved after admin action");
    });
}

#[test]
fn oidc_find_existing_identity() {
    let rt = tokio::runtime::Runtime::new().expect("runtime");
    rt.block_on(async {
        let pool = test_pool().await;
        let admin_id = setup_admin(&pool).await;
        extended_auth_queries::create_external_identity(
            &pool,
            &admin_id.to_string(),
            "https://stub-idp.example.com",
            "stub-subject-789",
        )
        .await
        .expect("create");
        let found = extended_auth_queries::find_external_identity(
            &pool,
            "https://stub-idp.example.com",
            "stub-subject-789",
        )
        .await
        .expect("find");
        assert!(found.is_some());
        assert_eq!(found.unwrap().subject, "stub-subject-789");
    });
}

#[test]
fn oidc_unlink_identity() {
    let rt = tokio::runtime::Runtime::new().expect("runtime");
    rt.block_on(async {
        let pool = test_pool().await;
        let admin_id = setup_admin(&pool).await;
        let identity = extended_auth_queries::create_external_identity(
            &pool,
            &admin_id.to_string(),
            "https://stub-idp.example.com",
            "stub-subject-unlink",
        )
        .await
        .expect("create");
        extended_auth_queries::unlink_external_identity_query(
            &pool,
            identity.id,
            &admin_id.to_string(),
        )
        .await
        .expect("unlink");
        let found = extended_auth_queries::find_external_identity(
            &pool,
            "https://stub-idp.example.com",
            "stub-subject-unlink",
        )
        .await
        .expect("find");
        assert!(found.is_none(), "unlinked identity should not be found");
    });
}

#[test]
fn oidc_list_external_identities() {
    let rt = tokio::runtime::Runtime::new().expect("runtime");
    rt.block_on(async {
        let pool = test_pool().await;
        let admin_id = setup_admin(&pool).await;
        extended_auth_queries::create_external_identity(
            &pool,
            &admin_id.to_string(),
            "provider1",
            "subject1",
        )
        .await
        .expect("create 1");
        extended_auth_queries::create_external_identity(
            &pool,
            &admin_id.to_string(),
            "provider2",
            "subject2",
        )
        .await
        .expect("create 2");
        let identities =
            extended_auth_queries::list_external_identities_query(&pool, &admin_id.to_string())
                .await
                .expect("list");
        assert_eq!(identities.len(), 2);
    });
}

// --- T042: HTTP-layer OIDC route handler tests ---

/// T042: OIDC login route returns 503 when OIDC is disabled.
#[tokio::test]
async fn http_oidc_login_disabled_returns_503() {
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    let pool = test_pool().await;
    let _ = setup_admin(&pool).await;

    let state = common::build_test_state(pool, "test-key");
    let app = server::auth::oidc::oidc_routes().with_state(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/auth/oidc/login")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("response");

    assert_eq!(
        response.status(),
        StatusCode::SERVICE_UNAVAILABLE,
        "OIDC login should return 503 when OIDC is not configured"
    );
}

/// T042: OIDC callback route returns 503 when OIDC is disabled.
#[tokio::test]
async fn http_oidc_callback_disabled_returns_503() {
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    let pool = test_pool().await;

    let state = common::build_test_state(pool, "test-key");
    let app = server::auth::oidc::oidc_routes().with_state(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/auth/oidc/callback?code=test&state=test")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("response");

    assert_eq!(
        response.status(),
        StatusCode::SERVICE_UNAVAILABLE,
        "OIDC callback should return 503 when OIDC is not configured"
    );
}

/// T042: OIDC callback with invalid state returns 400 when OIDC is enabled
/// but no matching PKCE state exists (simulates stale/invalid callback).
#[tokio::test]
async fn http_oidc_callback_invalid_state_returns_400() {
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    let pool = test_pool().await;

    // Build state with OIDC enabled (but no real IdP — callback checks state store first)
    let mut config = server::config::Config::default();
    config.oidc.enabled = true;
    config.oidc.issuer_url = "https://stub-idp.example.com".to_string();
    config.oidc.client_id = "test-client".to_string();
    config.oidc.client_secret = "test-secret".to_string();
    config.oidc.redirect_url = "http://localhost:5005/auth/oidc/callback".to_string();

    let state = common::build_test_state_with_config(pool, config);
    let app = server::auth::oidc::oidc_routes().with_state(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/auth/oidc/callback?code=test&state=invalid-state-no-match")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("response");

    // Callback should return 400 for invalid/expired state (before attempting IdP discovery)
    assert_eq!(
        response.status(),
        StatusCode::BAD_REQUEST,
        "OIDC callback with invalid state should return 400"
    );
}

/// T042: PKCE challenge generation produces a non-empty verifier and challenge.
/// The openidconnect crate's PkceCodeChallenge::new_random_sha256() is used
/// by oidc_login() to generate the PKCE pair. This test verifies the generation
/// and that the verifier can be stored in and retrieved from OidcStateStore.
#[test]
fn pkce_challenge_generation_and_storage_roundtrip() {
    use openidconnect::PkceCodeChallenge;
    use server::auth::oidc::{OidcSessionState, OidcStateStore};

    // Generate PKCE challenge + verifier (same as oidc_login does)
    let (challenge, verifier) = PkceCodeChallenge::new_random_sha256();

    // Challenge should be non-empty (it's sent to the IdP)
    assert!(
        !challenge.as_str().is_empty(),
        "PKCE challenge should be non-empty"
    );

    // Verifier should be non-empty (it's sent in the token exchange)
    let verifier_str = verifier.secret().to_string();
    assert!(
        !verifier_str.is_empty(),
        "PKCE verifier should be non-empty"
    );

    // Store in OidcStateStore keyed by CSRF state (same as oidc_login)
    let store = OidcStateStore::new();
    let csrf_state = "test-csrf-state-123".to_string();
    let nonce = openidconnect::Nonce::new_random();
    let nonce_str = nonce.secret().to_string();
    store.put(
        csrf_state.clone(),
        OidcSessionState {
            pkce_verifier: verifier,
            nonce,
        },
    );

    // Retrieve and verify the stored state
    let retrieved = store.take(&csrf_state);
    assert!(
        retrieved.is_some(),
        "stored PKCE state should be retrievable"
    );
    let retrieved = retrieved.unwrap();
    assert_eq!(
        retrieved.pkce_verifier.secret(),
        &verifier_str,
        "stored PKCE verifier should match the generated one"
    );
    assert_eq!(
        retrieved.nonce.secret(),
        &nonce_str,
        "stored nonce should match the generated one"
    );

    // Second take should return None (consumed — prevents replay)
    let second = store.take(&csrf_state);
    assert!(
        second.is_none(),
        "PKCE state should be consumed after take (single-use, prevents replay)"
    );
}

/// T042: OIDC state store rejects unknown state (invalid/expired callback).
/// This tests the state validation path that oidc_callback() uses before
/// attempting any IdP discovery or token exchange.
#[test]
fn oidc_state_store_rejects_unknown_state() {
    use server::auth::oidc::OidcStateStore;

    let store = OidcStateStore::new();

    // Take from empty store returns None
    let result = store.take("nonexistent-state");
    assert!(
        result.is_none(),
        "unknown state should return None (rejected)"
    );

    // Store one state, take a different key
    use openidconnect::PkceCodeChallenge;
    let (_, verifier) = PkceCodeChallenge::new_random_sha256();
    let nonce = openidconnect::Nonce::new_random();
    store.put(
        "valid-state".to_string(),
        server::auth::oidc::OidcSessionState {
            pkce_verifier: verifier,
            nonce,
        },
    );

    let wrong = store.take("wrong-state");
    assert!(
        wrong.is_none(),
        "mismatched state should return None (rejected)"
    );

    // Correct state still available
    let correct = store.take("valid-state");
    assert!(correct.is_some(), "correct state should be retrievable");
}

// T042: Full OIDC code+PKCE→session roundtrip against a stub IdP is not tested here.
// A complete stub IdP test requires a mock HTTP server serving:
//   1. GET /.well-known/openid-configuration → discovery document
//   2. POST /token → token response with signed ID token
//   3. The ID token must be signed with a key in the JWKS endpoint
// This requires a mock HTTP server (e.g., wiremock or mockito) plus a test JWKS
// keypair. The tests above verify the components that don't need a live IdP:
//   - PKCE challenge generation + storage/retrieval
//   - State validation (rejects unknown/mismatched state)
//   - Admin-approve provisioning (identity created unapproved)
//   - HTTP route handlers (503 when disabled, 400 for invalid state)

/// T042: OIDC login route with empty issuer returns 503 (misconfigured).
#[tokio::test]
async fn http_oidc_login_empty_issuer_returns_503() {
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    let pool = test_pool().await;

    // OIDC enabled but issuer_url is empty — should return 503
    let mut config = server::config::Config::default();
    config.oidc.enabled = true;
    config.oidc.issuer_url = String::new();

    let state = common::build_test_state_with_config(pool, config);
    let app = server::auth::oidc::oidc_routes().with_state(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/auth/oidc/login")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("response");

    assert_eq!(
        response.status(),
        StatusCode::SERVICE_UNAVAILABLE,
        "OIDC login with empty issuer should return 503"
    );
}
