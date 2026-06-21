//! Emberwake server entry point.
//! Loads config, initializes DB pool, runs migrations, builds the Axum router,
//! and serves the Leptos SSR application with security headers + telemetry.

use std::sync::Arc;

use app::App;
use leptos::prelude::*;
use leptos_axum::{LeptosRoutes, file_and_error_handler, generate_route_list};
use tower_http::set_header::SetResponseHeaderLayer;

use server::{audit, config, db, integrations, monitor, sse, state::AppState, telemetry};

/// Shell function for SSR rendering — wraps App in HTML document.
/// `provide_nonce()` is called here to generate a per-response CSP nonce.
fn shell(_options: LeptosOptions) -> impl IntoView {
    leptos::nonce::provide_nonce();

    view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8" />
                <meta name="viewport" content="width=device-width, initial-scale=1" />
                <title>"Emberwake"</title>
            </head>
            <body>
                <App />
            </body>
        </html>
    }
}

#[tokio::main]
async fn main() {
    let config = config::load().expect("failed to load configuration");

    telemetry::init_tracing(&config.telemetry.log_level);
    tracing::info!("Starting Emberwake server");

    config::ensure_db_dir(&config.db_path).expect("failed to create db directory");
    let pool = db::init_pool(&config.db_path)
        .await
        .expect("failed to initialize database pool");

    // Seed built-in themes (Light + Dark) if none exist.
    if let Err(e) = app::server::settings_queries::seed_builtin_themes(&pool).await {
        tracing::warn!("Failed to seed builtin themes: {e}");
    }

    let audit = Arc::new(audit::AuditWriter::new(pool.clone()));
    let sse_hub = sse::SseHub::new(256);
    let discovery_cache = app::server::discovery::DiscoveryCache::new();

    let options = LeptosOptions::builder()
        .output_name("emberwake")
        .site_addr(
            config
                .bind_addr
                .parse::<std::net::SocketAddr>()
                .expect("valid bind addr"),
        )
        .build();

    let state = AppState {
        leptos_options: options.clone(),
        db: pool.clone(),
        config: Arc::new(config.clone()),
        audit: audit.clone(),
        sse_hub: sse_hub.clone(),
    };

    db::backup::spawn_checkpoint_task(pool.clone(), config.backup.checkpoint_interval_s);
    db::backup::spawn_backup_task(pool.clone(), config.db_path.clone(), config.backup.clone());
    monitor::scheduler::spawn_scheduler(pool.clone(), sse_hub.clone());

    let routes = generate_route_list(App);

    let server_key = if config.server_key.is_empty() {
        tracing::warn!("server_key not set — API tokens disabled; set EMBERWAKE__SERVER_KEY");
        Vec::new()
    } else {
        config.server_key.as_bytes().to_vec()
    };

    integrations::weather::spawn_scheduler(pool.clone(), sse_hub.clone(), server_key.clone());
    integrations::docker::spawn_scheduler(pool.clone(), sse_hub.clone(), discovery_cache.clone());
    integrations::kubernetes::spawn_scheduler(
        pool.clone(),
        sse_hub.clone(),
        discovery_cache.clone(),
    );

    let rp_id = format!(
        "localhost:{}",
        config.bind_addr.split(':').nth(1).unwrap_or("5005")
    );
    let webauthn_rp = app::server::extended_auth::WebAuthnRpInfo {
        rp_id: rp_id.clone(),
        rp_origin: format!("http://{}", rp_id),
    };
    let challenge_store = app::server::extended_auth::ChallengeStore::new();

    let app = axum::Router::<AppState>::new()
        .merge(telemetry::health_routes())
        .merge(server::auth::oidc::oidc_routes())
        .merge(server::public_api::public_api_routes())
        .merge(sse::handler::sse_routes())
        .leptos_routes(&state, routes, App)
        .fallback(file_and_error_handler::<server::state::AppState, _>(shell))
        .layer(SetResponseHeaderLayer::overriding(
            axum::http::HeaderName::from_static("strict-transport-security"),
            axum::http::HeaderValue::from_str(&format!(
                "max-age={}; includeSubDomains",
                config.security.hsts_max_age
            ))
            .expect("valid HSTS header"),
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
        ))
        .layer(axum::Extension(pool.clone()))
        .layer(axum::Extension(app::server::extended_auth::ServerKey(
            server_key,
        )))
        .layer(axum::Extension(webauthn_rp))
        .layer(axum::Extension(challenge_store))
        .layer(axum::Extension(discovery_cache))
        .layer(axum::Extension(app::server::auth::Argon2Params {
            m_cost: config.argon2.m_cost,
            t_cost: config.argon2.t_cost,
            p_cost: config.argon2.p_cost,
        }))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(&config.bind_addr)
        .await
        .expect("failed to bind");

    tracing::info!("Listening on http://{}", config.bind_addr);
    axum::serve(listener, app).await.expect("server error");
}
