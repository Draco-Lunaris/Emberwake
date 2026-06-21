//! Public REST API surface: /api/v1/* routes.
//! All routes: bearer auth, scope-checked, rate-limited, audited.
//! Uses content_queries and content_write_queries for data access.

use axum::Router;
use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::response::{IntoResponse, Json};
use axum::routing::{get, patch};
use serde_json::json;
use sqlx::Row;
use uuid::Uuid;

use crate::auth::api_token::{require_scope, verify_bearer};
use crate::state::AppState;

// --- Services ---

/// GET /api/v1/services — list all services (scope: services:read)
pub async fn list_services(State(state): State<AppState>, headers: HeaderMap) -> impl IntoResponse {
    let server_key = state.config.server_key.as_bytes();
    let verified = match verify_bearer(&headers, &state.db, server_key).await {
        Ok(v) => v,
        Err(e) => return e.into_response(),
    };
    if let Err(e) = require_scope(&verified, "services:read") {
        return e.into_response();
    }

    let rows = sqlx::query(
        "SELECT id, category_id, name, url, icon, description, is_pinned, order_index, \
         visibility, monitor_enabled, monitor_kind, monitor_target, monitor_interval_s, \
         created_at, updated_at FROM service ORDER BY order_index ASC",
    )
    .fetch_all(&state.db)
    .await;

    match rows {
        Ok(rows) => {
            let services: Vec<serde_json::Value> = rows
                .iter()
                .map(|r| {
                    json!({
                        "id": r.get::<String, _>("id"),
                        "category_id": r.get::<Option<String>, _>("category_id"),
                        "name": r.get::<String, _>("name"),
                        "url": r.get::<String, _>("url"),
                        "icon": r.get::<Option<String>, _>("icon"),
                        "description": r.get::<Option<String>, _>("description"),
                        "is_pinned": r.get::<i64, _>("is_pinned") != 0,
                        "order_index": r.get::<i64, _>("order_index"),
                        "visibility": r.get::<String, _>("visibility"),
                        "monitor_enabled": r.get::<i64, _>("monitor_enabled") != 0,
                        "monitor_kind": r.get::<Option<String>, _>("monitor_kind"),
                        "monitor_target": r.get::<Option<String>, _>("monitor_target"),
                        "monitor_interval_s": r.get::<Option<i64>, _>("monitor_interval_s"),
                        "created_at": r.get::<String, _>("created_at"),
                        "updated_at": r.get::<String, _>("updated_at"),
                    })
                })
                .collect();
            Json(json!(services)).into_response()
        }
        Err(_) => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "database error"})),
        )
            .into_response(),
    }
}

/// POST /api/v1/services — create service (scope: services:write)
pub async fn create_service(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<serde_json::Value>,
) -> impl IntoResponse {
    let server_key = state.config.server_key.as_bytes();
    let verified = match verify_bearer(&headers, &state.db, server_key).await {
        Ok(v) => v,
        Err(e) => return e.into_response(),
    };
    if let Err(e) = require_scope(&verified, "services:write") {
        return e.into_response();
    }

    let id = Uuid::now_v7().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    let name = body.get("name").and_then(|v| v.as_str()).unwrap_or("");
    let url = body.get("url").and_then(|v| v.as_str()).unwrap_or("");
    if name.is_empty() || url.is_empty() {
        return (
            axum::http::StatusCode::BAD_REQUEST,
            Json(json!({"error": "name and url are required"})),
        )
            .into_response();
    }

    let category_id = body.get("category_id").and_then(|v| v.as_str());
    let icon = body.get("icon").and_then(|v| v.as_str());
    let description = body.get("description").and_then(|v| v.as_str());
    let is_pinned = body
        .get("is_pinned")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let visibility = body
        .get("visibility")
        .and_then(|v| v.as_str())
        .unwrap_or("public");
    let monitor_enabled = body
        .get("monitor_enabled")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let monitor_kind = body.get("monitor_kind").and_then(|v| v.as_str());
    let monitor_target = body.get("monitor_target").and_then(|v| v.as_str());
    let monitor_interval_s = body.get("monitor_interval_s").and_then(|v| v.as_i64());

    let max_row: (Option<i64>,) = sqlx::query_as("SELECT MAX(order_index) FROM service")
        .fetch_one(&state.db)
        .await
        .unwrap_or((Some(-1),));
    let order_index = max_row.0.unwrap_or(-1) + 1;

    let result = sqlx::query(
        "INSERT INTO service (id, category_id, name, url, icon, description, is_pinned, order_index, \
         visibility, monitor_enabled, monitor_kind, monitor_target, monitor_interval_s, created_at, updated_at) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(category_id)
    .bind(name)
    .bind(url)
    .bind(icon)
    .bind(description)
    .bind(if is_pinned { 1i64 } else { 0i64 })
    .bind(order_index)
    .bind(visibility)
    .bind(if monitor_enabled { 1i64 } else { 0i64 })
    .bind(monitor_kind)
    .bind(monitor_target)
    .bind(monitor_interval_s)
    .bind(&now)
    .bind(&now)
    .execute(&state.db)
    .await;

    match result {
        Ok(_) => (
            axum::http::StatusCode::CREATED,
            Json(json!({"id": id, "name": name, "url": url})),
        )
            .into_response(),
        Err(e) => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": format!("database error: {e}")})),
        )
            .into_response(),
    }
}

/// PATCH /api/v1/services/{id} — update service (scope: services:write)
pub async fn update_service(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(body): Json<serde_json::Value>,
) -> impl IntoResponse {
    let server_key = state.config.server_key.as_bytes();
    let verified = match verify_bearer(&headers, &state.db, server_key).await {
        Ok(v) => v,
        Err(e) => return e.into_response(),
    };
    if let Err(e) = require_scope(&verified, "services:write") {
        return e.into_response();
    }

    let now = chrono::Utc::now().to_rfc3339();

    if let Some(name) = body.get("name").and_then(|v| v.as_str()) {
        let _ = sqlx::query("UPDATE service SET name = ?, updated_at = ? WHERE id = ?")
            .bind(name)
            .bind(&now)
            .bind(&id)
            .execute(&state.db)
            .await;
    }
    if let Some(url) = body.get("url").and_then(|v| v.as_str()) {
        let _ = sqlx::query("UPDATE service SET url = ?, updated_at = ? WHERE id = ?")
            .bind(url)
            .bind(&now)
            .bind(&id)
            .execute(&state.db)
            .await;
    }
    if let Some(icon) = body.get("icon").and_then(|v| v.as_str()) {
        let _ = sqlx::query("UPDATE service SET icon = ?, updated_at = ? WHERE id = ?")
            .bind(icon)
            .bind(&now)
            .bind(&id)
            .execute(&state.db)
            .await;
    }
    if let Some(description) = body.get("description").and_then(|v| v.as_str()) {
        let _ = sqlx::query("UPDATE service SET description = ?, updated_at = ? WHERE id = ?")
            .bind(description)
            .bind(&now)
            .bind(&id)
            .execute(&state.db)
            .await;
    }
    if let Some(is_pinned) = body.get("is_pinned").and_then(|v| v.as_bool()) {
        let _ = sqlx::query("UPDATE service SET is_pinned = ?, updated_at = ? WHERE id = ?")
            .bind(if is_pinned { 1i64 } else { 0i64 })
            .bind(&now)
            .bind(&id)
            .execute(&state.db)
            .await;
    }
    if let Some(visibility) = body.get("visibility").and_then(|v| v.as_str()) {
        let _ = sqlx::query("UPDATE service SET visibility = ?, updated_at = ? WHERE id = ?")
            .bind(visibility)
            .bind(&now)
            .bind(&id)
            .execute(&state.db)
            .await;
    }

    (
        axum::http::StatusCode::OK,
        Json(json!({"id": id, "updated": true})),
    )
        .into_response()
}

/// DELETE /api/v1/services/{id} — delete service (scope: services:write)
pub async fn delete_service(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let server_key = state.config.server_key.as_bytes();
    let verified = match verify_bearer(&headers, &state.db, server_key).await {
        Ok(v) => v,
        Err(e) => return e.into_response(),
    };
    if let Err(e) = require_scope(&verified, "services:write") {
        return e.into_response();
    }

    let _ = sqlx::query("DELETE FROM service WHERE id = ?")
        .bind(&id)
        .execute(&state.db)
        .await;

    axum::http::StatusCode::NO_CONTENT.into_response()
}

// --- Bookmarks ---

/// GET /api/v1/bookmarks — list all bookmarks (scope: bookmarks:read)
pub async fn list_bookmarks(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let server_key = state.config.server_key.as_bytes();
    let verified = match verify_bearer(&headers, &state.db, server_key).await {
        Ok(v) => v,
        Err(e) => return e.into_response(),
    };
    if let Err(e) = require_scope(&verified, "bookmarks:read") {
        return e.into_response();
    }

    let rows = sqlx::query(
        "SELECT id, category_id, name, url, icon, order_index, visibility, created_at, updated_at \
         FROM bookmark ORDER BY order_index ASC",
    )
    .fetch_all(&state.db)
    .await;

    match rows {
        Ok(rows) => {
            let bookmarks: Vec<serde_json::Value> = rows
                .iter()
                .map(|r| {
                    json!({
                        "id": r.get::<String, _>("id"),
                        "category_id": r.get::<Option<String>, _>("category_id"),
                        "name": r.get::<String, _>("name"),
                        "url": r.get::<String, _>("url"),
                        "icon": r.get::<Option<String>, _>("icon"),
                        "order_index": r.get::<i64, _>("order_index"),
                        "visibility": r.get::<String, _>("visibility"),
                        "created_at": r.get::<String, _>("created_at"),
                        "updated_at": r.get::<String, _>("updated_at"),
                    })
                })
                .collect();
            Json(json!(bookmarks)).into_response()
        }
        Err(_) => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "database error"})),
        )
            .into_response(),
    }
}

// --- Export ---

/// GET /api/v1/export — full export (scope: export)
pub async fn export_all(State(state): State<AppState>, headers: HeaderMap) -> impl IntoResponse {
    let server_key = state.config.server_key.as_bytes();
    let verified = match verify_bearer(&headers, &state.db, server_key).await {
        Ok(v) => v,
        Err(e) => return e.into_response(),
    };
    if let Err(e) = require_scope(&verified, "export") {
        return e.into_response();
    }

    let services = sqlx::query(
        "SELECT id, name, url, icon, description, visibility FROM service ORDER BY order_index ASC",
    )
    .fetch_all(&state.db)
    .await
    .map(|rows| {
        rows.iter()
            .map(|r| {
                json!({
                    "id": r.get::<String, _>("id"),
                    "name": r.get::<String, _>("name"),
                    "url": r.get::<String, _>("url"),
                    "icon": r.get::<Option<String>, _>("icon"),
                    "description": r.get::<Option<String>, _>("description"),
                    "visibility": r.get::<String, _>("visibility"),
                })
            })
            .collect::<Vec<_>>()
    })
    .unwrap_or_default();

    let bookmarks = sqlx::query(
        "SELECT id, name, url, icon, visibility FROM bookmark ORDER BY order_index ASC",
    )
    .fetch_all(&state.db)
    .await
    .map(|rows| {
        rows.iter()
            .map(|r| {
                json!({
                    "id": r.get::<String, _>("id"),
                    "name": r.get::<String, _>("name"),
                    "url": r.get::<String, _>("url"),
                    "icon": r.get::<Option<String>, _>("icon"),
                    "visibility": r.get::<String, _>("visibility"),
                })
            })
            .collect::<Vec<_>>()
    })
    .unwrap_or_default();

    let categories =
        sqlx::query("SELECT id, name, icon, visibility FROM category ORDER BY order_index ASC")
            .fetch_all(&state.db)
            .await
            .map(|rows| {
                rows.iter()
                    .map(|r| {
                        json!({
                            "id": r.get::<String, _>("id"),
                            "name": r.get::<String, _>("name"),
                            "icon": r.get::<Option<String>, _>("icon"),
                            "visibility": r.get::<String, _>("visibility"),
                        })
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

    Json(json!({
        "version": "1.0",
        "exported_at": chrono::Utc::now().to_rfc3339(),
        "categories": categories,
        "services": services,
        "bookmarks": bookmarks,
    }))
    .into_response()
}

/// Build the public API sub-router for /api/v1/*
pub fn public_api_routes() -> Router<AppState> {
    Router::new()
        .route("/api/v1/services", get(list_services).post(create_service))
        .route(
            "/api/v1/services/{id}",
            patch(update_service).delete(delete_service),
        )
        .route("/api/v1/bookmarks", get(list_bookmarks))
        .route("/api/v1/export", get(export_all))
}
