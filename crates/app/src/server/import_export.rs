//! Server functions for import/export (US9).
//! export_data: admin-gated, returns ExportDocument.
//! import_preview: admin-gated, parses file under spawn_blocking, NO writes.
//! import_apply: admin-gated, transactional (all-or-nothing), audited.

use leptos::server_fn::ServerFnError;

use crate::domain::{
    ExportDocument, ExportScope, ImportKind, ImportOptions, ImportPreviewData, ImportResult,
};
use crate::error::AppError;

#[cfg(feature = "ssr")]
use crate::domain::{DuplicateStrategy, ParsedData};

/// Require admin session + CSRF for import/export operations.
#[cfg(feature = "ssr")]
async fn require_admin_csrf(
    pool: &sqlx::SqlitePool,
) -> Result<crate::server::auth_queries::SessionInfo, AppError> {
    let info = crate::server::auth_helper::require_session_csrf(pool).await?;
    if info.role != crate::domain::Role::Admin {
        return Err(AppError::Forbidden);
    }
    Ok(info)
}

/// Export all data (or selective subset) as a JSON document.
#[leptos::server]
pub async fn export_data(scope: ExportScope) -> Result<ExportDocument, ServerFnError<AppError>> {
    #[cfg(feature = "ssr")]
    {
        use axum::Extension;
        let pool = leptos_axum::extract::<Extension<sqlx::SqlitePool>>()
            .await
            .map_err(|_| AppError::Internal)?
            .0;
        let _info = require_admin_csrf(&pool).await?;
        crate::server::export_queries::export_data_query(&pool, &scope)
            .await
            .map_err(ServerFnError::from)
    }
    #[cfg(not(feature = "ssr"))]
    {
        let _ = scope;
        Err(ServerFnError::from(AppError::Unauthorized))
    }
}

/// Preview an import file without writing to the database.
/// Parses under spawn_blocking with size/derivation limits.
#[leptos::server]
pub async fn import_preview(
    file: Vec<u8>,
    kind: ImportKind,
) -> Result<ImportPreviewData, ServerFnError<AppError>> {
    #[cfg(feature = "ssr")]
    {
        use axum::Extension;
        let pool = leptos_axum::extract::<Extension<sqlx::SqlitePool>>()
            .await
            .map_err(|_| AppError::Internal)?
            .0;
        let _info = require_admin_csrf(&pool).await?;

        // Parse under spawn_blocking (heavy CPU work)
        let parsed =
            tokio::task::spawn_blocking(move || crate::server::importer::parse(&file, kind))
                .await
                .map_err(|e| AppError::Validation(format!("parsing task failed: {e}")))??;

        // Generate a preview token (base64-encoded ParsedData)
        let token = encode_token(&parsed);

        let sample_categories: Vec<String> = parsed
            .categories
            .iter()
            .take(5)
            .map(|c| c.name.clone())
            .collect();
        let sample_bookmarks: Vec<String> = parsed
            .bookmarks
            .iter()
            .take(5)
            .map(|b| b.name.clone())
            .collect();
        let sample_services: Vec<String> = parsed
            .services
            .iter()
            .take(5)
            .map(|s| s.name.clone())
            .collect();

        Ok(ImportPreviewData {
            token,
            category_count: parsed.categories.len(),
            bookmark_count: parsed.bookmarks.len(),
            service_count: parsed.services.len(),
            theme_count: parsed.themes.len(),
            has_settings: parsed.settings.is_some(),
            sample_categories,
            sample_bookmarks,
            sample_services,
        })
    }
    #[cfg(not(feature = "ssr"))]
    {
        let _ = (file, kind);
        Err(ServerFnError::from(AppError::Unauthorized))
    }
}

/// Apply a previewed import transactionally (all-or-nothing).
/// Regenerates UUIDs — source ids are never reused.
#[leptos::server]
pub async fn import_apply(
    token: String,
    options: ImportOptions,
) -> Result<ImportResult, ServerFnError<AppError>> {
    #[cfg(feature = "ssr")]
    {
        use axum::Extension;
        use chrono::Utc;
        use uuid::Uuid;

        let pool = leptos_axum::extract::<Extension<sqlx::SqlitePool>>()
            .await
            .map_err(|_| AppError::Internal)?
            .0;
        let info = require_admin_csrf(&pool).await?;

        // Decode the preview token back into ParsedData
        let parsed = decode_token(&token)?;

        let strategy = options.duplicate_strategy;
        let now = Utc::now().to_rfc3339();

        // Begin transaction — all or nothing
        let mut tx = pool.begin().await.map_err(AppError::from)?;

        let mut result = ImportResult::default();

        // Build a name→id map for categories so bookmarks/services can reference them
        let mut category_map: std::collections::HashMap<String, String> =
            std::collections::HashMap::new();

        // Import categories
        for cat in &parsed.categories {
            // Check for existing category with same name
            let existing: Option<(String,)> =
                sqlx::query_as("SELECT id FROM category WHERE name = ?")
                    .bind(&cat.name)
                    .fetch_optional(&mut *tx)
                    .await
                    .map_err(AppError::from)?;

            match existing {
                Some((existing_id,)) => match strategy {
                    DuplicateStrategy::Skip => {
                        result.categories_skipped += 1;
                        category_map.insert(cat.name.clone(), existing_id);
                    }
                    DuplicateStrategy::Overwrite => {
                        sqlx::query(
                            "UPDATE category SET icon = ?, visibility = ?, updated_at = ? WHERE id = ?",
                        )
                        .bind(&cat.icon)
                        .bind(cat.visibility.to_string())
                        .bind(&now)
                        .bind(&existing_id)
                        .execute(&mut *tx)
                        .await
                        .map_err(AppError::from)?;
                        result.categories_updated += 1;
                        category_map.insert(cat.name.clone(), existing_id);
                    }
                    DuplicateStrategy::Rename => {
                        let new_name = format!("{} (imported)", cat.name);
                        let new_id = Uuid::now_v7().to_string();
                        let max_row: (Option<i64>,) =
                            sqlx::query_as("SELECT MAX(order_index) FROM category")
                                .fetch_one(&mut *tx)
                                .await
                                .map_err(AppError::from)?;
                        let order_index = max_row.0.unwrap_or(-1) + 1;
                        sqlx::query(
                            "INSERT INTO category (id, name, icon, order_index, visibility, created_at, updated_at) \
                             VALUES (?, ?, ?, ?, ?, ?, ?)",
                        )
                        .bind(&new_id)
                        .bind(&new_name)
                        .bind(&cat.icon)
                        .bind(order_index)
                        .bind(cat.visibility.to_string())
                        .bind(&now)
                        .bind(&now)
                        .execute(&mut *tx)
                        .await
                        .map_err(AppError::from)?;
                        result.categories_created += 1;
                        category_map.insert(cat.name.clone(), new_id);
                    }
                },
                None => {
                    let new_id = Uuid::now_v7().to_string();
                    let max_row: (Option<i64>,) =
                        sqlx::query_as("SELECT MAX(order_index) FROM category")
                            .fetch_one(&mut *tx)
                            .await
                            .map_err(AppError::from)?;
                    let order_index = max_row.0.unwrap_or(-1) + 1;
                    sqlx::query(
                        "INSERT INTO category (id, name, icon, order_index, visibility, created_at, updated_at) \
                         VALUES (?, ?, ?, ?, ?, ?, ?)",
                    )
                    .bind(&new_id)
                    .bind(&cat.name)
                    .bind(&cat.icon)
                    .bind(order_index)
                    .bind(cat.visibility.to_string())
                    .bind(&now)
                    .bind(&now)
                    .execute(&mut *tx)
                    .await
                    .map_err(AppError::from)?;
                    result.categories_created += 1;
                    category_map.insert(cat.name.clone(), new_id);
                }
            }
        }

        // Import bookmarks
        for bm in &parsed.bookmarks {
            let cat_id = bm
                .category_name
                .as_ref()
                .and_then(|name| category_map.get(name))
                .cloned();

            // Check for duplicate by name+url
            let existing: Option<(String,)> =
                sqlx::query_as("SELECT id FROM bookmark WHERE name = ? AND url = ?")
                    .bind(&bm.name)
                    .bind(&bm.url)
                    .fetch_optional(&mut *tx)
                    .await
                    .map_err(AppError::from)?;

            match existing {
                Some((existing_id,)) => match strategy {
                    DuplicateStrategy::Skip => {
                        result.bookmarks_skipped += 1;
                    }
                    DuplicateStrategy::Overwrite => {
                        sqlx::query(
                            "UPDATE bookmark SET category_id = ?, icon = ?, visibility = ?, updated_at = ? WHERE id = ?",
                        )
                        .bind(&cat_id)
                        .bind(&bm.icon)
                        .bind(bm.visibility.to_string())
                        .bind(&now)
                        .bind(&existing_id)
                        .execute(&mut *tx)
                        .await
                        .map_err(AppError::from)?;
                        result.bookmarks_updated += 1;
                    }
                    DuplicateStrategy::Rename => {
                        let new_name = format!("{} (imported)", bm.name);
                        let new_id = Uuid::now_v7().to_string();
                        let max_row: (Option<i64>,) =
                            sqlx::query_as("SELECT MAX(order_index) FROM bookmark")
                                .fetch_one(&mut *tx)
                                .await
                                .map_err(AppError::from)?;
                        let order_index = max_row.0.unwrap_or(-1) + 1;
                        sqlx::query(
                            "INSERT INTO bookmark (id, category_id, name, url, icon, order_index, visibility, created_at, updated_at) \
                             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
                        )
                        .bind(&new_id)
                        .bind(&cat_id)
                        .bind(&new_name)
                        .bind(&bm.url)
                        .bind(&bm.icon)
                        .bind(order_index)
                        .bind(bm.visibility.to_string())
                        .bind(&now)
                        .bind(&now)
                        .execute(&mut *tx)
                        .await
                        .map_err(AppError::from)?;
                        result.bookmarks_created += 1;
                    }
                },
                None => {
                    let new_id = Uuid::now_v7().to_string();
                    let max_row: (Option<i64>,) =
                        sqlx::query_as("SELECT MAX(order_index) FROM bookmark")
                            .fetch_one(&mut *tx)
                            .await
                            .map_err(AppError::from)?;
                    let order_index = max_row.0.unwrap_or(-1) + 1;
                    sqlx::query(
                        "INSERT INTO bookmark (id, category_id, name, url, icon, order_index, visibility, created_at, updated_at) \
                         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
                    )
                    .bind(&new_id)
                    .bind(&cat_id)
                    .bind(&bm.name)
                    .bind(&bm.url)
                    .bind(&bm.icon)
                    .bind(order_index)
                    .bind(bm.visibility.to_string())
                    .bind(&now)
                    .bind(&now)
                    .execute(&mut *tx)
                    .await
                    .map_err(AppError::from)?;
                    result.bookmarks_created += 1;
                }
            }
        }

        // Import services
        for svc in &parsed.services {
            let cat_id = svc
                .category_name
                .as_ref()
                .and_then(|name| category_map.get(name))
                .cloned();

            // Check for duplicate by name+url
            let existing: Option<(String,)> =
                sqlx::query_as("SELECT id FROM service WHERE name = ? AND url = ?")
                    .bind(&svc.name)
                    .bind(&svc.url)
                    .fetch_optional(&mut *tx)
                    .await
                    .map_err(AppError::from)?;

            match existing {
                Some((existing_id,)) => match strategy {
                    DuplicateStrategy::Skip => {
                        result.services_skipped += 1;
                    }
                    DuplicateStrategy::Overwrite => {
                        sqlx::query(
                            "UPDATE service SET category_id = ?, icon = ?, description = ?, visibility = ?, \
                             monitor_enabled = ?, monitor_kind = ?, monitor_target = ?, monitor_interval_s = ?, \
                             updated_at = ? WHERE id = ?",
                        )
                        .bind(&cat_id)
                        .bind(&svc.icon)
                        .bind(&svc.description)
                        .bind(svc.visibility.to_string())
                        .bind(svc.monitor_enabled as i64)
                        .bind(&svc.monitor_kind)
                        .bind(&svc.monitor_target)
                        .bind(svc.monitor_interval_s)
                        .bind(&now)
                        .bind(&existing_id)
                        .execute(&mut *tx)
                        .await
                        .map_err(AppError::from)?;
                        result.services_updated += 1;
                    }
                    DuplicateStrategy::Rename => {
                        let new_name = format!("{} (imported)", svc.name);
                        let new_id = Uuid::now_v7().to_string();
                        let max_row: (Option<i64>,) =
                            sqlx::query_as("SELECT MAX(order_index) FROM service")
                                .fetch_one(&mut *tx)
                                .await
                                .map_err(AppError::from)?;
                        let order_index = max_row.0.unwrap_or(-1) + 1;
                        sqlx::query(
                            "INSERT INTO service (id, category_id, name, url, icon, description, is_pinned, \
                             order_index, visibility, monitor_enabled, monitor_kind, monitor_target, \
                             monitor_interval_s, created_at, updated_at) \
                             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                        )
                        .bind(&new_id)
                        .bind(&cat_id)
                        .bind(&new_name)
                        .bind(&svc.url)
                        .bind(&svc.icon)
                        .bind(&svc.description)
                        .bind(svc.is_pinned as i64)
                        .bind(order_index)
                        .bind(svc.visibility.to_string())
                        .bind(svc.monitor_enabled as i64)
                        .bind(&svc.monitor_kind)
                        .bind(&svc.monitor_target)
                        .bind(svc.monitor_interval_s)
                        .bind(&now)
                        .bind(&now)
                        .execute(&mut *tx)
                        .await
                        .map_err(AppError::from)?;
                        result.services_created += 1;
                    }
                },
                None => {
                    let new_id = Uuid::now_v7().to_string();
                    let max_row: (Option<i64>,) =
                        sqlx::query_as("SELECT MAX(order_index) FROM service")
                            .fetch_one(&mut *tx)
                            .await
                            .map_err(AppError::from)?;
                    let order_index = max_row.0.unwrap_or(-1) + 1;
                    sqlx::query(
                        "INSERT INTO service (id, category_id, name, url, icon, description, is_pinned, \
                         order_index, visibility, monitor_enabled, monitor_kind, monitor_target, \
                         monitor_interval_s, created_at, updated_at) \
                         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                    )
                    .bind(&new_id)
                    .bind(&cat_id)
                    .bind(&svc.name)
                    .bind(&svc.url)
                    .bind(&svc.icon)
                    .bind(&svc.description)
                    .bind(svc.is_pinned as i64)
                    .bind(order_index)
                    .bind(svc.visibility.to_string())
                    .bind(svc.monitor_enabled as i64)
                    .bind(&svc.monitor_kind)
                    .bind(&svc.monitor_target)
                    .bind(svc.monitor_interval_s)
                    .bind(&now)
                    .bind(&now)
                    .execute(&mut *tx)
                    .await
                    .map_err(AppError::from)?;
                    result.services_created += 1;
                }
            }
        }

        // Import themes (skip builtins)
        for theme in &parsed.themes {
            let existing: Option<(String,)> = sqlx::query_as("SELECT id FROM theme WHERE name = ?")
                .bind(&theme.name)
                .fetch_optional(&mut *tx)
                .await
                .map_err(AppError::from)?;

            if existing.is_some() {
                result.themes_skipped += 1;
            } else {
                let new_id = Uuid::now_v7().to_string();
                let tokens_json =
                    serde_json::to_string(&theme.tokens).unwrap_or_else(|_| "{}".to_string());
                sqlx::query(
                    "INSERT INTO theme (id, name, tokens, custom_css, is_builtin, created_by, created_at) \
                     VALUES (?, ?, ?, ?, 0, ?, ?)",
                )
                .bind(&new_id)
                .bind(&theme.name)
                .bind(&tokens_json)
                .bind(&theme.custom_css)
                .bind(info.user_id.to_string())
                .bind(&now)
                .execute(&mut *tx)
                .await
                .map_err(AppError::from)?;
                result.themes_created += 1;
            }
        }

        // Commit transaction
        tx.commit().await.map_err(AppError::from)?;

        // Audit the import
        crate::server::auth_queries::audit_write_query(
            &pool,
            Some(info.user_id),
            "content_mutate",
            Some(&format!("import:{}", token)),
            None,
            None,
            "success",
        )
        .await;

        Ok(result)
    }
    #[cfg(not(feature = "ssr"))]
    {
        let _ = (token, options);
        Err(ServerFnError::from(AppError::Unauthorized))
    }
}

/// Encode ParsedData as a base64 JSON token (stateless preview token).
#[cfg(feature = "ssr")]
fn encode_token(data: &ParsedData) -> String {
    use base64::prelude::*;
    let json = serde_json::to_string(data).unwrap_or_else(|_| "{}".to_string());
    BASE64_STANDARD.encode(json.as_bytes())
}

/// Decode a base64 JSON token back into ParsedData.
#[cfg(feature = "ssr")]
fn decode_token(token: &str) -> Result<ParsedData, AppError> {
    use base64::prelude::*;
    let json_bytes = BASE64_STANDARD
        .decode(token)
        .map_err(|_| AppError::Validation("invalid preview token".into()))?;
    serde_json::from_slice(&json_bytes)
        .map_err(|_| AppError::Validation("invalid preview token data".into()))
}
