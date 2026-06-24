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
/// T017: Uses the same `apply_security_headers` function as `main.rs` — no mock duplication.
///
/// CSP with nonce is set via Leptos Meta tags during SSR, not via a tower layer.
/// Full CSP verification requires the cargo-leptos SSR pipeline (generate_route_list +
/// LeptosRoutes + shell()), which is not practical in a unit test. The four layer-based
/// headers below are verified against the production code path.
#[tokio::test]
async fn security_headers_present() {
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use server::security::headers::apply_security_headers;
    use tower::ServiceExt;

    // Build a router and apply security headers using the SAME function as main.rs.
    let app = apply_security_headers(
        axum::Router::new().route("/", axum::routing::get(|| async { "ok" })),
        31536000,
    );

    let response = app
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::OK);

    // HSTS
    let hsts = response
        .headers()
        .get("strict-transport-security")
        .expect("HSTS header must be present");
    assert!(
        hsts.to_str().unwrap().contains("max-age=31536000"),
        "HSTS must have correct max-age"
    );
    assert!(
        hsts.to_str().unwrap().contains("includeSubDomains"),
        "HSTS must include subdomains"
    );

    // X-Content-Type-Options
    assert_eq!(
        response
            .headers()
            .get("x-content-type-options")
            .expect("X-Content-Type-Options must be present")
            .to_str()
            .unwrap(),
        "nosniff",
        "X-Content-Type-Options must be nosniff"
    );

    // X-Frame-Options
    assert_eq!(
        response
            .headers()
            .get("x-frame-options")
            .expect("X-Frame-Options must be present")
            .to_str()
            .unwrap(),
        "DENY",
        "X-Frame-Options must be DENY"
    );

    // Referrer-Policy
    assert_eq!(
        response
            .headers()
            .get("referrer-policy")
            .expect("Referrer-Policy must be present")
            .to_str()
            .unwrap(),
        "no-referrer",
        "Referrer-Policy must be no-referrer"
    );

    // CSP header is injected by Leptos Meta tags during SSR (per-response nonce).
    // It is not a static tower layer — verifying it requires the full Leptos SSR pipeline.
}

/// T013: Verify OTLP exporter is created when init_otlp is called.
/// The exporter builder creates an HTTP exporter object — it does not connect to the endpoint.
/// Connection happens lazily when spans are batched and exported.
#[tokio::test]
async fn otlp_exporter_created() {
    // init_otlp creates a real OTLP HTTP exporter with batch span processor
    // and installs it as the global tracer provider. The exporter does not
    // connect until spans are flushed, so a non-existent endpoint is safe here.
    // The guard flushes and shuts down the provider on drop.
    let guard = server::telemetry::init_otlp("http://localhost:4318/v1/traces");
    // If we reach this point without panicking, the exporter was created successfully.
    // Forget the guard to avoid blocking on flush to a non-existent endpoint during tests.
    // In production, the guard is held for the process lifetime and flushed on shutdown.
    std::mem::forget(guard);
}
