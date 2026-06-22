//! Account page — session list, OIDC identities, passkeys, and API token management.

use crate::domain::{
    ApiTokenInput, ApiTokenSecret, ApiTokenSummary, ExternalIdentity, SessionSummary,
};
use leptos::prelude::*;
use leptos_router::components::A;

#[component]
pub fn AccountPage() -> impl IntoView {
    let sessions = Resource::new(
        || (),
        |_| async {
            crate::server::auth::list_sessions()
                .await
                .unwrap_or_default()
        },
    );

    let identities = Resource::new(
        || (),
        |_| async {
            crate::server::extended_auth::list_external_identities()
                .await
                .unwrap_or_default()
        },
    );

    let tokens = Resource::new(
        || (),
        |_| async {
            crate::server::extended_auth::list_api_tokens()
                .await
                .unwrap_or_default()
        },
    );

    let logout = move |_| {
        leptos::task::spawn_local(async move {
            let _ = crate::server::auth::logout().await;
            leptos_router::hooks::use_navigate()("/login", Default::default());
        });
    };

    let revoke_all = move |_| {
        leptos::task::spawn_local(async move {
            let _ = crate::server::auth::revoke_all_other_sessions().await;
            sessions.refetch();
        });
    };

    view! {
        <nav class="navbar">
            <h1>"Emberwake"</h1>
            <A href="/">"Dashboard"</A>
            <A href="/settings">"Settings"</A>
            <A href="/account">"Account"</A>
            <button on:click=logout>"Logout"</button>
        </nav>
        <div class="account-page">
            <h1>"Account"</h1>
            <button on:click=logout>"Sign Out"</button>
            <button on:click=revoke_all>"Revoke All Other Sessions"</button>

            // --- Active Sessions ---
            <h2>"Active Sessions"</h2>
            <Suspense fallback=|| view! { <p>"Loading..."</p> }>
                {move || sessions.get().map(|s: Vec<SessionSummary>| {
                    s.iter().map(|sess| {
                        let id = sess.id.clone();
                        let ip = sess.ip.clone().unwrap_or("-".to_string());
                        let ua = sess.user_agent.clone().unwrap_or("-".to_string());
                        let created = sess.created_at.clone();
                        let expires = sess.expires_at.clone();
                        view! {
                            <div class="session-row">
                                <span>{ip}</span>
                                <span>{ua}</span>
                                <span>{created}</span>
                                <span>{expires}</span>
                                <button on:click=move |_| {
                                    let id = id.clone();
                                    leptos::task::spawn_local(async move {
                                        let _ = crate::server::auth::revoke_session(id).await;
                                        sessions.refetch();
                                    });
                                }>"Revoke"</button>
                            </div>
                        }.into_any()
                    }).collect::<Vec<_>>()
                })}
            </Suspense>

            // --- OIDC Identities ---
            <h2>"Linked Identity Providers"</h2>
            <p><a href="/auth/oidc/login">"Link OIDC Provider"</a></p>
            <Suspense fallback=|| view! { <p>"Loading..."</p> }>
                {move || identities.get().map(|ids: Vec<ExternalIdentity>| {
                    if ids.is_empty() {
                        vec![view! { <p>"No linked identity providers."</p> }.into_any()]
                    } else {
                        ids.iter().map(|id| {
                            let id_uuid = id.id;
                            let provider = id.provider.clone();
                            view! {
                                <div class="identity-row">
                                    <span>{provider}</span>
                                    <button on:click=move |_| {
                                        let id_uuid = id_uuid;
                                        leptos::task::spawn_local(async move {
                                            let _ = crate::server::extended_auth::unlink_external_identity(id_uuid).await;
                                            identities.refetch();
                                        });
                                    }>"Unlink"</button>
                                </div>
                            }.into_any()
                        }).collect::<Vec<_>>()
                    }
                })}
            </Suspense>

            // --- Passkeys ---
            <h2>"Passkeys"</h2>
            <button on:click=move |_| {
                leptos::task::spawn_local(async move {
                    // Begin registration — in a real browser, this would call
                    // navigator.credentials.create() with the returned challenge,
                    // then call passkey_register_finish with the result.
                    // For now, this is the server-function entry point.
                    let _ = crate::server::extended_auth::passkey_register_begin().await;
                });
            }>"Register Passkey"</button>

            // --- API Tokens ---
            <h2>"API Tokens"</h2>
            <ApiTokenSection tokens=tokens />
        </div>
    }
}

/// API token management section with create/revoke and secret-once display.
#[component]
fn ApiTokenSection(tokens: Resource<Vec<ApiTokenSummary>>) -> impl IntoView {
    let (new_secret, set_new_secret) = signal(None::<ApiTokenSecret>);
    let token_name = RwSignal::new(String::new());
    let token_scopes = RwSignal::new("services:read".to_string());

    let create_token = move |_| {
        leptos::task::spawn_local(async move {
            let scopes: Vec<String> = token_scopes
                .get()
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            let input = ApiTokenInput {
                name: token_name.get(),
                scopes,
                expires_at: None,
            };
            if let Ok(secret) = crate::server::extended_auth::create_api_token(input).await {
                set_new_secret.set(Some(secret));
                token_name.set(String::new());
                tokens.refetch();
            }
        });
    };

    view! {
        <div class="api-token-section">
            // Create token form
            <div class="token-create-form">
                <input
                    type="text"
                    placeholder="Token name"
                    prop:value=token_name
                    on:input=move |ev| token_name.set(event_target_value(&ev))
                />
                <input
                    type="text"
                    placeholder="scopes (comma-separated)"
                    prop:value=token_scopes
                    on:input=move |ev| token_scopes.set(event_target_value(&ev))
                />
                <button on:click=create_token>"Create Token"</button>
            </div>

            // Show secret once on creation
            {move || new_secret.get().map(|s| {
                let secret = s.secret.clone();
                let name = s.name.clone();
                view! {
                    <div class="token-secret-display">
                        <p><strong>"Token created: " {name}</strong></p>
                        <p><code>{secret}</code></p>
                        <p>"Copy this token now — it will not be shown again."</p>
                        <button on:click=move |_| set_new_secret.set(None)>"Dismiss"</button>
                    </div>
                }.into_any()
            })}

            // Token list
            <Suspense fallback=|| view! { <p>"Loading..."</p> }>
                {move || tokens.get().map(|list: Vec<ApiTokenSummary>| {
                    if list.is_empty() {
                        vec![view! { <p>"No API tokens."</p> }.into_any()]
                    } else {
                        list.iter().map(|t| {
                            let id = t.id;
                            let name = t.name.clone();
                            let scopes = t.scopes.join(", ");
                            let created = t.created_at.clone();
                            let revoked = t.revoked_at.is_some();
                            let revoked_label = if revoked { " (revoked)" } else { "" };
                            view! {
                                <div class="token-row">
                                    <span>{name}</span>
                                    <span>{scopes}</span>
                                    <span>{created}</span>
                                    <span>{revoked_label}</span>
                                    {move || if !revoked {
                                        Some(view! {
                                            <button on:click=move |_| {
                                                let id = id;
                                                leptos::task::spawn_local(async move {
                                                    let _ = crate::server::extended_auth::revoke_api_token(id).await;
                                                    tokens.refetch();
                                                });
                                            }>"Revoke"</button>
                                        }.into_any())
                                    } else {
                                        None
                                    }}
                                </div>
                            }.into_any()
                        }).collect::<Vec<_>>()
                    }
                })}
            </Suspense>
        </div>
    }
}
