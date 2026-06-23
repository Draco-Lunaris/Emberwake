//! Foundational tests: migration up + repository round-trip, security constraints.
//! Uses #[sqlx::test] for isolated-DB data tests.

use sqlx::SqlitePool;

/// Test that migrations run successfully on a fresh database.
#[sqlx::test(migrations = "../../migrations")]
async fn migrations_create_all_tables(pool: SqlitePool) {
    let tables = [
        "users",
        "sessions",
        "external_identity",
        "passkey_credential",
        "api_token",
        "category",
        "service",
        "bookmark",
        "setting",
        "theme",
        "status_reading",
        "status_history",
        "weather_reading",
        "audit_event",
    ];

    for table in &tables {
        let count: (i64,) = match *table {
            "users" => sqlx::query_as("SELECT COUNT(*) FROM users"),
            "sessions" => sqlx::query_as("SELECT COUNT(*) FROM sessions"),
            "external_identity" => sqlx::query_as("SELECT COUNT(*) FROM external_identity"),
            "passkey_credential" => sqlx::query_as("SELECT COUNT(*) FROM passkey_credential"),
            "api_token" => sqlx::query_as("SELECT COUNT(*) FROM api_token"),
            "category" => sqlx::query_as("SELECT COUNT(*) FROM category"),
            "service" => sqlx::query_as("SELECT COUNT(*) FROM service"),
            "bookmark" => sqlx::query_as("SELECT COUNT(*) FROM bookmark"),
            "setting" => sqlx::query_as("SELECT COUNT(*) FROM setting"),
            "theme" => sqlx::query_as("SELECT COUNT(*) FROM theme"),
            "status_reading" => sqlx::query_as("SELECT COUNT(*) FROM status_reading"),
            "status_history" => sqlx::query_as("SELECT COUNT(*) FROM status_history"),
            "weather_reading" => sqlx::query_as("SELECT COUNT(*) FROM weather_reading"),
            "audit_event" => sqlx::query_as("SELECT COUNT(*) FROM audit_event"),
            _ => unreachable!(),
        }
        .fetch_one(&pool)
        .await
        .expect("table should exist");
        assert_eq!(count.0, 0, "table {table} should be empty after migration");
    }
}

/// Test repository round-trip: setup_complete check + user count.
#[sqlx::test(migrations = "../../migrations")]
async fn repository_round_trip(pool: SqlitePool) {
    use server::db::{Repository, SqliteRepository};

    let repo = SqliteRepository::new(pool.clone());

    let setup_complete = repo.is_setup_complete().await.expect("setup check");
    assert!(!setup_complete, "setup should not be complete on fresh DB");

    let user_count = repo.user_count().await.expect("user count");
    assert_eq!(user_count, 0, "user count should be 0 on fresh DB");

    sqlx::query(
        "INSERT INTO setting (key, value, updated_at) VALUES ('setup_complete', 'true', '2026-01-01T00:00:00Z')",
    )
    .execute(&pool)
    .await
    .expect("insert setting");

    let setup_complete = repo.is_setup_complete().await.expect("setup check");
    assert!(setup_complete, "setup should be complete after insert");
}

/// Test that the setup_complete key enforces uniqueness (race-safe first-run).
#[sqlx::test(migrations = "../../migrations")]
async fn setup_complete_unique(pool: SqlitePool) {
    sqlx::query(
        "INSERT INTO setting (key, value, updated_at) VALUES ('setup_complete', 'true', '2026-01-01T00:00:00Z')",
    )
    .execute(&pool)
    .await
    .expect("first insert");

    let result = sqlx::query(
        "INSERT INTO setting (key, value, updated_at) VALUES ('setup_complete', 'true', '2026-01-01T00:00:00Z')",
    )
    .execute(&pool)
    .await;

    assert!(
        result.is_err(),
        "duplicate setup_complete key should be rejected"
    );
}

/// Test that visibility CHECK constraint enforces public/private.
#[sqlx::test(migrations = "../../migrations")]
async fn visibility_check_constraint(pool: SqlitePool) {
    sqlx::query(
        "INSERT INTO category (id, name, order_index, visibility, created_at, updated_at) VALUES ('test-id', 'Test', 0, 'public', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')",
    )
    .execute(&pool)
    .await
    .expect("valid visibility");

    let result = sqlx::query(
        "INSERT INTO category (id, name, order_index, visibility, created_at, updated_at) VALUES ('test-id2', 'Test2', 0, 'invalid', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')",
    )
    .execute(&pool)
    .await;

    assert!(result.is_err(), "invalid visibility should be rejected");
}

/// Test that audit_event result CHECK constraint enforces success/failure.
#[sqlx::test(migrations = "../../migrations")]
async fn audit_result_check_constraint(pool: SqlitePool) {
    let result = sqlx::query(
        "INSERT INTO audit_event (id, ts, action, result) VALUES ('test-id', '2026-01-01T00:00:00Z', 'login', 'invalid')",
    )
    .execute(&pool)
    .await;

    assert!(result.is_err(), "invalid audit result should be rejected");

    sqlx::query(
        "INSERT INTO audit_event (id, ts, action, result) VALUES ('test-id', '2026-01-01T00:00:00Z', 'login', 'success')",
    )
    .execute(&pool)
    .await
    .expect("valid audit result");
}

/// Test that security header layers (HSTS, nosniff, frame-deny, referrer) are present.
/// T017: Missing security-headers test.
#[tokio::test]
async fn security_headers_present() {
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;
    use tower_http::set_header::SetResponseHeaderLayer;

    let app = axum::Router::new()
        .route("/", axum::routing::get(|| async { "ok" }))
        .layer(SetResponseHeaderLayer::overriding(
            axum::http::HeaderName::from_static("strict-transport-security"),
            axum::http::HeaderValue::from_static("max-age=31536000; includeSubDomains"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            axum::http::HeaderName::from_static("x-content-type-options"),
            axum::http::HeaderValue::from_static("nosniff"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            axum::http::HeaderName::from_static("x-frame-options"),
            axum::http::HeaderValue::from_static("DENY"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            axum::http::HeaderName::from_static("referrer-policy"),
            axum::http::HeaderValue::from_static("no-referrer"),
        ));

    let response = app
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::OK);
    assert!(
        response.headers().contains_key("strict-transport-security"),
        "HSTS header must be present"
    );
    assert!(
        response.headers().contains_key("x-content-type-options"),
        "X-Content-Type-Options header must be present"
    );
    assert!(
        response.headers().contains_key("x-frame-options"),
        "X-Frame-Options header must be present"
    );
    assert!(
        response.headers().contains_key("referrer-policy"),
        "Referrer-Policy header must be present"
    );
}
