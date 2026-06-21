//! SQL query functions for data export (US9).
//! Reads all entity types and converts to export DTOs.
//! Excludes: password hashes, session data, API token hashes, secret settings.
//! Uses parameterized SQL with static string literals.

#![cfg(feature = "ssr")]

use chrono::Utc;
use sqlx::{Row, SqlitePool};

use crate::domain::{
    ExportBookmark, ExportCategory, ExportDocument, ExportEntity, ExportScope, ExportService,
    ExportTheme, Visibility,
};
use crate::error::AppError;

/// Export all data (or a selective subset) into an ExportDocument.
pub async fn export_data_query(
    pool: &SqlitePool,
    scope: &ExportScope,
) -> Result<ExportDocument, AppError> {
    let exported_at = Utc::now().to_rfc3339();

    let (include_categories, include_services, include_bookmarks, include_settings, include_themes) =
        match scope {
            ExportScope::Full => (true, true, true, true, true),
            ExportScope::Selective(entities) => {
                let cat = entities.contains(&ExportEntity::Categories);
                let svc = entities.contains(&ExportEntity::Services);
                let bm = entities.contains(&ExportEntity::Bookmarks);
                let set = entities.contains(&ExportEntity::Settings);
                let th = entities.contains(&ExportEntity::Themes);
                (cat, svc, bm, set, th)
            }
        };

    let categories = if include_categories {
        export_categories_query(pool).await?
    } else {
        Vec::new()
    };

    let services = if include_services {
        export_services_query(pool).await?
    } else {
        Vec::new()
    };

    let bookmarks = if include_bookmarks {
        export_bookmarks_query(pool).await?
    } else {
        Vec::new()
    };

    let settings = if include_settings {
        export_settings_query(pool).await?
    } else {
        None
    };

    let themes = if include_themes {
        export_themes_query(pool).await?
    } else {
        Vec::new()
    };

    Ok(ExportDocument {
        version: "1.0".to_string(),
        exported_at,
        categories,
        services,
        bookmarks,
        settings,
        themes,
    })
}

async fn export_categories_query(pool: &SqlitePool) -> Result<Vec<ExportCategory>, AppError> {
    let rows = sqlx::query(
        "SELECT name, icon, order_index, visibility FROM category ORDER BY order_index",
    )
    .fetch_all(pool)
    .await?;

    Ok(rows
        .iter()
        .map(|r| ExportCategory {
            name: r.get("name"),
            icon: r.get("icon"),
            order_index: r.get("order_index"),
            visibility: parse_visibility(r.get("visibility")),
        })
        .collect())
}

async fn export_services_query(pool: &SqlitePool) -> Result<Vec<ExportService>, AppError> {
    let rows = sqlx::query(
        "SELECT s.name, s.url, s.icon, s.description, s.is_pinned, s.order_index, \
         s.visibility, s.monitor_enabled, s.monitor_kind, s.monitor_target, s.monitor_interval_s, \
         c.name AS category_name \
         FROM service s \
         LEFT JOIN category c ON s.category_id = c.id \
         ORDER BY s.order_index",
    )
    .fetch_all(pool)
    .await?;

    Ok(rows
        .iter()
        .map(|r| ExportService {
            name: r.get("name"),
            url: r.get("url"),
            icon: r.get("icon"),
            description: r.get("description"),
            is_pinned: r.get::<i64, _>("is_pinned") != 0,
            order_index: r.get("order_index"),
            visibility: parse_visibility(r.get("visibility")),
            category_name: r.get("category_name"),
            monitor_enabled: r.get::<i64, _>("monitor_enabled") != 0,
            monitor_kind: r.get("monitor_kind"),
            monitor_target: r.get("monitor_target"),
            monitor_interval_s: r.get("monitor_interval_s"),
        })
        .collect())
}

async fn export_bookmarks_query(pool: &SqlitePool) -> Result<Vec<ExportBookmark>, AppError> {
    let rows = sqlx::query(
        "SELECT b.name, b.url, b.icon, b.order_index, b.visibility, \
         c.name AS category_name \
         FROM bookmark b \
         LEFT JOIN category c ON b.category_id = c.id \
         ORDER BY b.order_index",
    )
    .fetch_all(pool)
    .await?;

    Ok(rows
        .iter()
        .map(|r| ExportBookmark {
            name: r.get("name"),
            url: r.get("url"),
            icon: r.get("icon"),
            order_index: r.get("order_index"),
            visibility: parse_visibility(r.get("visibility")),
            category_name: r.get("category_name"),
        })
        .collect())
}

async fn export_settings_query(pool: &SqlitePool) -> Result<Option<serde_json::Value>, AppError> {
    // Export non-secret settings only.
    // Secret-bearing keys: weather, auth — these are excluded.
    let rows = sqlx::query("SELECT key, value FROM setting WHERE key != 'setup_complete'")
        .fetch_all(pool)
        .await?;

    let mut map = serde_json::Map::new();
    for row in &rows {
        let key: String = row.get("key");
        let value: String = row.get("value");
        // Skip secret-bearing keys
        if key.starts_with("weather") || key.starts_with("auth") {
            continue;
        }
        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&value) {
            map.insert(key, parsed);
        } else {
            map.insert(key, serde_json::Value::String(value));
        }
    }

    if map.is_empty() {
        Ok(None)
    } else {
        Ok(Some(serde_json::Value::Object(map)))
    }
}

async fn export_themes_query(pool: &SqlitePool) -> Result<Vec<ExportTheme>, AppError> {
    let rows = sqlx::query("SELECT name, tokens, custom_css, is_builtin FROM theme")
        .fetch_all(pool)
        .await?;

    Ok(rows
        .iter()
        .map(|r| {
            let tokens_json: String = r.get("tokens");
            let tokens: crate::domain::DesignTokens =
                serde_json::from_str(&tokens_json).unwrap_or_default();
            ExportTheme {
                name: r.get("name"),
                tokens,
                custom_css: r.get("custom_css"),
                is_builtin: r.get::<i64, _>("is_builtin") != 0,
            }
        })
        .collect())
}

fn parse_visibility(s: String) -> Visibility {
    s.parse().unwrap_or_default()
}
