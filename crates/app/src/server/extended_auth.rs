//! Extended auth server functions: OIDC, WebAuthn passkeys, scoped API tokens.
//! These are #[leptos::server] functions that call into extended_auth_queries.

use leptos::server_fn::ServerFnError;
use uuid::Uuid;

use crate::domain::{
    ApiTokenInput, ApiTokenSecret, ApiTokenSummary, AuthResponse, CredentialCreationOptions,
    ExternalIdentity, PasskeySummary, RedirectUrl, RegisterResponse, RequestOptions,
    SessionSummary,
};
use crate::error::AppError;

#[cfg(feature = "ssr")]
use webauthn_rs::prelude::*;

/// Server key for HMAC token hashing. Passed via Axum Extension.
#[derive(Clone)]
pub struct ServerKey(pub Vec<u8>);

/// WebAuthn RP info. Passed via Axum Extension.
#[derive(Clone)]
pub struct WebAuthnRpInfo {
    pub rp_id: String,
    pub rp_origin: String,
}

/// In-memory challenge store for WebAuthn flows.
#[derive(Clone, Default)]
pub struct ChallengeStore {
    inner: std::sync::Arc<std::sync::Mutex<std::collections::HashMap<String, Vec<u8>>>>,
}

impl ChallengeStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn put(&self, key: &str, value: Vec<u8>) {
        if let Ok(mut map) = self.inner.lock() {
            map.insert(key.to_string(), value);
        }
    }

    pub fn take(&self, key: &str) -> Option<Vec<u8>> {
        if let Ok(mut map) = self.inner.lock() {
            map.remove(key)
        } else {
            None
        }
    }
}

// --- OIDC ---

#[leptos::server]
pub async fn oidc_begin() -> Result<RedirectUrl, ServerFnError<AppError>> {
    #[cfg(feature = "ssr")]
    {
        Ok(RedirectUrl {
            url: "/auth/oidc/login".to_string(),
        })
    }
    #[cfg(not(feature = "ssr"))]
    {
        Err(ServerFnError::from(AppError::Internal))
    }
}

#[leptos::server]
pub async fn list_external_identities() -> Result<Vec<ExternalIdentity>, ServerFnError<AppError>> {
    #[cfg(feature = "ssr")]
    {
        use axum::Extension;
        let pool = leptos_axum::extract::<Extension<sqlx::SqlitePool>>()
            .await
            .map_err(|_| AppError::Internal)?
            .0;
        let info = crate::server::auth_helper::require_session(&pool).await?;
        crate::server::extended_auth_queries::list_external_identities_query(
            &pool,
            &info.user_id.to_string(),
        )
        .await
        .map_err(ServerFnError::from)
    }
    #[cfg(not(feature = "ssr"))]
    {
        Err(ServerFnError::from(AppError::Unauthorized))
    }
}

#[leptos::server]
pub async fn unlink_external_identity(id: Uuid) -> Result<(), ServerFnError<AppError>> {
    #[cfg(feature = "ssr")]
    {
        use axum::Extension;
        let pool = leptos_axum::extract::<Extension<sqlx::SqlitePool>>()
            .await
            .map_err(|_| AppError::Internal)?
            .0;
        let info = crate::server::auth_helper::require_session_csrf(&pool).await?;
        crate::server::extended_auth_queries::unlink_external_identity_query(
            &pool,
            id,
            &info.user_id.to_string(),
        )
        .await?;
        crate::server::auth_queries::audit_write_query(
            &pool,
            Some(info.user_id),
            "oidc_unlink",
            Some(&id.to_string()),
            None,
            None,
            "success",
        )
        .await;
        Ok(())
    }
    #[cfg(not(feature = "ssr"))]
    {
        let _ = id;
        Err(ServerFnError::from(AppError::Unauthorized))
    }
}

// --- WebAuthn passkeys ---

#[cfg(feature = "ssr")]
pub fn build_webauthn(rp_info: &WebAuthnRpInfo) -> Result<Webauthn, AppError> {
    let origin = Url::parse(&rp_info.rp_origin).map_err(|_| AppError::Internal)?;
    let builder = WebauthnBuilder::new(&rp_info.rp_id, &origin).map_err(|_| AppError::Internal)?;
    let builder = builder.rp_name("Emberwake");
    builder.build().map_err(|_| AppError::Internal)
}

#[leptos::server]
pub async fn passkey_register_begin() -> Result<CredentialCreationOptions, ServerFnError<AppError>>
{
    #[cfg(feature = "ssr")]
    {
        use axum::Extension;
        let pool = leptos_axum::extract::<Extension<sqlx::SqlitePool>>()
            .await
            .map_err(|_| AppError::Internal)?
            .0;
        let info = crate::server::auth_helper::require_session(&pool).await?;

        let rp_info = leptos_axum::extract::<Extension<WebAuthnRpInfo>>()
            .await
            .map_err(|_| AppError::Internal)?
            .0;
        let challenge_store = leptos_axum::extract::<Extension<ChallengeStore>>()
            .await
            .map_err(|_| AppError::Internal)?
            .0;

        let webauthn = build_webauthn(&rp_info)?;
        let (challenge, state) = webauthn
            .start_securitykey_registration(
                info.user_id,
                &info.username,
                &info.username,
                None,
                None,
                None,
            )
            .map_err(|_| AppError::Internal)?;

        let state_bytes = serde_json::to_vec(&state).map_err(|_| AppError::Internal)?;
        let key = format!("reg:{}", info.user_id);
        challenge_store.put(&key, state_bytes);

        let challenge_json = serde_json::to_value(&challenge).map_err(|_| AppError::Internal)?;
        Ok(CredentialCreationOptions {
            challenge: challenge_json,
        })
    }
    #[cfg(not(feature = "ssr"))]
    {
        Err(ServerFnError::from(AppError::Unauthorized))
    }
}

#[leptos::server]
pub async fn passkey_register_finish(
    resp: RegisterResponse,
) -> Result<(), ServerFnError<AppError>> {
    #[cfg(feature = "ssr")]
    {
        use axum::Extension;
        let pool = leptos_axum::extract::<Extension<sqlx::SqlitePool>>()
            .await
            .map_err(|_| AppError::Internal)?
            .0;
        let info = crate::server::auth_helper::require_session_csrf(&pool).await?;

        let rp_info = leptos_axum::extract::<Extension<WebAuthnRpInfo>>()
            .await
            .map_err(|_| AppError::Internal)?
            .0;
        let challenge_store = leptos_axum::extract::<Extension<ChallengeStore>>()
            .await
            .map_err(|_| AppError::Internal)?
            .0;

        let key = format!("reg:{}", info.user_id);
        let state_bytes = challenge_store.take(&key).ok_or(AppError::Internal)?;
        let state: SecurityKeyRegistration =
            serde_json::from_slice(&state_bytes).map_err(|_| AppError::Internal)?;

        let credential: RegisterPublicKeyCredential = serde_json::from_value(resp.credential)
            .map_err(|_| AppError::Validation("invalid credential response".into()))?;

        let webauthn = build_webauthn(&rp_info)?;
        let passkey = webauthn
            .finish_securitykey_registration(&credential, &state)
            .map_err(|_| AppError::Validation("registration failed".into()))?;

        let cred_id: &[u8] = passkey.cred_id().as_ref();
        let pub_key_bytes = serde_json::to_vec(&passkey).map_err(|_| AppError::Internal)?;

        crate::server::extended_auth_queries::store_passkey(
            &pool,
            &info.user_id.to_string(),
            cred_id,
            &pub_key_bytes,
            0,
        )
        .await?;

        crate::server::auth_queries::audit_write_query(
            &pool,
            Some(info.user_id),
            "passkey_register",
            None,
            None,
            None,
            "success",
        )
        .await;
        Ok(())
    }
    #[cfg(not(feature = "ssr"))]
    {
        let _ = resp;
        Err(ServerFnError::from(AppError::Unauthorized))
    }
}

#[leptos::server]
pub async fn passkey_login_begin(
    username: String,
) -> Result<RequestOptions, ServerFnError<AppError>> {
    #[cfg(feature = "ssr")]
    {
        use axum::Extension;
        let pool = leptos_axum::extract::<Extension<sqlx::SqlitePool>>()
            .await
            .map_err(|_| AppError::Internal)?
            .0;

        let rp_info = leptos_axum::extract::<Extension<WebAuthnRpInfo>>()
            .await
            .map_err(|_| AppError::Internal)?
            .0;
        let challenge_store = leptos_axum::extract::<Extension<ChallengeStore>>()
            .await
            .map_err(|_| AppError::Internal)?
            .0;

        let row: Option<(String,)> = sqlx::query_as(
            "SELECT id FROM users WHERE username = ? COLLATE NOCASE AND is_active = 1",
        )
        .bind(&username)
        .fetch_optional(&pool)
        .await
        .map_err(|_| AppError::Internal)?;

        let user_id_str = row.ok_or(AppError::Unauthorized)?.0;
        let user_id = Uuid::parse_str(&user_id_str).map_err(|_| AppError::Internal)?;

        let passkeys =
            crate::server::extended_auth_queries::list_passkeys_for_user(&pool, &user_id_str)
                .await?;

        if passkeys.is_empty() {
            return Err(ServerFnError::from(AppError::Unauthorized));
        }

        // Reconstruct SecurityKey from stored serialized public_key BLOB
        let security_keys: Vec<SecurityKey> = passkeys
            .iter()
            .filter_map(|p| serde_json::from_slice::<SecurityKey>(&p.public_key).ok())
            .collect();

        if security_keys.is_empty() {
            return Err(ServerFnError::from(AppError::Unauthorized));
        }

        let webauthn = build_webauthn(&rp_info)?;
        let (challenge, state) = webauthn
            .start_securitykey_authentication(&security_keys)
            .map_err(|_| AppError::Internal)?;

        let state_bytes = serde_json::to_vec(&state).map_err(|_| AppError::Internal)?;
        let key = format!("auth:{}", user_id);
        challenge_store.put(&key, state_bytes);

        let challenge_json = serde_json::to_value(&challenge).map_err(|_| AppError::Internal)?;
        Ok(RequestOptions {
            challenge: challenge_json,
        })
    }
    #[cfg(not(feature = "ssr"))]
    {
        let _ = username;
        Err(ServerFnError::from(AppError::Internal))
    }
}

#[leptos::server]
pub async fn passkey_login_finish(
    resp: AuthResponse,
) -> Result<SessionSummary, ServerFnError<AppError>> {
    #[cfg(feature = "ssr")]
    {
        use axum::Extension;
        use leptos::prelude::use_context;

        let pool = leptos_axum::extract::<Extension<sqlx::SqlitePool>>()
            .await
            .map_err(|_| AppError::Internal)?
            .0;

        let rp_info = leptos_axum::extract::<Extension<WebAuthnRpInfo>>()
            .await
            .map_err(|_| AppError::Internal)?
            .0;
        let challenge_store = leptos_axum::extract::<Extension<ChallengeStore>>()
            .await
            .map_err(|_| AppError::Internal)?
            .0;

        let assertion: PublicKeyCredential = serde_json::from_value(resp.assertion)
            .map_err(|_| AppError::Validation("invalid assertion".into()))?;

        let cred_id: &[u8] = assertion.raw_id.as_ref();

        let passkey = crate::server::extended_auth_queries::find_passkey(&pool, cred_id)
            .await?
            .ok_or(AppError::Unauthorized)?;

        let user_id = Uuid::parse_str(&passkey.user_id).map_err(|_| AppError::Internal)?;
        let key = format!("auth:{}", user_id);
        let state_bytes = challenge_store.take(&key).ok_or(AppError::Unauthorized)?;
        let state: SecurityKeyAuthentication =
            serde_json::from_slice(&state_bytes).map_err(|_| AppError::Internal)?;

        let webauthn = build_webauthn(&rp_info)?;
        let result = webauthn
            .finish_securitykey_authentication(&assertion, &state)
            .map_err(|_| AppError::Unauthorized)?;

        crate::server::extended_auth_queries::update_passkey_sign_count(
            &pool,
            cred_id,
            result.counter() as i64,
        )
        .await?;

        let (token, _csrf) =
            crate::server::auth_queries::create_session(&pool, &passkey.user_id, None, None)
                .await?;

        if let Some(res_opts) = use_context::<leptos_axum::ResponseOptions>() {
            let secure = crate::server::auth_queries::is_secure_request().await;
            let cookie = crate::server::auth_queries::build_session_cookie(&token, secure);
            res_opts.insert_header(
                axum::http::HeaderName::from_static("set-cookie"),
                axum::http::HeaderValue::from_str(&cookie)
                    .unwrap_or_else(|_| axum::http::HeaderValue::from_static("")),
            );
        }

        crate::server::auth_queries::audit_write_query(
            &pool,
            Some(user_id),
            "passkey_login",
            None,
            None,
            None,
            "success",
        )
        .await;

        let sessions =
            crate::server::auth_queries::list_sessions_query(&pool, &passkey.user_id).await?;
        let summary = sessions
            .into_iter()
            .find(|s| s.id == token)
            .ok_or(AppError::Internal)?;
        Ok(summary)
    }
    #[cfg(not(feature = "ssr"))]
    {
        let _ = resp;
        Err(ServerFnError::from(AppError::Internal))
    }
}

/// List the current user's registered passkeys (for account UI).
#[leptos::server]
pub async fn list_passkeys() -> Result<Vec<PasskeySummary>, ServerFnError<AppError>> {
    #[cfg(feature = "ssr")]
    {
        use axum::Extension;
        let pool = leptos_axum::extract::<Extension<sqlx::SqlitePool>>()
            .await
            .map_err(|_| AppError::Internal)?
            .0;
        let info = crate::server::auth_helper::require_session(&pool).await?;
        let passkeys = crate::server::extended_auth_queries::list_passkeys_for_user(
            &pool,
            &info.user_id.to_string(),
        )
        .await?;
        Ok(passkeys
            .into_iter()
            .map(|p| PasskeySummary {
                id: p.id,
                created_at: p.created_at,
            })
            .collect())
    }
    #[cfg(not(feature = "ssr"))]
    {
        Err(ServerFnError::from(AppError::Unauthorized))
    }
}

/// Delete a registered passkey by ID. Requires session + CSRF.
#[leptos::server]
pub async fn delete_passkey(id: String) -> Result<(), ServerFnError<AppError>> {
    #[cfg(feature = "ssr")]
    {
        use axum::Extension;
        let pool = leptos_axum::extract::<Extension<sqlx::SqlitePool>>()
            .await
            .map_err(|_| AppError::Internal)?
            .0;
        let info = crate::server::auth_helper::require_session_csrf(&pool).await?;
        crate::server::extended_auth_queries::delete_passkey(&pool, &id, &info.user_id.to_string())
            .await?;
        crate::server::auth_queries::audit_write_query(
            &pool,
            Some(info.user_id),
            "passkey_delete",
            Some(&id),
            None,
            None,
            "success",
        )
        .await;
        Ok(())
    }
    #[cfg(not(feature = "ssr"))]
    {
        let _ = id;
        Err(ServerFnError::from(AppError::Unauthorized))
    }
}

// --- API Tokens ---

#[leptos::server]
pub async fn create_api_token(
    input: ApiTokenInput,
) -> Result<ApiTokenSecret, ServerFnError<AppError>> {
    #[cfg(feature = "ssr")]
    {
        use axum::Extension;
        let pool = leptos_axum::extract::<Extension<sqlx::SqlitePool>>()
            .await
            .map_err(|_| AppError::Internal)?
            .0;
        let info = crate::server::auth_helper::require_session_csrf(&pool).await?;
        let server_key = leptos_axum::extract::<Extension<ServerKey>>()
            .await
            .map_err(|_| AppError::Internal)?
            .0;
        let secret = crate::server::extended_auth_queries::create_api_token_query(
            &pool,
            &info.user_id.to_string(),
            &input,
            &server_key.0,
        )
        .await?;
        crate::server::auth_queries::audit_write_query(
            &pool,
            Some(info.user_id),
            "api_token_create",
            Some(&secret.id.to_string()),
            None,
            None,
            "success",
        )
        .await;
        Ok(secret)
    }
    #[cfg(not(feature = "ssr"))]
    {
        let _ = input;
        Err(ServerFnError::from(AppError::Unauthorized))
    }
}

#[leptos::server]
pub async fn list_api_tokens() -> Result<Vec<ApiTokenSummary>, ServerFnError<AppError>> {
    #[cfg(feature = "ssr")]
    {
        use axum::Extension;
        let pool = leptos_axum::extract::<Extension<sqlx::SqlitePool>>()
            .await
            .map_err(|_| AppError::Internal)?
            .0;
        let info = crate::server::auth_helper::require_session(&pool).await?;
        crate::server::extended_auth_queries::list_api_tokens_query(
            &pool,
            &info.user_id.to_string(),
        )
        .await
        .map_err(ServerFnError::from)
    }
    #[cfg(not(feature = "ssr"))]
    {
        Err(ServerFnError::from(AppError::Unauthorized))
    }
}

#[leptos::server]
pub async fn revoke_api_token(id: Uuid) -> Result<(), ServerFnError<AppError>> {
    #[cfg(feature = "ssr")]
    {
        use axum::Extension;
        let pool = leptos_axum::extract::<Extension<sqlx::SqlitePool>>()
            .await
            .map_err(|_| AppError::Internal)?
            .0;
        let info = crate::server::auth_helper::require_session_csrf(&pool).await?;
        crate::server::extended_auth_queries::revoke_api_token_query(
            &pool,
            id,
            &info.user_id.to_string(),
        )
        .await?;
        crate::server::auth_queries::audit_write_query(
            &pool,
            Some(info.user_id),
            "api_token_revoke",
            Some(&id.to_string()),
            None,
            None,
            "success",
        )
        .await;
        Ok(())
    }
    #[cfg(not(feature = "ssr"))]
    {
        let _ = id;
        Err(ServerFnError::from(AppError::Unauthorized))
    }
}
