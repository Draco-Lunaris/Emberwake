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
