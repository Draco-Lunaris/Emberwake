#![recursion_limit = "256"]

use leptos::prelude::*;
use leptos_meta::{Meta, Style, provide_meta_context};
use leptos_router::components::{Redirect, Route, Router, Routes};
use leptos_router::path;

pub mod components;
pub mod domain;
pub mod error;
pub mod server;

use components::auth::{AccountPage, AdminPage, LoginPage, SetupPage};
use components::dashboard::Dashboard;
use components::editors::{BookmarkEditPage, CategoryEditPage, EditPage, ServiceEditPage};
use components::search::SearchIsland;
use components::settings::SettingsPage;
use domain::{DashboardView, SetupState};

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
        <Meta
            http_equiv="Content-Security-Policy"
            content=move || {
                leptos::nonce::use_nonce()
                    .map(|nonce| {
                        format!(
                            "default-src 'self'; \
                             script-src 'self' 'nonce-{nonce}' 'wasm-unsafe-eval'; \
                             style-src 'self' 'nonce-{nonce}'; \
                             img-src 'self' data: https:; \
                             font-src 'self' https://fonts.googleapis.com https://fonts.gstatic.com; \
                             connect-src 'self'; \
                             frame-ancestors 'none'; \
                             base-uri 'self'; \
                             form-action 'self'"
                        )
                    })
                    .unwrap_or_default()
            }
        />
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
                    <Route path=path!("/edit") view=EditPage />
                    <Route path=path!("/edit/service") view=ServiceEditPage />
                    <Route path=path!("/edit/bookmark") view=BookmarkEditPage />
                    <Route path=path!("/edit/category") view=CategoryEditPage />
                    <Route path=path!("/settings") view=SettingsPage />
                </Routes>
            </main>
        </Router>
    }
}

/// Format theme design tokens as CSS custom properties.
fn format_theme_css(theme: &domain::Theme) -> String {
    let t = &theme.tokens;
    let mut vars: Vec<(&str, &str)> = Vec::new();

    macro_rules! push_token {
        ($css_var:expr, $field:expr) => {
            if let Some(ref v) = $field {
                vars.push(($css_var, v.as_str()));
            }
        };
    }

    push_token!("--bg", t.bg);
    push_token!("--bg-deep", t.bg_deep);
    push_token!("--surface", t.surface);
    push_token!("--surface-raised", t.surface_raised);
    push_token!("--text", t.text);
    push_token!("--text-muted", t.text_muted);
    push_token!("--text-faint", t.text_faint);
    push_token!("--accent", t.accent);
    push_token!("--accent-text", t.accent_text);
    push_token!("--accent-alt", t.accent_alt);
    push_token!("--border", t.border);
    push_token!("--radius", t.radius);
    push_token!("--radius-sm", t.radius_sm);
    push_token!("--radius-lg", t.radius_lg);
    push_token!("--spacing", t.spacing);
    push_token!("--font", t.font);
    push_token!("--font-mono", t.font_mono);

    if vars.is_empty() {
        return String::new();
    }

    let mut css = String::from(":root {\n");
    for (name, val) in &vars {
        css.push_str(&format!("  {}: {};\n", name, val));
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
    let search_providers = Resource::new(
        || (),
        |_| async {
            server::content_read::get_search_providers()
                .await
                .unwrap_or_default()
        },
    );

    view! {
        <Suspense fallback=|| view! { <p>"Loading..."</p> }>
            {move || {
                setup.get().map(|state| {
                    match state {
                        SetupState::Open => view! {
                            <Redirect path="/setup" />
                        }.into_any(),
                        SetupState::Complete => view! {
                            <components::Navbar />
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
                                            let providers = search_providers.get().unwrap_or_default().providers;
                                            view! { <SearchIsland items providers /> }
                                        })
                                }}
                            </Suspense>
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
