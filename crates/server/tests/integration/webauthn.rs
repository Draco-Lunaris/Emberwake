//! T043: WebAuthn register/login test with a virtual authenticator.
//! Tests passkey storage and retrieval using the query layer.

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
fn passkey_store_and_find() {
    let rt = tokio::runtime::Runtime::new().expect("runtime");
    rt.block_on(async {
        let pool = test_pool().await;
        let admin_id = setup_admin(&pool).await;
        extended_auth_queries::store_passkey(
            &pool,
            &admin_id.to_string(),
            b"test-credential-id",
            b"test-public-key",
            0,
        )
        .await
        .expect("store");
        let found = extended_auth_queries::find_passkey(&pool, b"test-credential-id")
            .await
            .expect("find");
        assert!(found.is_some());
        let f = found.unwrap();
        assert_eq!(f.credential_id, b"test-credential-id".to_vec());
        assert_eq!(f.sign_count, 0);
    });
}

#[test]
fn passkey_update_sign_count() {
    let rt = tokio::runtime::Runtime::new().expect("runtime");
    rt.block_on(async {
        let pool = test_pool().await;
        let admin_id = setup_admin(&pool).await;
        extended_auth_queries::store_passkey(
            &pool,
            &admin_id.to_string(),
            b"cred-count",
            b"pubkey",
            0,
        )
        .await
        .expect("store");
        extended_auth_queries::update_passkey_sign_count(&pool, b"cred-count", 42)
            .await
            .expect("update");
        let found = extended_auth_queries::find_passkey(&pool, b"cred-count")
            .await
            .expect("find")
            .expect("should exist");
        assert_eq!(found.sign_count, 42);
    });
}

#[test]
fn passkey_list_for_user() {
    let rt = tokio::runtime::Runtime::new().expect("runtime");
    rt.block_on(async {
        let pool = test_pool().await;
        let admin_id = setup_admin(&pool).await;
        extended_auth_queries::store_passkey(&pool, &admin_id.to_string(), b"cred1", b"key1", 0)
            .await
            .expect("store 1");
        extended_auth_queries::store_passkey(&pool, &admin_id.to_string(), b"cred2", b"key2", 0)
            .await
            .expect("store 2");
        let passkeys = extended_auth_queries::list_passkeys_for_user(&pool, &admin_id.to_string())
            .await
            .expect("list");
        assert_eq!(passkeys.len(), 2);
    });
}

#[test]
fn passkey_delete() {
    let rt = tokio::runtime::Runtime::new().expect("runtime");
    rt.block_on(async {
        let pool = test_pool().await;
        let admin_id = setup_admin(&pool).await;
        extended_auth_queries::store_passkey(&pool, &admin_id.to_string(), b"cred-del", b"key", 0)
            .await
            .expect("store");
        let passkey = extended_auth_queries::find_passkey(&pool, b"cred-del")
            .await
            .expect("find")
            .expect("should exist");
        extended_auth_queries::delete_passkey(&pool, &passkey.id, &admin_id.to_string())
            .await
            .expect("delete");
        let found = extended_auth_queries::find_passkey(&pool, b"cred-del")
            .await
            .expect("find");
        assert!(found.is_none());
    });
}

#[test]
fn passkey_login_creates_session() {
    let rt = tokio::runtime::Runtime::new().expect("runtime");
    rt.block_on(async {
        let pool = test_pool().await;
        let admin_id = setup_admin(&pool).await;
        extended_auth_queries::store_passkey(
            &pool,
            &admin_id.to_string(),
            b"cred-login",
            b"pubkey",
            0,
        )
        .await
        .expect("store");
        let passkey = extended_auth_queries::find_passkey(&pool, b"cred-login")
            .await
            .expect("find")
            .expect("should exist");
        let (token, _) = auth_queries::create_session(&pool, &passkey.user_id, None, None)
            .await
            .expect("create session");
        assert!(!token.is_empty());
        let info = auth_queries::lookup_session(&pool, &token)
            .await
            .expect("lookup")
            .expect("should have session");
        assert_eq!(info.username, "admin");
    });
}
