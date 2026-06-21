//! Admin user management page — list/create/deactivate users.

use crate::domain::{NewUserInput, Role, UserSummary};
use leptos::prelude::*;

#[component]
pub fn AdminPage() -> impl IntoView {
    let users = Resource::new(
        || (),
        |_| async { crate::server::auth::list_users().await.unwrap_or_default() },
    );

    let new_username = RwSignal::new(String::new());
    let new_password = RwSignal::new(String::new());
    let new_email = RwSignal::new(String::new());

    let create = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        let input = NewUserInput {
            username: new_username.get(),
            password: new_password.get(),
            email: if new_email.get().is_empty() {
                None
            } else {
                Some(new_email.get())
            },
            role: Role::User,
        };
        leptos::task::spawn_local(async move {
            let _ = crate::server::auth::create_user(input).await;
            users.refetch();
        });
    };

    view! {
        <div class="admin-page">
            <h1>"User Management"</h1>
            <h2>"Create User"</h2>
            <form on:submit=create>
                <input type="text" placeholder="username" bind:value=new_username required />
                <input type="password" placeholder="password" bind:value=new_password required minlength="8" />
                <input type="email" placeholder="email (optional)" bind:value=new_email />
                <button type="submit">"Create User"</button>
            </form>
            <h2>"Users"</h2>
            <Suspense fallback=|| view! { <p>"Loading..."</p> }>
                {move || users.get().map(|u: Vec<UserSummary>| {
                    u.iter().map(|user| {
                        let uid = user.id;
                        let username = user.username.clone();
                        let role = user.role.to_string();
                        let active = if user.is_active { "yes" } else { "no" };
                        let created = user.created_at.clone();
                        view! {
                            <div class="user-row">
                                <span>{username}</span>
                                <span>{role}</span>
                                <span>{active}</span>
                                <span>{created}</span>
                                {if user.is_active {
                                    view! {
                                        <button on:click=move |_| {
                                            leptos::task::spawn_local(async move {
                                                let _ = crate::server::auth::deactivate_user(uid).await;
                                                users.refetch();
                                            });
                                        }>"Deactivate"</button>
                                    }.into_any()
                                } else {
                                    "-".into_any()
                                }}
                            </div>
                        }.into_any()
                    }).collect::<Vec<_>>()
                })}
            </Suspense>
        </div>
    }
}
