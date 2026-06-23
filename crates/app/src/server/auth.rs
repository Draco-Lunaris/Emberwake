//! Auth server functions: setup, login, logout, current_user, session/user management.
//! Every auth event is audited. Auth fails closed — no session = rejection.

use leptos::server_fn::ServerFnError;
use uuid::Uuid;

use crate::domain::{
    AdminSetupInput, LoginInput, NewUserInput, SessionSummary, SetupState, UserPatch, UserSummary,
};
use crate::error::AppError;

#[cfg(feature = "ssr")]
use crate::domain::Role;

/// Argon2 params wrapper for Axum Extension extraction.
#[derive(Clone)]
pub struct Argon2Params {
    pub m_cost: u32,
    pub t_cost: u32,
    pub p_cost: u32,
}

#[cfg(feature = "ssr")]
async fn get_argon2_params() -> (u32, u32, u32) {
    use axum::Extension;
    leptos_axum::extract::<Extension<Argon2Params>>()
        .await
        .map(|p| (p.0.m_cost, p.0.t_cost, p.0.p_cost))
        .unwrap_or((32 * 1024, 3, 1))
}

#[leptos::server]
pub async fn setup_status() -> Result<SetupState, ServerFnError<AppError>> {
    #[cfg(feature = "ssr")]
    {
        use axum::Extension;
        let pool = leptos_axum::extract::<Extension<sqlx::SqlitePool>>()
            .await
            .map_err(|_| AppError::Internal)?
            .0;
        crate::server::auth_queries::setup_status_query(&pool)
            .await
            .map_err(ServerFnError::from)
    }
    #[cfg(not(feature = "ssr"))]
    {
        Err(ServerFnError::from(AppError::Internal))
    }
}

#[leptos::server]
pub async fn complete_setup(input: AdminSetupInput) -> Result<(), ServerFnError<AppError>> {
    #[cfg(feature = "ssr")]
    {
        use axum::Extension;
        let pool = leptos_axum::extract::<Extension<sqlx::SqlitePool>>()
            .await
            .map_err(|_| AppError::Internal)?
            .0;
        let (m, t, p) = get_argon2_params().await;
        crate::server::auth_queries::complete_setup_query(
            &pool,
            &input.username,
            &input.password,
            input.email.as_deref(),
            m,
            t,
            p,
        )
        .await?;
        crate::server::auth_queries::audit_write_query(
            &pool,
            None,
            "setup_complete",
            Some(&input.username),
            None,
            None,
            "success",
        )
        .await;
        Ok(())
    }
    #[cfg(not(feature = "ssr"))]
    {
        let _ = input;
        Err(ServerFnError::from(AppError::Unauthorized))
    }
}

#[leptos::server]
pub async fn login(input: LoginInput) -> Result<SessionSummary, ServerFnError<AppError>> {
    #[cfg(feature = "ssr")]
    {
        use axum::Extension;
        use axum::http::HeaderMap;
        use leptos::prelude::use_context;

        let pool = leptos_axum::extract::<Extension<sqlx::SqlitePool>>()
            .await
            .map_err(|_| AppError::Internal)?
            .0;

        let headers = leptos_axum::extract::<HeaderMap>().await.ok();
        let user_agent = headers
            .as_ref()
            .and_then(|h| h.get("user-agent"))
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());
        let ip = headers
            .as_ref()
            .and_then(|h| h.get("x-forwarded-for"))
            .and_then(|v| v.to_str().ok())
            .map(|s| s.split(',').next().unwrap_or("").trim().to_string());

        match crate::server::auth_queries::login_query(
            &pool,
            &input,
            user_agent.as_deref(),
            ip.as_deref(),
        )
        .await
        {
            Ok((token, csrf, user_id)) => {
                let res_opts = use_context::<leptos_axum::ResponseOptions>();
                if res_opts.is_none() {
                    #[cfg(feature = "ssr")]
                    tracing::warn!("ResponseOptions not found in context — cookie will not be set");
                }
                if let Some(res_opts) = res_opts {
                    let session_cookie =
                        crate::server::auth_queries::build_session_cookie(&token, false);
                    res_opts.insert_header(
                        axum::http::HeaderName::from_static("set-cookie"),
                        axum::http::HeaderValue::from_str(&session_cookie)
                            .unwrap_or_else(|_| axum::http::HeaderValue::from_static("")),
                    );
                    let csrf_cookie = crate::server::auth_queries::build_csrf_cookie(&csrf, false);
                    res_opts.append_header(
                        axum::http::HeaderName::from_static("set-cookie"),
                        axum::http::HeaderValue::from_str(&csrf_cookie)
                            .unwrap_or_else(|_| axum::http::HeaderValue::from_static("")),
                    );
                }
                crate::server::auth_queries::audit_write_query(
                    &pool,
                    Some(user_id),
                    "login",
                    Some(&input.username),
                    ip.as_deref(),
                    user_agent.as_deref(),
                    "success",
                )
                .await;
                let sessions =
                    crate::server::auth_queries::list_sessions_query(&pool, &user_id.to_string())
                        .await?;
                let summary = sessions
                    .into_iter()
                    .find(|s| s.id == token)
                    .ok_or(AppError::Internal)?;
                Ok(summary)
            }
            Err(e) => {
                crate::server::auth_queries::audit_write_query(
                    &pool,
                    None,
                    "login_fail",
                    Some(&input.username),
                    ip.as_deref(),
                    user_agent.as_deref(),
                    "failure",
                )
                .await;
                Err(ServerFnError::from(e))
            }
        }
    }
    #[cfg(not(feature = "ssr"))]
    {
        let _ = input;
        Err(ServerFnError::from(AppError::Unauthorized))
    }
}

#[leptos::server]
pub async fn logout() -> Result<(), ServerFnError<AppError>> {
    #[cfg(feature = "ssr")]
    {
        use axum::Extension;
        use axum::http::HeaderMap;
        use leptos::prelude::use_context;

        let pool = leptos_axum::extract::<Extension<sqlx::SqlitePool>>()
            .await
            .map_err(|_| AppError::Internal)?
            .0;

        let headers = leptos_axum::extract::<HeaderMap>().await.ok();
        let cookie_header = headers
            .as_ref()
            .and_then(|h| h.get("cookie"))
            .and_then(|v| v.to_str().ok());
        let session_token = crate::server::auth_queries::parse_session_cookie(cookie_header);

        if let Some(token) = session_token
            && let Ok(Some(info)) = crate::server::auth_queries::lookup_session(&pool, &token).await
        {
            let _ = crate::server::auth_queries::delete_session(&pool, &token).await;
            crate::server::auth_queries::audit_write_query(
                &pool,
                Some(info.user_id),
                "logout",
                Some(&token),
                None,
                None,
                "success",
            )
            .await;
        }

        if let Some(res_opts) = use_context::<leptos_axum::ResponseOptions>() {
            let session_cookie = crate::server::auth_queries::build_clear_session_cookie(false);
            res_opts.insert_header(
                axum::http::HeaderName::from_static("set-cookie"),
                axum::http::HeaderValue::from_str(&session_cookie)
                    .unwrap_or_else(|_| axum::http::HeaderValue::from_static("")),
            );
            let csrf_cookie = crate::server::auth_queries::build_clear_csrf_cookie(false);
            res_opts.append_header(
                axum::http::HeaderName::from_static("set-cookie"),
                axum::http::HeaderValue::from_str(&csrf_cookie)
                    .unwrap_or_else(|_| axum::http::HeaderValue::from_static("")),
            );
        }
        Ok(())
    }
    #[cfg(not(feature = "ssr"))]
    {
        Err(ServerFnError::from(AppError::Unauthorized))
    }
}

#[leptos::server]
pub async fn current_user() -> Result<Option<UserSummary>, ServerFnError<AppError>> {
    #[cfg(feature = "ssr")]
    {
        use axum::Extension;
        use axum::http::HeaderMap;

        let pool = leptos_axum::extract::<Extension<sqlx::SqlitePool>>()
            .await
            .map_err(|_| AppError::Internal)?
            .0;

        let headers = leptos_axum::extract::<HeaderMap>().await.ok();
        let cookie_header = headers
            .as_ref()
            .and_then(|h| h.get("cookie"))
            .and_then(|v| v.to_str().ok());
        let session_token = crate::server::auth_queries::parse_session_cookie(cookie_header);

        match session_token {
            Some(token) => {
                let info = crate::server::auth_queries::lookup_session(&pool, &token).await?;
                match info {
                    Some(si) => {
                        let user =
                            crate::server::auth_queries::get_user_by_id(&pool, si.user_id).await?;
                        Ok(user)
                    }
                    None => Ok(None),
                }
            }
            None => Ok(None),
        }
    }
    #[cfg(not(feature = "ssr"))]
    {
        Err(ServerFnError::from(AppError::Internal))
    }
}

#[leptos::server]
pub async fn list_sessions() -> Result<Vec<SessionSummary>, ServerFnError<AppError>> {
    #[cfg(feature = "ssr")]
    {
        use axum::Extension;
        let pool = leptos_axum::extract::<Extension<sqlx::SqlitePool>>()
            .await
            .map_err(|_| AppError::Internal)?
            .0;
        let info = crate::server::auth_helper::require_session(&pool).await?;
        crate::server::auth_queries::list_sessions_query(&pool, &info.user_id.to_string())
            .await
            .map_err(ServerFnError::from)
    }
    #[cfg(not(feature = "ssr"))]
    {
        Err(ServerFnError::from(AppError::Unauthorized))
    }
}

#[leptos::server]
pub async fn revoke_session(id: String) -> Result<(), ServerFnError<AppError>> {
    #[cfg(feature = "ssr")]
    {
        use axum::Extension;
        let pool = leptos_axum::extract::<Extension<sqlx::SqlitePool>>()
            .await
            .map_err(|_| AppError::Internal)?
            .0;
        let info = crate::server::auth_helper::require_session(&pool).await?;

        let own_sessions =
            crate::server::auth_queries::list_sessions_query(&pool, &info.user_id.to_string())
                .await?;
        let is_own = own_sessions.iter().any(|s| s.id == id);

        if !is_own && info.role != Role::Admin {
            crate::server::auth_queries::audit_write_query(
                &pool,
                Some(info.user_id),
                "perm_denied",
                Some(&format!("revoke_session:{id}")),
                None,
                None,
                "failure",
            )
            .await;
            return Err(ServerFnError::from(AppError::Forbidden));
        }

        let _ = crate::server::auth_queries::revoke_session_query(&pool, &id).await;
        crate::server::auth_queries::audit_write_query(
            &pool,
            Some(info.user_id),
            "session_revoke",
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

#[leptos::server]
pub async fn revoke_all_other_sessions() -> Result<(), ServerFnError<AppError>> {
    #[cfg(feature = "ssr")]
    {
        use axum::Extension;
        use axum::http::HeaderMap;
        let pool = leptos_axum::extract::<Extension<sqlx::SqlitePool>>()
            .await
            .map_err(|_| AppError::Internal)?
            .0;
        let info = crate::server::auth_helper::require_session(&pool).await?;

        let headers = leptos_axum::extract::<HeaderMap>().await.ok();
        let cookie_header = headers
            .as_ref()
            .and_then(|h| h.get("cookie"))
            .and_then(|v| v.to_str().ok());
        let current_token =
            crate::server::auth_queries::parse_session_cookie(cookie_header).unwrap_or_default();

        let _ = crate::server::auth_queries::revoke_all_other_sessions_query(
            &pool,
            &info.user_id.to_string(),
            &current_token,
        )
        .await;
        crate::server::auth_queries::audit_write_query(
            &pool,
            Some(info.user_id),
            "session_revoke",
            Some("all_other"),
            None,
            None,
            "success",
        )
        .await;
        Ok(())
    }
    #[cfg(not(feature = "ssr"))]
    {
        Err(ServerFnError::from(AppError::Unauthorized))
    }
}

#[leptos::server]
pub async fn list_users() -> Result<Vec<UserSummary>, ServerFnError<AppError>> {
    #[cfg(feature = "ssr")]
    {
        use axum::Extension;
        let pool = leptos_axum::extract::<Extension<sqlx::SqlitePool>>()
            .await
            .map_err(|_| AppError::Internal)?
            .0;
        let info = crate::server::auth_helper::require_session(&pool).await?;
        if info.role != Role::Admin {
            crate::server::auth_queries::audit_write_query(
                &pool,
                Some(info.user_id),
                "perm_denied",
                Some("list_users"),
                None,
                None,
                "failure",
            )
            .await;
            return Err(ServerFnError::from(AppError::Forbidden));
        }
        crate::server::auth_queries::list_users_query(&pool)
            .await
            .map_err(ServerFnError::from)
    }
    #[cfg(not(feature = "ssr"))]
    {
        Err(ServerFnError::from(AppError::Unauthorized))
    }
}

#[leptos::server]
pub async fn create_user(input: NewUserInput) -> Result<UserSummary, ServerFnError<AppError>> {
    #[cfg(feature = "ssr")]
    {
        use axum::Extension;
        let pool = leptos_axum::extract::<Extension<sqlx::SqlitePool>>()
            .await
            .map_err(|_| AppError::Internal)?
            .0;
        let info = crate::server::auth_helper::require_session(&pool).await?;
        if info.role != Role::Admin {
            return Err(ServerFnError::from(AppError::Forbidden));
        }
        let (m, t, p) = get_argon2_params().await;
        let user = crate::server::auth_queries::create_user_query(&pool, &input, m, t, p).await?;
        crate::server::auth_queries::audit_write_query(
            &pool,
            Some(info.user_id),
            "user_create",
            Some(&user.id.to_string()),
            None,
            None,
            "success",
        )
        .await;
        Ok(user)
    }
    #[cfg(not(feature = "ssr"))]
    {
        let _ = input;
        Err(ServerFnError::from(AppError::Unauthorized))
    }
}

#[leptos::server]
pub async fn update_user(
    id: Uuid,
    patch: UserPatch,
) -> Result<UserSummary, ServerFnError<AppError>> {
    #[cfg(feature = "ssr")]
    {
        use axum::Extension;
        let pool = leptos_axum::extract::<Extension<sqlx::SqlitePool>>()
            .await
            .map_err(|_| AppError::Internal)?
            .0;
        let info = crate::server::auth_helper::require_session(&pool).await?;
        if info.role != Role::Admin {
            return Err(ServerFnError::from(AppError::Forbidden));
        }
        let (m, t, p) = get_argon2_params().await;
        let user =
            crate::server::auth_queries::update_user_query(&pool, id, &patch, m, t, p).await?;
        if patch.role.is_some() {
            let _ = sqlx::query("DELETE FROM sessions WHERE user_id = ?")
                .bind(id.to_string())
                .execute(&pool)
                .await;
        }
        Ok(user)
    }
    #[cfg(not(feature = "ssr"))]
    {
        let _ = (id, patch);
        Err(ServerFnError::from(AppError::Unauthorized))
    }
}

#[leptos::server]
pub async fn deactivate_user(id: Uuid) -> Result<(), ServerFnError<AppError>> {
    #[cfg(feature = "ssr")]
    {
        use axum::Extension;
        let pool = leptos_axum::extract::<Extension<sqlx::SqlitePool>>()
            .await
            .map_err(|_| AppError::Internal)?
            .0;
        let info = crate::server::auth_helper::require_session(&pool).await?;
        if info.role != Role::Admin {
            return Err(ServerFnError::from(AppError::Forbidden));
        }
        if info.user_id == id {
            return Err(ServerFnError::from(AppError::Conflict(
                "cannot deactivate self".into(),
            )));
        }
        crate::server::auth_queries::deactivate_user_query(&pool, id).await?;
        crate::server::auth_queries::audit_write_query(
            &pool,
            Some(info.user_id),
            "user_deactivate",
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
