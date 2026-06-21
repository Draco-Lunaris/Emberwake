use leptos::prelude::*;
use leptos_meta::{Style, provide_meta_context};
use leptos_router::components::{Redirect, Route, Router, Routes};
use leptos_router::path;

pub mod components;
pub mod domain;
pub mod error;
pub mod server;

use components::auth::{AccountPage, AdminPage, LoginPage, SetupPage};
use components::dashboard::Dashboard;
use components::search::SearchIsland;
use components::settings::SettingsPage;
use domain::{DashboardView, Role, SetupState};

/// Root application component — renders Router content and theme styles.
/// The HTML document shell (DOCTYPE, <head>, hydration scripts) is provided by
/// the `shell()` function in `server/src/main.rs`. This component only renders
/// the router routes and injects the active theme as CSS custom properties via
/// `<Style>` from leptos_meta (injected into <head> during SSR — no flash).
#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();

    let theme = Resource::new(
        || (),
        |_| async { server::settings::get_active_theme().await.unwrap_or(None) },
    );

    view! {
        <Suspense fallback=|| view! { <p>"Loading..."</p> }>
            {move || {
                theme.get().map(|t| {
                    let css = match t {
                        Some(theme) => {
                            let mut css = format_theme_css(&theme);
                            if let Some(ref custom) = theme.custom_css {
                                css.push_str(custom);
                            }
                            css
                        }
                        None => {
                            "@media (prefers-color-scheme: dark) { :root { --bg: #1a1a2e; --surface: #16213e; --text: #e2e8f0; --text-muted: #94a3b8; --accent: #3b82f6; --accent-text: #ffffff; --border: #334155; --radius: 8px; --spacing: 16px; --font: system-ui, sans-serif; } } @media (prefers-color-scheme: light) { :root { --bg: #ffffff; --surface: #f5f5f5; --text: #1a1a1a; --text-muted: #6b7280; --accent: #3b82f6; --accent-text: #ffffff; --border: #e5e7eb; --radius: 8px; --spacing: 16px; --font: system-ui, sans-serif; } }".to_string()
                        }
                    };
                    view! {
                        <Style>{css}</Style>
                    }
                })
            }}
        </Suspense>
        <Router>
            <main>
                <Routes fallback=|| "Not found.">
                    <Route path=path!("/") view=HomePage />
                    <Route path=path!("/setup") view=SetupPage />
                    <Route path=path!("/login") view=LoginPage />
                    <Route path=path!("/account") view=AccountPage />
                    <Route path=path!("/admin") view=AdminPage />
                    <Route path=path!("/settings") view=SettingsPage />
                </Routes>
            </main>
        </Router>
    }
}

/// Format theme design tokens as CSS custom properties.
fn format_theme_css(theme: &domain::Theme) -> String {
    let t = &theme.tokens;
    let mut css = String::from(":root {\n");
    if let Some(ref v) = t.bg {
        css.push_str(&format!("  --bg: {};\n", v));
    }
    if let Some(ref v) = t.surface {
        css.push_str(&format!("  --surface: {};\n", v));
    }
    if let Some(ref v) = t.text {
        css.push_str(&format!("  --text: {};\n", v));
    }
    if let Some(ref v) = t.text_muted {
        css.push_str(&format!("  --text-muted: {};\n", v));
    }
    if let Some(ref v) = t.accent {
        css.push_str(&format!("  --accent: {};\n", v));
    }
    if let Some(ref v) = t.accent_text {
        css.push_str(&format!("  --accent-text: {};\n", v));
    }
    if let Some(ref v) = t.border {
        css.push_str(&format!("  --border: {};\n", v));
    }
    if let Some(ref v) = t.radius {
        css.push_str(&format!("  --radius: {};\n", v));
    }
    if let Some(ref v) = t.spacing {
        css.push_str(&format!("  --spacing: {};\n", v));
    }
    if let Some(ref v) = t.font {
        css.push_str(&format!("  --font: {};\n", v));
    }
    css.push_str("}\n");
    css
}

/// Home page — SSR-rendered dashboard with nav, search, and pinned content.
/// On first run (setup open), redirects to /setup. When setup is complete but
/// the user is not authenticated, shows a login link alongside public content.
#[component]
fn HomePage() -> impl IntoView {
    let setup = Resource::new(
        || (),
        |_| async {
            server::auth::setup_status()
                .await
                .unwrap_or(SetupState::Complete)
        },
    );
    let user = Resource::new(
        || (),
        |_| async { server::auth::current_user().await.unwrap_or(None) },
    );
    let dashboard = Resource::new(
        || (),
        |_| async {
            server::content_read::list_dashboard()
                .await
                .unwrap_or_default()
        },
    );
    let weather = Resource::new(
        || (),
        |_| async { server::weather_read::get_weather().await.unwrap_or(None) },
    );

    let logout = move |_| {
        leptos::task::spawn_local(async move {
            let _ = server::auth::logout().await;
            leptos_router::hooks::use_navigate()("/login", Default::default());
        });
    };

    view! {
        <Suspense fallback=|| view! { <p>"Loading..."</p> }>
            {move || {
                setup.get().map(|state| {
                    match state {
                        SetupState::Open => view! {
                            <Redirect path="/setup" />
                        }.into_any(),
                        SetupState::Complete => view! {
                            <h1>"Emberwake"</h1>
                            <nav>
                                {move || {
                                    user.get().map(|u| {
                                        match u {
                                            Some(u) => view! {
                                                <a href="/settings">"Settings"</a>
                                                <a href="/account">"Account"</a>
                                                {if u.role == Role::Admin {
                                                    view! { <a href="/admin">"Admin"</a> }.into_any()
                                                } else {
                                                    ().into_any()
                                                }}
                                                <button on:click=logout>"Logout"</button>
                                            }.into_any(),
                                            None => view! {
                                                <a href="/login">"Login"</a>
                                            }.into_any()
                                        }
                                    })
                                }}
                            </nav>
                            <Suspense fallback=|| view! { <p>"Loading..."</p> }>
                                {move || {
                                    dashboard
                                        .get()
                                        .map(|data: DashboardView| {
                                            let mut items: Vec<(String, String)> = data
                                                .pinned_services
                                                .iter()
                                                .map(|s| (s.name.clone(), s.url.clone()))
                                                .collect();
                                            for group in &data.pinned_categories {
                                                for bm in &group.bookmarks {
                                                    items.push((bm.name.clone(), bm.url.clone()));
                                                }
                                            }
                                            view! { <SearchIsland items providers=vec![] /> }
                                        })
                                }}
                            </Suspense>
                            {move || {
                                user.get().map(|u| {
                                    if u.is_some() {
                                        view! {
                                            <div class="add-content">
                                                <a href="/settings">"Add Service"</a>
                                                <a href="/settings">"Add Bookmark"</a>
                                                <a href="/settings">"Add Category"</a>
                                            </div>
                                        }.into_any()
                                    } else {
                                        ().into_any()
                                    }
                                })
                            }}
                            <Suspense fallback=|| view! { <p>"Loading..."</p> }>
                                {move || {
                                    weather
                                        .get()
                                        .map(|w| view! { <components::dashboard::weather_widget::WeatherWidget initial=w /> })
                                }}
                            </Suspense>
                            <Suspense fallback=|| view! { <p>"Loading..."</p> }>
                                {move || {
                                    dashboard
                                        .get()
                                        .map(|data: DashboardView| {
                                            view! { <Dashboard data /> }
                                        })
                                }}
                            </Suspense>
                        }.into_any(),
                    }
                })
            }}
        </Suspense>
    }
}

/// WASM hydrate entry point — called by the Leptos hydration script as `mod.hydrate()`.
/// Only compiled when the `hydrate` feature is enabled (WASM build).
#[cfg(feature = "hydrate")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    use leptos::mount::hydrate_body;
    hydrate_body(App);
}
