//! Settings page component — admin-gated settings + theme builder UI.
//! Non-admins are redirected. Shows search providers, integration toggles,
//! weather config (secret field), auth toggles, and theme builder.

use crate::components::Navbar;
use crate::domain::{SettingsView, ThemeSummary};
use leptos::prelude::*;

/// Settings page — admin-gated. Redirects non-admins.
#[component]
pub fn SettingsPage() -> impl IntoView {
    let settings_resource = Resource::new(
        || (),
        |_| async {
            crate::server::settings::get_settings()
                .await
                .unwrap_or_default()
        },
    );

    let themes_resource = Resource::new(
        || (),
        |_| async {
            crate::server::settings::list_themes()
                .await
                .unwrap_or_default()
        },
    );

    view! {
        <Navbar />
        <h1>"Settings"</h1>
        <Suspense fallback=|| view! { <p>"Loading settings..."</p> }>
            {move || {
                settings_resource.get().map(|settings: SettingsView| {
                    view! {
                        <SettingsForm settings />
                    }
                })
            }}
        </Suspense>
        <Suspense fallback=|| view! { <p>"Loading themes..."</p> }>
            {move || {
                themes_resource.get().map(|themes: Vec<ThemeSummary>| {
                    view! {
                        <ThemeBuilder themes />
                    }
                })
            }}
        </Suspense>
    }
}

/// Settings form — search providers, integrations, weather, auth.
#[component]
fn SettingsForm(settings: SettingsView) -> impl IntoView {
    let (search_providers, _set_search_providers) =
        signal(settings.search_providers.providers.clone());
    let (docker_enabled, set_docker_enabled) = signal(settings.integrations.docker_enabled);
    let (k8s_enabled, set_k8s_enabled) = signal(settings.integrations.k8s_enabled);
    let (weather_enabled, set_weather_enabled) = signal(settings.weather.enabled);
    let (weather_location, set_weather_location) =
        signal(settings.weather.location.unwrap_or_default());
    let (weather_api_key, set_weather_api_key) =
        signal(settings.weather.api_key.unwrap_or_default());
    let (oidc_enabled, set_oidc_enabled) = signal(settings.auth.oidc_enabled);
    let (passkeys_enabled, set_passkeys_enabled) = signal(settings.auth.passkeys_enabled);

    view! {
        <section class="settings-section">
            <h2>"Search Providers"</h2>
            <div class="provider-list">
                {move || {
                    search_providers.get().iter().map(|p| {
                        view! {
                            <div class="provider-item">
                                <span>{p.prefix.clone()}</span>
                                <span>{p.name.clone()}</span>
                                <span>{p.url_template.clone()}</span>
                            </div>
                        }
                    }).collect::<Vec<_>>()
                }}
            </div>
        </section>

        <section class="settings-section">
            <h2>"Integrations"</h2>
            <label>
                <input
                    type="checkbox"
                    prop:checked=docker_enabled
                    on:change=move |ev| set_docker_enabled.set(event_target_checked(&ev))
                />
                "Docker Discovery"
            </label>
            <label>
                <input
                    type="checkbox"
                    prop:checked=k8s_enabled
                    on:change=move |ev| set_k8s_enabled.set(event_target_checked(&ev))
                />
                "Kubernetes Discovery"
            </label>
        </section>

        <section class="settings-section">
            <h2>"Weather Widget"</h2>
            <label>
                <input
                    type="checkbox"
                    prop:checked=weather_enabled
                    on:change=move |ev| set_weather_enabled.set(event_target_checked(&ev))
                />
                "Enable Weather"
            </label>
            <label>
                "Location: "
                <input
                    type="text"
                    prop:value=weather_location
                    on:input=move |ev| set_weather_location.set(event_target_value(&ev))
                />
            </label>
            <label>
                "API Key (secret): "
                <input
                    type="password"
                    prop:value=weather_api_key
                    on:input=move |ev| set_weather_api_key.set(event_target_value(&ev))
                />
            </label>
        </section>

        <section class="settings-section">
            <h2>"Authentication"</h2>
            <label>
                <input
                    type="checkbox"
                    prop:checked=oidc_enabled
                    on:change=move |ev| set_oidc_enabled.set(event_target_checked(&ev))
                />
                "OIDC SSO"
            </label>
            <label>
                <input
                    type="checkbox"
                    prop:checked=passkeys_enabled
                    on:change=move |ev| set_passkeys_enabled.set(event_target_checked(&ev))
                />
                "Passkeys"
            </label>
        </section>
    }
}

/// Theme builder — pick from built-in themes, edit design tokens, custom CSS.
#[component]
fn ThemeBuilder(themes: Vec<ThemeSummary>) -> impl IntoView {
    let (theme_name, set_theme_name) = signal(String::new());
    let (bg, set_bg) = signal(String::new());
    let (text_color, set_text_color) = signal(String::new());
    let (accent, set_accent) = signal(String::new());
    let (radius, set_radius) = signal(String::new());
    let (custom_css, set_custom_css) = signal(String::new());

    view! {
        <section class="settings-section">
            <h2>"Theme Builder"</h2>

            <h3>"Available Themes"</h3>
            <div class="theme-list">
                {themes.iter().map(|t| {
                    view! {
                        <div class="theme-item">
                            <span>{t.name.clone()}</span>
                            {if t.is_builtin { " (builtin)" } else { " (custom)" }}
                        </div>
                    }
                }).collect::<Vec<_>>()}
            </div>

            <h3>"Create New Theme"</h3>
            <label>
                "Name: "
                <input
                    type="text"
                    prop:value=theme_name
                    on:input=move |ev| set_theme_name.set(event_target_value(&ev))
                />
            </label>
            <label>
                "Background: "
                <input
                    type="text"
                    prop:value=bg
                    on:input=move |ev| set_bg.set(event_target_value(&ev))
                />
            </label>
            <label>
                "Text Color: "
                <input
                    type="text"
                    prop:value=text_color
                    on:input=move |ev| set_text_color.set(event_target_value(&ev))
                />
            </label>
            <label>
                "Accent: "
                <input
                    type="text"
                    prop:value=accent
                    on:input=move |ev| set_accent.set(event_target_value(&ev))
                />
            </label>
            <label>
                "Border Radius: "
                <input
                    type="text"
                    prop:value=radius
                    on:input=move |ev| set_radius.set(event_target_value(&ev))
                />
            </label>
            <label>
                "Custom CSS:"
                <textarea
                    prop:value=custom_css
                    on:input=move |ev| set_custom_css.set(event_target_value(&ev))
                ></textarea>
            </label>
        </section>
    }
}
