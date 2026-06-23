//! Settings page — admin-gated settings + real theme builder.
//!
//! Theme builder features:
//! - Built-in preset themes selectable with one click
//! - Full token grid: color pickers + hex text inputs for all colour tokens
//! - Shape/spacing/font controls
//! - Live preview pane (updates CSS vars on a scoped div in real time)
//! - Custom CSS textarea
//! - Save → calls save_theme server function
//! - Activate → calls set_active_theme; reload to see effect globally

use crate::components::Navbar;
use crate::domain::{
    AuthSettings, DesignTokens, IntegrationSettings, SettingsPatch, SettingsView, ThemeInput,
    ThemeSummary, WeatherSettings,
};
use leptos::prelude::*;
use uuid::Uuid;

// ── Built-in preset definitions ──────────────────────────────────────────────

#[derive(Clone)]
struct Preset {
    name: &'static str,
    accent: &'static str,
    bg: &'static str,
    surface: &'static str,
    text: &'static str,
}

const PRESETS: &[Preset] = &[
    Preset {
        name: "Emberwake (default)",
        accent: "#f97316",
        bg: "#0d0d0f",
        surface: "#141418",
        text: "#eeeef0",
    },
    Preset {
        name: "Midnight Blue",
        accent: "#6366f1",
        bg: "#0a0b1a",
        surface: "#111228",
        text: "#e8eaf6",
    },
    Preset {
        name: "Forest",
        accent: "#22c55e",
        bg: "#080e0a",
        surface: "#0f1a11",
        text: "#e8f5e9",
    },
    Preset {
        name: "Rose",
        accent: "#f43f5e",
        bg: "#0d080a",
        surface: "#1a0e12",
        text: "#fce7f3",
    },
    Preset {
        name: "Slate Light",
        accent: "#f97316",
        bg: "#f5f5f7",
        surface: "#ffffff",
        text: "#1a1a1e",
    },
    Preset {
        name: "High Contrast",
        accent: "#facc15",
        bg: "#000000",
        surface: "#0a0a0a",
        text: "#ffffff",
    },
];

// ── Page root ─────────────────────────────────────────────────────────────────

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
        <div class="settings-page">
            <h1>"Settings"</h1>
            <Suspense fallback=|| view! { <p class="text-muted">"Loading settings…"</p> }>
                {move || {
                    settings_resource.get().map(|settings: SettingsView| {
                        view! { <SettingsForm settings /> }
                    })
                }}
            </Suspense>
            <Suspense fallback=|| view! { <p class="text-muted">"Loading themes…"</p> }>
                {move || {
                    themes_resource.get().map(|themes: Vec<ThemeSummary>| {
                        view! { <ThemeBuilderSection themes /> }
                    })
                }}
            </Suspense>
        </div>
    }
}

// ── Settings form ─────────────────────────────────────────────────────────────

#[component]
fn SettingsForm(settings: SettingsView) -> impl IntoView {
    let (docker_enabled, set_docker_enabled) = signal(settings.integrations.docker_enabled);
    let (k8s_enabled, set_k8s_enabled) = signal(settings.integrations.k8s_enabled);
    let (weather_enabled, set_weather_enabled) = signal(settings.weather.enabled);
    let (weather_location, set_weather_location) =
        signal(settings.weather.location.unwrap_or_default());
    let (weather_api_key, set_weather_api_key) =
        signal(settings.weather.api_key.unwrap_or_default());
    let (oidc_enabled, set_oidc_enabled) = signal(settings.auth.oidc_enabled);
    let (passkeys_enabled, set_passkeys_enabled) = signal(settings.auth.passkeys_enabled);

    let (save_status, set_save_status) = signal(Option::<String>::None);

    let save_action = Action::new(move |patch: &SettingsPatch| {
        let patch = patch.clone();
        async move {
            crate::server::settings::update_settings(patch)
                .await
                .map_err(|e| e.to_string())
        }
    });

    Effect::new(move || match save_action.value().get() {
        Some(Ok(_)) => set_save_status.set(Some("✓ Settings saved".to_string())),
        Some(Err(e)) => set_save_status.set(Some(format!("Error: {e}"))),
        None => {}
    });

    view! {
        // ── Search providers ─────────────────────────────────────
        <div class="settings-section">
            <h2>"Search Providers"</h2>
            <div class="provider-list">
                {settings.search_providers.providers.iter().map(|p| {
                    view! {
                        <div class="provider-item">
                            <span class="provider-prefix">{p.prefix.clone()}</span>
                            <span class="provider-name">{p.name.clone()}</span>
                            <span class="provider-url">{p.url_template.clone()}</span>
                        </div>
                    }
                }).collect::<Vec<_>>()}
            </div>
        </div>

        // ── Integrations ─────────────────────────────────────────
        <div class="settings-section">
            <h2>"Integrations"</h2>
            <div class="settings-row">
                <div>
                    <div class="settings-row-label">"Docker Discovery"</div>
                    <div class="settings-row-hint">"Reads container labels via the Docker socket"</div>
                </div>
                <label>
                    <input
                        type="checkbox"
                        prop:checked=docker_enabled
                        on:change=move |ev| set_docker_enabled.set(event_target_checked(&ev))
                    />
                </label>
            </div>
            <div class="settings-row">
                <div>
                    <div class="settings-row-label">"Kubernetes Discovery"</div>
                    <div class="settings-row-hint">"Reads Ingress annotations via the kube API"</div>
                </div>
                <label>
                    <input
                        type="checkbox"
                        prop:checked=k8s_enabled
                        on:change=move |ev| set_k8s_enabled.set(event_target_checked(&ev))
                    />
                </label>
            </div>
        </div>

        // ── Weather ──────────────────────────────────────────────
        <div class="settings-section">
            <h2>"Weather Widget"</h2>
            <div class="settings-row">
                <div class="settings-row-label">"Enable Weather"</div>
                <label>
                    <input
                        type="checkbox"
                        prop:checked=weather_enabled
                        on:change=move |ev| set_weather_enabled.set(event_target_checked(&ev))
                    />
                </label>
            </div>
            <div class="form-group">
                <label>"Location"</label>
                <input
                    type="text"
                    placeholder="e.g. Springfield, MO"
                    prop:value=weather_location
                    on:input=move |ev| set_weather_location.set(event_target_value(&ev))
                />
            </div>
            <div class="form-group">
                <label>"API Key"</label>
                <input
                    type="password"
                    placeholder="WeatherAPI secret key"
                    prop:value=weather_api_key
                    on:input=move |ev| set_weather_api_key.set(event_target_value(&ev))
                />
            </div>
        </div>

        // ── Auth ─────────────────────────────────────────────────
        <div class="settings-section">
            <h2>"Authentication"</h2>
            <div class="settings-row">
                <div>
                    <div class="settings-row-label">"OIDC / SSO"</div>
                    <div class="settings-row-hint">"Sign in via an external identity provider"</div>
                </div>
                <label>
                    <input
                        type="checkbox"
                        prop:checked=oidc_enabled
                        on:change=move |ev| set_oidc_enabled.set(event_target_checked(&ev))
                    />
                </label>
            </div>
            <div class="settings-row">
                <div>
                    <div class="settings-row-label">"Passkeys"</div>
                    <div class="settings-row-hint">"WebAuthn passwordless login"</div>
                </div>
                <label>
                    <input
                        type="checkbox"
                        prop:checked=passkeys_enabled
                        on:change=move |ev| set_passkeys_enabled.set(event_target_checked(&ev))
                    />
                </label>
            </div>
        </div>

        // ── Save ──────────────────────────────────────────────────
        <div style="display:flex; align-items:center; gap:12px; margin-top:16px;">
            <button on:click=move |_| {
                set_save_status.set(None);
                save_action.dispatch(SettingsPatch {
                    integrations: Some(IntegrationSettings {
                        docker_enabled: docker_enabled.get(),
                        docker_socket: None,
                        k8s_enabled: k8s_enabled.get(),
                    }),
                    weather: Some(WeatherSettings {
                        enabled: weather_enabled.get(),
                        location: {
                            let loc = weather_location.get();
                            if loc.is_empty() { None } else { Some(loc) }
                        },
                        api_key: {
                            let key = weather_api_key.get();
                            if key.is_empty() { None } else { Some(key) }
                        },
                        ..Default::default()
                    }),
                    auth: Some(AuthSettings {
                        oidc_enabled: oidc_enabled.get(),
                        passkeys_enabled: passkeys_enabled.get(),
                        ..Default::default()
                    }),
                    ..Default::default()
                });
            }>
                "Save Settings"
            </button>
            {move || save_status.get().map(|msg| view! {
                <span class=if msg.starts_with('✓') { "success-msg" } else { "error" }>
                    {msg.clone()}
                </span>
            })}
        </div>
    }
}

// ── Theme builder section ─────────────────────────────────────────────────────

#[component]
fn ThemeBuilderSection(themes: Vec<ThemeSummary>) -> impl IntoView {
    // ── Editable token signals ────────────────────────────────
    let (theme_name, set_theme_name) = signal("My Theme".to_string());
    let (bg, set_bg) = signal("#0d0d0f".to_string());
    let (bg_deep, set_bg_deep) = signal("#080809".to_string());
    let (surface, set_surface) = signal("#141418".to_string());
    let (surface_raised, set_surface_raised) = signal("#1e1e26".to_string());
    let (text, set_text) = signal("#eeeef0".to_string());
    let (text_muted, set_text_muted) = signal("#8888a0".to_string());
    let (text_faint, set_text_faint) = signal("#484860".to_string());
    let (accent, set_accent) = signal("#f97316".to_string());
    let (accent_text, set_accent_text) = signal("#ffffff".to_string());
    let (accent_alt, set_accent_alt) = signal("#6366f1".to_string());
    let (border, _set_border) = signal("rgba(255,255,255,0.07)".to_string());
    let (radius, set_radius) = signal("10px".to_string());
    let (font, set_font) = signal("'Inter', system-ui, sans-serif".to_string());
    let (custom_css, set_custom_css) = signal(String::new());

    // ── Save action ───────────────────────────────────────────
    let (save_status, set_save_status) = signal(Option::<String>::None);

    let save_action = Action::new(move |input: &ThemeInput| {
        let input = input.clone();
        async move {
            crate::server::settings::save_theme(input)
                .await
                .map_err(|e| e.to_string())
        }
    });

    Effect::new(move || match save_action.value().get() {
        Some(Ok(_)) => set_save_status.set(Some("✓ Theme saved".to_string())),
        Some(Err(e)) => set_save_status.set(Some(format!("Error: {e}"))),
        None => {}
    });

    // ── Apply preset ──────────────────────────────────────────
    let apply_preset = move |p: &Preset| {
        set_bg.set(p.bg.to_string());
        set_surface.set(p.surface.to_string());
        set_text.set(p.text.to_string());
        set_accent.set(p.accent.to_string());
        set_theme_name.set(p.name.to_string());
    };

    view! {
        <div class="settings-section">
            <h2>"Appearance & Themes"</h2>
            <div class="theme-builder">

                // ── Left column: controls ─────────────────────
                <div class="theme-controls">

                    // Saved themes list
                    <div>
                        <h3>"Saved Themes"</h3>
                        <div class="theme-list">
                            {themes.iter().map(|t| {
                                let id = t.id;
                                let name = t.name.clone();
                                let builtin = t.is_builtin;
                                view! {
                                    <div class="theme-item">
                                        <div class="theme-swatch" style=format!("background:{}", "#f97316")></div>
                                        <span class="theme-item-name">{name}</span>
                                        <span class="theme-item-badge">
                                            {if builtin { "built-in" } else { "custom" }}
                                        </span>
                                        <ActivateThemeButton id />
                                    </div>
                                }
                            }).collect::<Vec<_>>()}
                        </div>
                    </div>

                    // Preset picker
                    <div>
                        <h3>"Start from a Preset"</h3>
                        <div class="theme-list">
                            {PRESETS.iter().map(|p| {
                                let p_clone = p.clone();
                                view! {
                                    <div
                                        class="theme-item"
                                        style="cursor:pointer"
                                        on:click=move |_| apply_preset(&p_clone)
                                    >
                                        <div class="theme-swatch" style=format!("background:{}", p.accent)></div>
                                        <span class="theme-item-name">{p.name}</span>
                                    </div>
                                }
                            }).collect::<Vec<_>>()}
                        </div>
                    </div>

                    // Name
                    <div class="form-group">
                        <label>"Theme Name"</label>
                        <input
                            type="text"
                            prop:value=theme_name
                            on:input=move |ev| set_theme_name.set(event_target_value(&ev))
                        />
                    </div>

                    // Colour tokens
                    <div>
                        <h3>"Colour Tokens"</h3>
                        <div class="token-grid">
                            <TokenField label="Background" value=bg set_value=set_bg />
                            <TokenField label="Deep BG" value=bg_deep set_value=set_bg_deep />
                            <TokenField label="Surface" value=surface set_value=set_surface />
                            <TokenField label="Surface Raised" value=surface_raised set_value=set_surface_raised />
                            <TokenField label="Text" value=text set_value=set_text />
                            <TokenField label="Text Muted" value=text_muted set_value=set_text_muted />
                            <TokenField label="Text Faint" value=text_faint set_value=set_text_faint />
                            <TokenField label="Accent" value=accent set_value=set_accent />
                            <TokenField label="Accent Text" value=accent_text set_value=set_accent_text />
                            <TokenField label="Accent Alt" value=accent_alt set_value=set_accent_alt />
                        </div>
                    </div>

                    // Shape & type
                    <div>
                        <h3>"Shape & Type"</h3>
                        <div class="form-group">
                            <label>"Border Radius (e.g. 10px)"</label>
                            <input
                                type="text"
                                prop:value=radius
                                on:input=move |ev| set_radius.set(event_target_value(&ev))
                            />
                        </div>
                        <div class="form-group">
                            <label>"Font Stack"</label>
                            <input
                                type="text"
                                prop:value=font
                                on:input=move |ev| set_font.set(event_target_value(&ev))
                            />
                        </div>
                    </div>

                    // Custom CSS
                    <div class="form-group">
                        <label>"Custom CSS"</label>
                        <textarea
                            placeholder="/* Any valid CSS — applied after tokens */"
                            prop:value=custom_css
                            on:input=move |ev| set_custom_css.set(event_target_value(&ev))
                        ></textarea>
                    </div>

                    // Save
                    <div style="display:flex; align-items:center; gap:12px;">
                        <button on:click=move |_| {
                            set_save_status.set(None);
                            save_action.dispatch(ThemeInput {
                                name: theme_name.get(),
                                tokens: DesignTokens {
                                    bg: Some(bg.get()),
                                    bg_deep: Some(bg_deep.get()),
                                    surface: Some(surface.get()),
                                    surface_raised: Some(surface_raised.get()),
                                    text: Some(text.get()),
                                    text_muted: Some(text_muted.get()),
                                    text_faint: Some(text_faint.get()),
                                    accent: Some(accent.get()),
                                    accent_text: Some(accent_text.get()),
                                    accent_alt: Some(accent_alt.get()),
                                    border: Some(border.get()),
                                    radius: Some(radius.get()),
                                    font: Some(font.get()),
                                    ..Default::default()
                                },
                                custom_css: {
                                    let css = custom_css.get();
                                    if css.is_empty() { None } else { Some(css) }
                                },
                            });
                        }>
                            "Save Theme"
                        </button>
                        {move || save_status.get().map(|msg| view! {
                            <span class=if msg.starts_with('✓') { "success-msg" } else { "error" }>
                                {msg.clone()}
                            </span>
                        })}
                    </div>
                </div>

                // ── Right column: live preview ────────────────
                <div class="theme-preview">
                    <div class="theme-preview-header">"Live Preview"</div>
                    <div
                        class="theme-preview-body"
                        style=move || format!(
                            "--preview-bg:{}; --preview-surface:{}; --preview-accent:{}; \
                             --preview-text:{}; --preview-border:rgba(255,255,255,0.07); \
                             --preview-radius:{}; background:{}; color:{};",
                            bg.get(), surface.get(), accent.get(),
                            text.get(), radius.get(),
                            bg.get(), text.get()
                        )
                    >
                        // Mock search bar
                        <div class="preview-search"></div>

                        // Mock tile grid
                        <div class="preview-tiles">
                            {["Proxmox", "Grafana", "Gitea"].iter().map(|name| view! {
                                <div
                                    class="preview-tile"
                                    style=move || format!(
                                        "background:{}; border:1px solid rgba(255,255,255,0.07); border-radius:{}",
                                        surface.get(), radius.get()
                                    )
                                >
                                    <div
                                        class="preview-tile-icon"
                                        style=move || format!("background:{}; border-radius:5px", accent.get())
                                    ></div>
                                    <div
                                        class="preview-tile-label"
                                        style=move || format!("color:{}", text_muted.get())
                                    >
                                        {*name}
                                    </div>
                                </div>
                            }).collect::<Vec<_>>()}
                        </div>

                        // Mock category
                        <div style=move || format!(
                            "background:{}; border:1px solid rgba(255,255,255,0.07); \
                             border-radius:{}; padding:10px;",
                            surface.get(), radius.get()
                        )>
                            <div style=move || format!(
                                "font-size:0.65rem; letter-spacing:0.08em; text-transform:uppercase; \
                                 color:{}; margin-bottom:8px; padding-bottom:8px; \
                                 border-bottom:1px solid rgba(255,255,255,0.07);",
                                text_faint.get()
                            )>
                                "Bookmarks"
                            </div>
                            {["GitHub", "Docs", "Homelab wiki"].iter().map(|name| view! {
                                <div style=move || format!(
                                    "font-size:0.78rem; padding:4px 0; color:{}",
                                    text_muted.get()
                                )>
                                    {format!("› {name}")}
                                </div>
                            }).collect::<Vec<_>>()}
                        </div>

                        // Accent colour swatch
                        <div style=move || format!(
                            "margin-top:12px; height:3px; border-radius:2px; background:{}",
                            accent.get()
                        )></div>
                    </div>
                </div>

            </div>
        </div>
    }
}

// ── Token field: color picker + hex text input side by side ──────────────────

#[component]
fn TokenField(
    label: &'static str,
    value: ReadSignal<String>,
    set_value: WriteSignal<String>,
) -> impl IntoView {
    view! {
        <div class="token-field">
            <label>{label}</label>
            <div class="token-input-row">
                <input
                    type="color"
                    prop:value=move || value.get()
                    on:input=move |ev| set_value.set(event_target_value(&ev))
                    title=label
                />
                <input
                    type="text"
                    prop:value=move || value.get()
                    on:input=move |ev| set_value.set(event_target_value(&ev))
                    placeholder="#rrggbb"
                    spellcheck="false"
                />
            </div>
        </div>
    }
}

// ── Activate theme button ─────────────────────────────────────────────────────

#[component]
fn ActivateThemeButton(id: Uuid) -> impl IntoView {
    let (status, set_status) = signal(Option::<String>::None);

    let activate = Action::new(move |id: &Uuid| {
        let id = *id;
        async move {
            crate::server::settings::set_active_theme(id)
                .await
                .map_err(|e| e.to_string())
        }
    });

    Effect::new(move || match activate.value().get() {
        Some(Ok(_)) => set_status.set(Some("Active".to_string())),
        Some(Err(_)) => set_status.set(Some("Failed".to_string())),
        None => {}
    });

    view! {
        <button
            class="btn-ghost btn-sm"
            on:click=move |_| { activate.dispatch(id); }
        >
            {move || status.get().unwrap_or_else(|| "Activate".to_string())}
        </button>
    }
}
