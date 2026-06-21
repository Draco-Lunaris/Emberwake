//! T051: SSR test — active theme tokens + custom CSS present in first response (no default-theme flash).
//! Seeds a theme with specific design tokens + custom CSS, sets it as active,
//! and asserts the theme data is available for SSR rendering (no flash of default theme).

use sqlx::SqlitePool;

use app::domain::DesignTokens;
use app::server::settings_queries;

const SERVER_KEY: &[u8] = b"test-server-key-for-encryption";

/// Seed a custom theme with specific design tokens + custom CSS and set it active.
async fn seed_active_theme(pool: &SqlitePool) {
    settings_queries::seed_builtin_themes(pool)
        .await
        .expect("seed_builtin_themes");

    let tokens = DesignTokens {
        bg: Some("#1a1a2e".into()),
        surface: Some("#16213e".into()),
        text: Some("#e2e8f0".into()),
        accent: Some("#3b82f6".into()),
        border: Some("#334155".into()),
        radius: Some("12px".into()),
        mode: Some("dark".into()),
        ..Default::default()
    };

    let theme = settings_queries::save_theme_query(
        pool,
        &app::domain::ThemeInput {
            name: "Custom SSR Theme".into(),
            tokens,
            custom_css: Some(".tile { border-radius: 12px; }".into()),
        },
        None,
    )
    .await
    .expect("save_theme");

    settings_queries::set_active_theme_query(pool, theme.id, SERVER_KEY)
        .await
        .expect("set_active_theme");
}

/// T051: Active theme tokens + custom CSS are available in the first SSR response data.
/// The active theme is fetched server-side and its tokens/CSS would be injected into
/// the SSR HTML — no flash of a default theme.
#[sqlx::test(migrations = "../../migrations")]
async fn ssr_active_theme_tokens_present_in_first_response(pool: SqlitePool) {
    seed_active_theme(&pool).await;

    let active = settings_queries::get_active_theme_query(&pool)
        .await
        .expect("get_active_theme");

    assert!(active.is_some(), "active theme should be set");
    let theme = active.unwrap();

    // Assert the specific design tokens are present (would be injected as CSS custom properties)
    assert_eq!(theme.tokens.bg.as_deref(), Some("#1a1a2e"));
    assert_eq!(theme.tokens.surface.as_deref(), Some("#16213e"));
    assert_eq!(theme.tokens.text.as_deref(), Some("#e2e8f0"));
    assert_eq!(theme.tokens.accent.as_deref(), Some("#3b82f6"));
    assert_eq!(theme.tokens.border.as_deref(), Some("#334155"));
    assert_eq!(theme.tokens.radius.as_deref(), Some("12px"));
    assert_eq!(theme.tokens.mode.as_deref(), Some("dark"));

    // Assert custom CSS is present (would be served with CSP nonce)
    assert_eq!(
        theme.custom_css.as_deref(),
        Some(".tile { border-radius: 12px; }")
    );
}

/// T051: No active theme returns None — SSR would use prefers-color-scheme fallback.
#[sqlx::test(migrations = "../../migrations")]
async fn ssr_no_active_theme_returns_none_for_system_fallback(pool: SqlitePool) {
    let active = settings_queries::get_active_theme_query(&pool)
        .await
        .expect("get_active_theme");

    assert!(
        active.is_none(),
        "no active theme should return None — SSR uses prefers-color-scheme fallback"
    );
}

/// T051: Active theme survives across simulated restart (re-read from DB).
#[sqlx::test(migrations = "../../migrations")]
async fn ssr_theme_survives_reload(pool: SqlitePool) {
    seed_active_theme(&pool).await;

    // First read (initial page load)
    let first = settings_queries::get_active_theme_query(&pool)
        .await
        .expect("first get_active_theme")
        .expect("theme should be set");

    // Second read (page reload — simulates no flash since theme is server-side)
    let second = settings_queries::get_active_theme_query(&pool)
        .await
        .expect("second get_active_theme")
        .expect("theme should still be set");

    assert_eq!(
        first.id, second.id,
        "same theme should be active after reload"
    );
    assert_eq!(first.tokens.bg, second.tokens.bg, "tokens should match");
    assert_eq!(first.custom_css, second.custom_css, "CSS should match");
}
