//! T050: Server-fn tests for settings + theme CRUD incl. secret redaction for non-admins.
//! Tests query functions directly (no HTTP layer needed) using #[sqlx::test].

use sqlx::SqlitePool;

use app::domain::{
    AuthSettings, DesignTokens, IntegrationSettings, SearchProvider, SearchProviderConfig,
    SettingsPatch, SettingsView, ThemeInput, WeatherSettings,
};
use app::error::AppError;
use app::server::settings_queries;

const SERVER_KEY: &[u8] = b"test-server-key-for-encryption";

// --- Theme CRUD tests ---

#[sqlx::test(migrations = "../../migrations")]
async fn save_theme_persists_and_returns_with_id(pool: SqlitePool) {
    let input = ThemeInput {
        name: "Custom Dark".into(),
        tokens: DesignTokens {
            bg: Some("#111111".into()),
            text: Some("#ffffff".into()),
            mode: Some("dark".into()),
            ..Default::default()
        },
        custom_css: Some("body { margin: 0; }".into()),
    };

    let theme = settings_queries::save_theme_query(&pool, &input, None)
        .await
        .expect("save_theme should succeed");

    assert!(!theme.id.to_string().is_empty(), "theme should have an id");
    assert_eq!(theme.name, "Custom Dark");
    assert_eq!(theme.tokens.bg.as_deref(), Some("#111111"));
    assert_eq!(theme.custom_css.as_deref(), Some("body { margin: 0; }"));
    assert!(!theme.is_builtin, "custom theme should not be builtin");
}

#[sqlx::test(migrations = "../../migrations")]
async fn get_active_theme_returns_none_when_unset(pool: SqlitePool) {
    let theme = settings_queries::get_active_theme_query(&pool)
        .await
        .expect("get_active_theme should succeed");
    assert!(theme.is_none(), "no active theme should be set");
}

#[sqlx::test(migrations = "../../migrations")]
async fn set_active_theme_changes_active_theme(pool: SqlitePool) {
    let input = ThemeInput {
        name: "My Theme".into(),
        tokens: DesignTokens::default(),
        custom_css: None,
    };
    let theme = settings_queries::save_theme_query(&pool, &input, None)
        .await
        .expect("save_theme");

    settings_queries::set_active_theme_query(&pool, theme.id, SERVER_KEY)
        .await
        .expect("set_active_theme should succeed");

    let active = settings_queries::get_active_theme_query(&pool)
        .await
        .expect("get_active_theme");
    assert!(active.is_some(), "active theme should be set");
    assert_eq!(active.unwrap().id, theme.id);
}

#[sqlx::test(migrations = "../../migrations")]
async fn list_themes_returns_all_themes(pool: SqlitePool) {
    // Seed built-in themes
    settings_queries::seed_builtin_themes(&pool)
        .await
        .expect("seed_builtin_themes");

    // Save a custom theme
    let input = ThemeInput {
        name: "Custom".into(),
        tokens: DesignTokens::default(),
        custom_css: None,
    };
    settings_queries::save_theme_query(&pool, &input, None)
        .await
        .expect("save_theme");

    let themes = settings_queries::list_themes_query(&pool)
        .await
        .expect("list_themes");
    assert!(
        themes.len() >= 3,
        "should have 2 builtin + 1 custom, got {}",
        themes.len()
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn set_active_theme_nonexistent_returns_not_found(pool: SqlitePool) {
    let result =
        settings_queries::set_active_theme_query(&pool, uuid::Uuid::now_v7(), SERVER_KEY).await;
    assert!(
        matches!(result, Err(AppError::NotFound)),
        "setting non-existent theme should return NotFound"
    );
}

// --- Settings CRUD tests ---

#[sqlx::test(migrations = "../../migrations")]
async fn update_settings_persists(pool: SqlitePool) {
    let patch = SettingsPatch {
        search_providers: Some(SearchProviderConfig {
            providers: vec![SearchProvider {
                prefix: "g".into(),
                name: "Google".into(),
                url_template: "https://google.com/search?q={}".into(),
            }],
            default_provider: Some("Google".into()),
        }),
        integrations: Some(IntegrationSettings {
            docker_socket: None,
            docker_enabled: true,
            k8s_enabled: false,
        }),
        weather: Some(WeatherSettings {
            api_key: Some("secret-weather-key".into()),
            location: Some("Berlin".into()),
            api_url: None,
            refresh_interval_s: None,
            enabled: true,
        }),
        auth: Some(AuthSettings {
            oidc_enabled: true,
            oidc_issuer_url: Some("https://idp.example.com".into()),
            oidc_client_id: Some("client123".into()),
            oidc_client_secret: Some("super-secret-client-secret".into()),
            passkeys_enabled: false,
        }),
        theme_active: None,
    };

    settings_queries::apply_settings_patch(&pool, &patch, SERVER_KEY)
        .await
        .expect("apply_settings_patch should succeed");

    // Verify settings persisted (with secrets, as admin)
    let view = settings_queries::get_settings_view(&pool, SERVER_KEY, true)
        .await
        .expect("get_settings_view");

    assert_eq!(view.search_providers.providers.len(), 1);
    assert_eq!(view.search_providers.providers[0].prefix, "g");
    assert!(view.integrations.docker_enabled);
    assert_eq!(view.weather.api_key.as_deref(), Some("secret-weather-key"));
    assert_eq!(view.weather.location.as_deref(), Some("Berlin"));
    assert!(view.auth.oidc_enabled);
    assert_eq!(
        view.auth.oidc_client_secret.as_deref(),
        Some("super-secret-client-secret")
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn get_settings_as_admin_returns_full_secrets(pool: SqlitePool) {
    let patch = SettingsPatch {
        weather: Some(WeatherSettings {
            api_key: Some("my-api-key".into()),
            location: None,
            api_url: None,
            refresh_interval_s: None,
            enabled: true,
        }),
        auth: Some(AuthSettings {
            oidc_client_secret: Some("oidc-secret".into()),
            ..Default::default()
        }),
        ..Default::default()
    };
    settings_queries::apply_settings_patch(&pool, &patch, SERVER_KEY)
        .await
        .expect("patch");

    let view: SettingsView = settings_queries::get_settings_view(&pool, SERVER_KEY, true)
        .await
        .expect("get_settings_view admin");

    assert_eq!(view.weather.api_key.as_deref(), Some("my-api-key"));
    assert_eq!(view.auth.oidc_client_secret.as_deref(), Some("oidc-secret"));
}

#[sqlx::test(migrations = "../../migrations")]
async fn get_settings_as_non_admin_redacts_secrets(pool: SqlitePool) {
    let patch = SettingsPatch {
        weather: Some(WeatherSettings {
            api_key: Some("my-api-key".into()),
            location: Some("NYC".into()),
            api_url: None,
            refresh_interval_s: None,
            enabled: true,
        }),
        auth: Some(AuthSettings {
            oidc_client_secret: Some("oidc-secret".into()),
            oidc_client_id: Some("client-id".into()),
            ..Default::default()
        }),
        ..Default::default()
    };
    settings_queries::apply_settings_patch(&pool, &patch, SERVER_KEY)
        .await
        .expect("patch");

    let view = settings_queries::get_settings_view(&pool, SERVER_KEY, false)
        .await
        .expect("get_settings_view non-admin");

    // Secrets should be redacted
    assert!(
        view.weather.api_key.is_none(),
        "weather API key should be redacted"
    );
    assert_eq!(view.weather.location.as_deref(), Some("NYC")); // non-secret preserved
    assert!(
        view.auth.oidc_client_secret.is_none(),
        "OIDC client secret should be redacted"
    );
    assert_eq!(view.auth.oidc_client_id.as_deref(), Some("client-id")); // non-secret preserved
}

#[sqlx::test(migrations = "../../migrations")]
async fn secret_bearing_settings_encrypted_at_rest(pool: SqlitePool) {
    let patch = SettingsPatch {
        weather: Some(WeatherSettings {
            api_key: Some("plaintext-weather-key".into()),
            location: Some("Tokyo".into()),
            api_url: None,
            refresh_interval_s: None,
            enabled: true,
        }),
        auth: Some(AuthSettings {
            oidc_client_secret: Some("plaintext-oidc-secret".into()),
            ..Default::default()
        }),
        ..Default::default()
    };
    settings_queries::apply_settings_patch(&pool, &patch, SERVER_KEY)
        .await
        .expect("patch");

    // Read raw values from DB — they should be encrypted (not plaintext)
    let weather_raw: (String,) = sqlx::query_as("SELECT value FROM setting WHERE key = 'weather'")
        .fetch_one(&pool)
        .await
        .expect("read weather setting");
    assert!(
        !weather_raw.0.contains("plaintext-weather-key"),
        "weather setting should be encrypted at rest, not stored as plaintext"
    );

    let auth_raw: (String,) = sqlx::query_as("SELECT value FROM setting WHERE key = 'auth'")
        .fetch_one(&pool)
        .await
        .expect("read auth setting");
    assert!(
        !auth_raw.0.contains("plaintext-oidc-secret"),
        "auth setting should be encrypted at rest, not stored as plaintext"
    );

    // Verify decryption round-trip works (admin view)
    let view = settings_queries::get_settings_view(&pool, SERVER_KEY, true)
        .await
        .expect("get_settings_view admin");
    assert_eq!(
        view.weather.api_key.as_deref(),
        Some("plaintext-weather-key")
    );
    assert_eq!(
        view.auth.oidc_client_secret.as_deref(),
        Some("plaintext-oidc-secret")
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn save_theme_empty_name_rejected(pool: SqlitePool) {
    let input = ThemeInput {
        name: "  ".into(),
        tokens: DesignTokens::default(),
        custom_css: None,
    };
    let result = settings_queries::save_theme_query(&pool, &input, None).await;
    assert!(
        matches!(result, Err(AppError::Validation(_))),
        "empty theme name should be rejected"
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn seed_builtin_themes_creates_light_and_dark(pool: SqlitePool) {
    settings_queries::seed_builtin_themes(&pool)
        .await
        .expect("seed_builtin_themes");

    let themes = settings_queries::list_themes_query(&pool)
        .await
        .expect("list_themes");
    assert_eq!(themes.len(), 2, "should have exactly 2 builtin themes");

    let names: Vec<&str> = themes.iter().map(|t| t.name.as_str()).collect();
    assert!(names.contains(&"Light"), "should have Light theme");
    assert!(names.contains(&"Dark"), "should have Dark theme");
    assert!(
        themes.iter().all(|t| t.is_builtin),
        "all seeded themes should be builtin"
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn seed_builtin_themes_idempotent(pool: SqlitePool) {
    settings_queries::seed_builtin_themes(&pool)
        .await
        .expect("first seed");
    settings_queries::seed_builtin_themes(&pool)
        .await
        .expect("second seed (idempotent)");

    let themes = settings_queries::list_themes_query(&pool)
        .await
        .expect("list_themes");
    assert_eq!(
        themes.len(),
        2,
        "should still have exactly 2 builtin themes after double seed"
    );
}
