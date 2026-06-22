//! Shared navbar component — consistent navigation across all pages.

use leptos::prelude::*;
use leptos_router::components::A;

use crate::domain::Role;

/// Shared navbar — renders auth-aware navigation for all pages.
/// Calls `current_user()` to determine which links to show.
/// When authenticated: Add Service, Add Bookmark, Add Category, Settings, Account,
/// Admin (admin only), Logout. When not authenticated: Login link only.
#[component]
pub fn Navbar() -> impl IntoView {
    let user = Resource::new(
        || (),
        |_| async { crate::server::auth::current_user().await.unwrap_or(None) },
    );

    let logout = move |_| {
        leptos::task::spawn_local(async move {
            let _ = crate::server::auth::logout().await;
            // Full page reload ensures the cleared session cookie takes effect immediately.
            if let Some(window) = web_sys::window() {
                let _ = window.location().assign("/login");
            } else {
                leptos_router::hooks::use_navigate()("/login", Default::default());
            }
        });
    };

    view! {
        <nav class="navbar">
            <h1>"Emberwake"</h1>
            <Suspense fallback=|| view! { <p>"Loading..."</p> }>
                {move || {
                    user.get().map(|u| match u {
                        Some(u) => view! {
                            <A href="/edit/service">"Add Service"</A>
                            <A href="/edit/bookmark">"Add Bookmark"</A>
                            <A href="/edit/category">"Add Category"</A>
                            <A href="/settings">"Settings"</A>
                            <A href="/account">"Account"</A>
                            {if u.role == Role::Admin {
                                view! { <A href="/admin">"Admin"</A> }.into_any()
                            } else {
                                ().into_any()
                            }}
                            <button on:click=logout>"Logout"</button>
                        }.into_any(),
                        None => view! {
                            <A href="/login">"Login"</A>
                        }.into_any(),
                    })
                }}
            </Suspense>
        </nav>
    }
}
