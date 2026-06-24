//! Account page — session list, OIDC identities, passkeys, and API token management.

use crate::components::Navbar;
use crate::domain::{
    ApiTokenInput, ApiTokenSecret, ApiTokenSummary, ExternalIdentity, PasskeySummary,
    SessionSummary,
};

#[cfg(all(feature = "hydrate", target_arch = "wasm32"))]
use crate::domain::RegisterResponse;
use leptos::prelude::*;

/// Call navigator.credentials.create() with the server-provided WebAuthn challenge,
/// extract the credential fields, base64url-encode the ArrayBuffers, and return a
/// RegisterResponse ready to send back to the server.
#[cfg(all(feature = "hydrate", target_arch = "wasm32"))]
async fn webauthn_create(challenge: serde_json::Value) -> Result<RegisterResponse, String> {
    use base64::Engine;
    use base64::engine::general_purpose::URL_SAFE_NO_PAD;
    use wasm_bindgen::JsCast;
    use wasm_bindgen_futures::JsFuture;

    let window = web_sys::window().ok_or("no window")?;
    let challenge_str = serde_json::to_string(&challenge).map_err(|e| format!("serialize: {e}"))?;
    let public_key = js_sys::JSON::parse(&challenge_str).map_err(|e| format!("parse: {e:?}"))?;

    let opts = js_sys::Object::new();
    js_sys::Reflect::set(&opts, &"publicKey".into(), &public_key)
        .map_err(|_| "failed to set publicKey".to_string())?;
    let create_options: web_sys::CredentialCreationOptions = opts.unchecked_into();

    let credentials = window.navigator().credentials();
    let result = JsFuture::from({
        let create_fn = js_sys::Reflect::get(&credentials, &"create".into()).unwrap();
        let create_fn: js_sys::Function = create_fn.unchecked_into();
        let promise = create_fn.call1(&credentials, &create_options).unwrap();
        promise.unchecked_into::<js_sys::Promise>()
    })
    .await
    .map_err(|e| format!("credentials.create failed: {e:?}"))?;

    let pk_cred: web_sys::PublicKeyCredential = result.unchecked_into();

    let raw_id = js_sys::Uint8Array::new(&pk_cred.raw_id()).to_vec();
    let raw_id_b64 = URL_SAFE_NO_PAD.encode(&raw_id);

    let response = pk_cred.response();
    let att_response: web_sys::AuthenticatorAttestationResponse = response.unchecked_into();
    let att_obj = js_sys::Uint8Array::new(&att_response.attestation_object()).to_vec();
    let att_obj_b64 = URL_SAFE_NO_PAD.encode(&att_obj);

    let client_data = js_sys::Uint8Array::new(&att_response.client_data_json()).to_vec();
    let client_data_b64 = URL_SAFE_NO_PAD.encode(&client_data);

    let id = pk_cred.id();

    let credential = serde_json::json!({
        "id": id,
        "rawId": raw_id_b64,
        "type": "public-key",
        "response": {
            "attestationObject": att_obj_b64,
            "clientDataJSON": client_data_b64,
        },
    });

    Ok(RegisterResponse { credential })
}

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

    let passkeys = Resource::new(
        || (),
        |_| async {
            crate::server::extended_auth::list_passkeys()
                .await
                .unwrap_or_default()
        },
    );

    let (passkey_status, set_passkey_status) = signal(Option::<String>::None);

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
        <Navbar />
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
                    set_passkey_status.set(Some("Starting registration…".to_string()));
                    match crate::server::extended_auth::passkey_register_begin().await {
                        Ok(opts) => {
                            #[cfg(all(feature = "hydrate", target_arch = "wasm32"))]
                            {
                                match webauthn_create(opts.challenge).await {
                                    Ok(resp) => {
                                        match crate::server::extended_auth::passkey_register_finish(resp).await {
                                            Ok(_) => {
                                                set_passkey_status.set(Some("✓ Passkey registered".to_string()));
                                                passkeys.refetch();
                                            }
                                            Err(e) => {
                                                set_passkey_status.set(Some(format!("Registration failed: {e}")));
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        set_passkey_status.set(Some(format!("WebAuthn error: {e}")));
                                    }
                                }
                            }
                            #[cfg(not(all(feature = "hydrate", target_arch = "wasm32")))]
                            {
                                let _ = opts;
                                set_passkey_status.set(Some("Passkey registration requires a browser.".to_string()));
                            }
                        }
                        Err(e) => {
                            set_passkey_status.set(Some(format!("Error: {e}")));
                        }
                    }
                });
            }>"Register Passkey"</button>
            {move || passkey_status.get().map(|msg| view! {
                <p class=if msg.starts_with('✓') { "success-msg" } else { "text-muted" }>
                    {msg.clone()}
                </p>
            })}
            <Suspense fallback=|| view! { <p>"Loading..."</p> }>
                {move || passkeys.get().map(|list: Vec<PasskeySummary>| {
                    if list.is_empty() {
                        vec![view! { <p>"No registered passkeys."</p> }.into_any()]
                    } else {
                        list.iter().map(|pk| {
                            let id = pk.id.clone();
                            let created = pk.created_at.clone();
                            view! {
                                <div class="passkey-row">
                                    <span>{format!("Passkey (created: {created})")}</span>
                                    <button on:click=move |_| {
                                        let id = id.clone();
                                        leptos::task::spawn_local(async move {
                                            let _ = crate::server::extended_auth::delete_passkey(id).await;
                                            passkeys.refetch();
                                        });
                                    }>"Delete"</button>
                                </div>
                            }.into_any()
                        }).collect::<Vec<_>>()
                    }
                })}
            </Suspense>

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
