//! OIDC client: auth code + PKCE flow, provider discovery.
//! Routes: /auth/oidc/login (redirect to IdP), /auth/oidc/callback (exchange code).
//! Provisioning policy: admin-approve — new identities require admin approval.

use std::collections::HashMap;
use std::sync::{Arc, LazyLock, Mutex};

use axum::Router;
use axum::extract::{Query, State};
use axum::response::{IntoResponse, Redirect};
use axum::routing::get;
use openidconnect::core::{CoreAuthenticationFlow, CoreClient, CoreProviderMetadata};
use openidconnect::{
    AuthorizationCode, ClientId, ClientSecret, CsrfToken, IssuerUrl, Nonce, PkceCodeChallenge,
    PkceCodeVerifier, RedirectUrl, Scope, TokenResponse,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::state::AppState;

/// In-memory store for OIDC PKCE verifiers and nonces, keyed by CSRF state.
#[derive(Clone, Default)]
pub struct OidcStateStore {
    inner: Arc<Mutex<HashMap<String, OidcSessionState>>>,
}

struct OidcSessionState {
    pkce_verifier: PkceCodeVerifier,
    nonce: Nonce,
}

impl OidcStateStore {
    pub fn new() -> Self {
        Self::default()
    }

    fn put(&self, key: String, state: OidcSessionState) {
        if let Ok(mut map) = self.inner.lock() {
            map.insert(key, state);
        }
    }

    fn take(&self, key: &str) -> Option<OidcSessionState> {
        if let Ok(mut map) = self.inner.lock() {
            map.remove(key)
        } else {
            None
        }
    }
}

static OIDC_STORE: LazyLock<OidcStateStore> = LazyLock::new(OidcStateStore::new);

#[derive(Deserialize)]
pub struct CallbackParams {
    code: String,
    state: String,
}

/// Build a reqwest async HTTP client for OIDC discovery.
fn http_client() -> reqwest::Client {
    reqwest::Client::builder().build().unwrap_or_default()
}

/// GET /auth/oidc/login — redirect to IdP authorization endpoint.
pub async fn oidc_login(State(state): State<AppState>) -> impl IntoResponse {
    let oidc = &state.config.oidc;
    if !oidc.enabled || oidc.issuer_url.is_empty() {
        return error_response(503, "OIDC not configured");
    }

    let issuer_url = match IssuerUrl::new(oidc.issuer_url.clone()) {
        Ok(u) => u,
        Err(_) => return error_response(500, "invalid issuer URL"),
    };

    let client = http_client();
    let metadata = match CoreProviderMetadata::discover_async(issuer_url, &client).await {
        Ok(m) => m,
        Err(e) => return error_response(502, &format!("IdP discovery failed: {e}")),
    };

    let redirect_url = match RedirectUrl::new(oidc.redirect_url.clone()) {
        Ok(u) => u,
        Err(_) => return error_response(500, "invalid redirect URL"),
    };

    let oidc_client = CoreClient::from_provider_metadata(
        metadata,
        ClientId::new(oidc.client_id.clone()),
        Some(ClientSecret::new(oidc.client_secret.clone())),
    )
    .set_redirect_uri(redirect_url);

    let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();
    let (auth_url, csrf_token, nonce) = oidc_client
        .authorize_url(
            CoreAuthenticationFlow::AuthorizationCode,
            CsrfToken::new_random,
            Nonce::new_random,
        )
        .add_scope(Scope::new("openid".to_string()))
        .add_scope(Scope::new("email".to_string()))
        .add_scope(Scope::new("profile".to_string()))
        .set_pkce_challenge(pkce_challenge)
        .url();

    OIDC_STORE.put(
        csrf_token.secret().to_string(),
        OidcSessionState {
            pkce_verifier,
            nonce,
        },
    );

    Redirect::to(auth_url.as_str()).into_response()
}

/// GET /auth/oidc/callback — exchange code, map to local user.
pub async fn oidc_callback(
    State(state): State<AppState>,
    Query(params): Query<CallbackParams>,
) -> impl IntoResponse {
    let oidc = &state.config.oidc;
    if !oidc.enabled {
        return error_response(503, "OIDC not configured");
    }

    let session_state = match OIDC_STORE.take(&params.state) {
        Some(s) => s,
        None => return error_response(400, "invalid or expired state"),
    };

    let issuer_url = match IssuerUrl::new(oidc.issuer_url.clone()) {
        Ok(u) => u,
        Err(_) => return error_response(500, "invalid issuer URL"),
    };

    let client = http_client();
    let metadata = match CoreProviderMetadata::discover_async(issuer_url, &client).await {
        Ok(m) => m,
        Err(e) => return error_response(502, &format!("IdP discovery failed: {e}")),
    };

    let redirect_url = match RedirectUrl::new(oidc.redirect_url.clone()) {
        Ok(u) => u,
        Err(_) => return error_response(500, "invalid redirect URL"),
    };

    let oidc_client = CoreClient::from_provider_metadata(
        metadata,
        ClientId::new(oidc.client_id.clone()),
        Some(ClientSecret::new(oidc.client_secret.clone())),
    )
    .set_redirect_uri(redirect_url);

    let token_request = match oidc_client.exchange_code(AuthorizationCode::new(params.code)) {
        Ok(req) => req,
        Err(e) => return error_response(400, &format!("token exchange setup failed: {e}")),
    };
    let token_response = match token_request
        .set_pkce_verifier(session_state.pkce_verifier)
        .request_async(&client)
        .await
    {
        Ok(t) => t,
        Err(e) => return error_response(400, &format!("token exchange failed: {e}")),
    };

    let id_token = match token_response.id_token() {
        Some(t) => t,
        None => return error_response(400, "no ID token in response"),
    };

    let claims = match id_token.claims(&oidc_client.id_token_verifier(), &session_state.nonce) {
        Ok(c) => c,
        Err(e) => return error_response(400, &format!("ID token verification failed: {e}")),
    };

    let subject = claims.subject().to_string();
    let provider = oidc.issuer_url.clone();

    // Check if identity exists and is approved
    let identity =
        app::server::extended_auth_queries::find_external_identity(&state.db, &provider, &subject)
            .await
            .ok()
            .flatten();

    match identity {
        Some(ext) => {
            let approved = app::server::extended_auth_queries::is_external_identity_approved(
                &state.db, &provider, &subject,
            )
            .await
            .unwrap_or(false);

            if approved {
                let (token, csrf) = app::server::auth_queries::create_session(
                    &state.db,
                    &ext.user_id.to_string(),
                    None,
                    None,
                )
                .await
                .unwrap_or_else(|_| (String::new(), String::new()));

                let session_cookie = app::server::auth_queries::build_session_cookie(&token, false);
                let csrf_cookie = app::server::auth_queries::build_csrf_cookie(&csrf, false);
                axum::response::Response::builder()
                    .status(302)
                    .header("set-cookie", session_cookie)
                    .header("set-cookie", csrf_cookie)
                    .header("location", "/")
                    .body(axum::body::Body::from(""))
                    .unwrap()
            } else {
                pending_approval_response()
            }
        }
        None => {
            // Create a new local user for this OIDC identity (no password, OIDC-only)
            let user_id = Uuid::now_v7().to_string();
            let now = chrono::Utc::now().to_rfc3339();
            let username = format!("oidc_{}", &subject[..subject.len().min(32)]);

            let _ = sqlx::query(
                "INSERT INTO users (id, username, email, password_hash, role, is_active, created_at, updated_at) \
                 VALUES (?, ?, ?, NULL, 'user', 1, ?, ?)",
            )
            .bind(&user_id)
            .bind(&username)
            .bind(claims.email().map(|e| e.as_str().to_string()))
            .bind(&now)
            .bind(&now)
            .execute(&state.db)
            .await;

            let _ = app::server::extended_auth_queries::create_external_identity(
                &state.db, &user_id, &provider, &subject,
            )
            .await;

            pending_approval_response()
        }
    }
}

fn error_response(status: u16, msg: &str) -> axum::response::Response {
    axum::response::Response::builder()
        .status(status)
        .body(axum::body::Body::from(msg.to_string()))
        .unwrap()
}

fn pending_approval_response() -> axum::response::Response {
    axum::response::Response::builder()
        .status(403)
        .body(axum::body::Body::from(
            "OIDC identity created. An administrator must approve your account before you can log in.",
        ))
        .unwrap()
}

/// Build the OIDC sub-router.
pub fn oidc_routes() -> Router<AppState> {
    Router::new()
        .route("/auth/oidc/login", get(oidc_login))
        .route("/auth/oidc/callback", get(oidc_callback))
}
