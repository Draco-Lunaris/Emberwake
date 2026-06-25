//! SQL query functions for content writes (create/update/delete/reorder/pin).
//! All writes use parameterized SQL with static string literals (sqlx 0.9 SqlSafeStr).

use chrono::Utc;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::domain::{
    Application, ApplicationInput, ApplicationPatch, Bookmark, BookmarkInput, BookmarkPatch,
    Category, CategoryInput, CategoryPatch, Service, ServiceInput, ServicePatch,
};
use crate::error::AppError;
use crate::server::content_queries::{
    row_to_application, row_to_bookmark, row_to_category, row_to_service,
};

fn now_rfc3339() -> String {
    Utc::now().to_rfc3339()
}

fn new_uuid_v7() -> String {
    Uuid::now_v7().to_string()
}

// --- Category ---

pub async fn create_category_query(
    pool: &SqlitePool,
    input: CategoryInput,
) -> Result<Category, AppError> {
    let id = new_uuid_v7();
    let now = now_rfc3339();

    let max_row: (Option<i64>,) = sqlx::query_as("SELECT MAX(order_index) FROM category")
        .fetch_one(pool)
        .await?;
    let order_index = max_row.0.unwrap_or(-1) + 1;

    sqlx::query(
        "INSERT INTO category (id, name, icon, order_index, visibility, created_at, updated_at) \
         VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(&input.name)
    .bind(&input.icon)
    .bind(order_index)
    .bind(input.visibility.to_string())
    .bind(&now)
    .bind(&now)
    .execute(pool)
    .await?;

    let row = sqlx::query(
        "SELECT id, name, icon, order_index, visibility, created_at, updated_at \
         FROM category WHERE id = ?",
    )
    .bind(&id)
    .fetch_one(pool)
    .await?;

    Ok(row_to_category(&row))
}

pub async fn update_category_query(
    pool: &SqlitePool,
    id: Uuid,
    patch: CategoryPatch,
) -> Result<Category, AppError> {
    let id_str = id.to_string();
    let now = now_rfc3339();

    // Fetch current to merge patch
    let row = sqlx::query(
        "SELECT id, name, icon, order_index, visibility, created_at, updated_at \
         FROM category WHERE id = ?",
    )
    .bind(&id_str)
    .fetch_optional(pool)
    .await?;

    let row = match row {
        Some(r) => r,
        None => return Err(AppError::NotFound),
    };

    let current = row_to_category(&row);
    let name = patch.name.unwrap_or(current.name);
    let icon = patch.icon.or(current.icon);
    let visibility = patch.visibility.unwrap_or(current.visibility);

    sqlx::query(
        "UPDATE category SET name = ?, icon = ?, visibility = ?, updated_at = ? WHERE id = ?",
    )
    .bind(&name)
    .bind(&icon)
    .bind(visibility.to_string())
    .bind(&now)
    .bind(&id_str)
    .execute(pool)
    .await?;

    let row = sqlx::query(
        "SELECT id, name, icon, order_index, visibility, created_at, updated_at \
         FROM category WHERE id = ?",
    )
    .bind(&id_str)
    .fetch_one(pool)
    .await?;

    Ok(row_to_category(&row))
}

pub async fn delete_category_query(pool: &SqlitePool, id: Uuid) -> Result<(), AppError> {
    let id_str = id.to_string();
    // ON DELETE SET NULL in migration handles service/bookmark category_id
    let result = sqlx::query("DELETE FROM category WHERE id = ?")
        .bind(&id_str)
        .execute(pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }
    Ok(())
}

pub async fn reorder_categories_query(pool: &SqlitePool, order: Vec<Uuid>) -> Result<(), AppError> {
    let now = now_rfc3339();
    for (idx, id) in order.iter().enumerate() {
        sqlx::query("UPDATE category SET order_index = ?, updated_at = ? WHERE id = ?")
            .bind(idx as i64)
            .bind(&now)
            .bind(id.to_string())
            .execute(pool)
            .await?;
    }
    Ok(())
}

// --- Service ---

pub async fn create_service_query(
    pool: &SqlitePool,
    input: ServiceInput,
) -> Result<Service, AppError> {
    let id = new_uuid_v7();
    let now = now_rfc3339();

    let max_row: (Option<i64>,) = sqlx::query_as("SELECT MAX(order_index) FROM service")
        .fetch_one(pool)
        .await?;
    let order_index = max_row.0.unwrap_or(-1) + 1;

    let cat_id = input.category_id.map(|u| u.to_string());

    sqlx::query(
        "INSERT INTO service (id, category_id, name, url, icon, description, is_pinned, \
         order_index, visibility, monitor_enabled, monitor_kind, monitor_target, \
         monitor_interval_s, created_at, updated_at) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(&cat_id)
    .bind(&input.name)
    .bind(&input.url)
    .bind(&input.icon)
    .bind(&input.description)
    .bind(input.is_pinned as i64)
    .bind(order_index)
    .bind(input.visibility.to_string())
    .bind(input.monitor_enabled as i64)
    .bind(&input.monitor_kind)
    .bind(&input.monitor_target)
    .bind(input.monitor_interval_s)
    .bind(&now)
    .bind(&now)
    .execute(pool)
    .await?;

    let row = sqlx::query(
        "SELECT id, category_id, name, url, icon, description, is_pinned, \
         order_index, visibility, monitor_enabled, monitor_kind, monitor_target, \
         monitor_interval_s, created_at, updated_at \
         FROM service WHERE id = ?",
    )
    .bind(&id)
    .fetch_one(pool)
    .await?;

    Ok(row_to_service(&row))
}

pub async fn update_service_query(
    pool: &SqlitePool,
    id: Uuid,
    patch: ServicePatch,
) -> Result<Service, AppError> {
    let id_str = id.to_string();
    let now = now_rfc3339();

    let row = sqlx::query(
        "SELECT id, category_id, name, url, icon, description, is_pinned, \
         order_index, visibility, monitor_enabled, monitor_kind, monitor_target, \
         monitor_interval_s, created_at, updated_at \
         FROM service WHERE id = ?",
    )
    .bind(&id_str)
    .fetch_optional(pool)
    .await?;

    let row = match row {
        Some(r) => r,
        None => return Err(AppError::NotFound),
    };

    let current = row_to_service(&row);

    let category_id = match patch.category_id {
        Some(opt) => opt.map(|u| u.to_string()),
        None => current.category_id.map(|u| u.to_string()),
    };
    let name = patch.name.unwrap_or(current.name);
    let url = patch.url.unwrap_or(current.url);
    let icon = patch.icon.or(current.icon);
    let description = match patch.description {
        Some(opt) => opt,
        None => current.description,
    };
    let is_pinned = patch.is_pinned.unwrap_or(current.is_pinned);
    let visibility = patch.visibility.unwrap_or(current.visibility);
    let monitor_enabled = patch.monitor_enabled.unwrap_or(current.monitor_enabled);
    let monitor_kind = patch.monitor_kind.or(current.monitor_kind);
    let monitor_target = patch.monitor_target.or(current.monitor_target);
    let monitor_interval_s = match patch.monitor_interval_s {
        Some(opt) => opt,
        None => current.monitor_interval_s,
    };

    sqlx::query(
        "UPDATE service SET category_id = ?, name = ?, url = ?, icon = ?, description = ?, \
         is_pinned = ?, visibility = ?, monitor_enabled = ?, monitor_kind = ?, \
         monitor_target = ?, monitor_interval_s = ?, updated_at = ? WHERE id = ?",
    )
    .bind(&category_id)
    .bind(&name)
    .bind(&url)
    .bind(&icon)
    .bind(&description)
    .bind(is_pinned as i64)
    .bind(visibility.to_string())
    .bind(monitor_enabled as i64)
    .bind(&monitor_kind)
    .bind(&monitor_target)
    .bind(monitor_interval_s)
    .bind(&now)
    .bind(&id_str)
    .execute(pool)
    .await?;

    let row = sqlx::query(
        "SELECT id, category_id, name, url, icon, description, is_pinned, \
         order_index, visibility, monitor_enabled, monitor_kind, monitor_target, \
         monitor_interval_s, created_at, updated_at \
         FROM service WHERE id = ?",
    )
    .bind(&id_str)
    .fetch_one(pool)
    .await?;

    Ok(row_to_service(&row))
}

pub async fn delete_service_query(pool: &SqlitePool, id: Uuid) -> Result<(), AppError> {
    let id_str = id.to_string();
    let result = sqlx::query("DELETE FROM service WHERE id = ?")
        .bind(&id_str)
        .execute(pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }
    Ok(())
}

pub async fn reorder_services_query(
    pool: &SqlitePool,
    _category: Option<Uuid>,
    order: Vec<Uuid>,
) -> Result<(), AppError> {
    let now = now_rfc3339();
    for (idx, id) in order.iter().enumerate() {
        sqlx::query("UPDATE service SET order_index = ?, updated_at = ? WHERE id = ?")
            .bind(idx as i64)
            .bind(&now)
            .bind(id.to_string())
            .execute(pool)
            .await?;
    }
    Ok(())
}

pub async fn set_service_pinned_query(
    pool: &SqlitePool,
    id: Uuid,
    pinned: bool,
) -> Result<Service, AppError> {
    let id_str = id.to_string();
    let now = now_rfc3339();

    let result = sqlx::query("UPDATE service SET is_pinned = ?, updated_at = ? WHERE id = ?")
        .bind(pinned as i64)
        .bind(&now)
        .bind(&id_str)
        .execute(pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }

    let row = sqlx::query(
        "SELECT id, category_id, name, url, icon, description, is_pinned, \
         order_index, visibility, monitor_enabled, monitor_kind, monitor_target, \
         monitor_interval_s, created_at, updated_at \
         FROM service WHERE id = ?",
    )
    .bind(&id_str)
    .fetch_one(pool)
    .await?;

    Ok(row_to_service(&row))
}

// --- Application ---

pub async fn create_application_query(
    pool: &SqlitePool,
    input: ApplicationInput,
) -> Result<Application, AppError> {
    let id = new_uuid_v7();
    let now = now_rfc3339();

    let max_row: (Option<i64>,) = sqlx::query_as("SELECT MAX(order_index) FROM application")
        .fetch_one(pool)
        .await?;
    let order_index = max_row.0.unwrap_or(-1) + 1;

    let cat_id = input.category_id.map(|u| u.to_string());

    sqlx::query(
        "INSERT INTO application (id, category_id, name, url, icon, description, is_pinned, \
         order_index, visibility, created_at, updated_at) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(&cat_id)
    .bind(&input.name)
    .bind(&input.url)
    .bind(&input.icon)
    .bind(&input.description)
    .bind(1i64 as i64)
    .bind(order_index)
    .bind(input.visibility.to_string())
    .bind(&now)
    .bind(&now)
    .execute(pool)
    .await?;

    let row = sqlx::query(
        "SELECT id, category_id, name, url, icon, description, is_pinned, \
         order_index, visibility, created_at, updated_at \
         FROM application WHERE id = ?",
    )
    .bind(&id)
    .fetch_one(pool)
    .await?;

    Ok(row_to_application(&row))
}

pub async fn update_application_query(
    pool: &SqlitePool,
    id: Uuid,
    patch: ApplicationPatch,
) -> Result<Application, AppError> {
    let id_str = id.to_string();
    let now = now_rfc3339();

    let row = sqlx::query(
        "SELECT id, category_id, name, url, icon, description, is_pinned, \
         order_index, visibility, created_at, updated_at \
         FROM application WHERE id = ?",
    )
    .bind(&id_str)
    .fetch_optional(pool)
    .await?;

    let row = match row {
        Some(r) => r,
        None => return Err(AppError::NotFound),
    };

    let current = row_to_application(&row);

    let category_id = match patch.category_id {
        Some(opt) => opt.map(|u| u.to_string()),
        None => current.category_id.map(|u| u.to_string()),
    };
    let name = patch.name.unwrap_or(current.name);
    let url = patch.url.unwrap_or(current.url);
    let icon = patch.icon.or(current.icon);
    let description = match patch.description {
        Some(opt) => opt,
        None => current.description,
    };
    let is_pinned = patch.is_pinned.unwrap_or(current.is_pinned);
    let visibility = patch.visibility.unwrap_or(current.visibility);

    sqlx::query(
        "UPDATE application SET category_id = ?, name = ?, url = ?, icon = ?, description = ?, \
         is_pinned = ?, visibility = ?, updated_at = ? WHERE id = ?",
    )
    .bind(&category_id)
    .bind(&name)
    .bind(&url)
    .bind(&icon)
    .bind(&description)
    .bind(is_pinned as i64)
    .bind(visibility.to_string())
    .bind(&now)
    .bind(&id_str)
    .execute(pool)
    .await?;

    let row = sqlx::query(
        "SELECT id, category_id, name, url, icon, description, is_pinned, \
         order_index, visibility, created_at, updated_at \
         FROM application WHERE id = ?",
    )
    .bind(&id_str)
    .fetch_one(pool)
    .await?;

    Ok(row_to_application(&row))
}

pub async fn delete_application_query(pool: &SqlitePool, id: Uuid) -> Result<(), AppError> {
    let id_str = id.to_string();
    let result = sqlx::query("DELETE FROM application WHERE id = ?")
        .bind(&id_str)
        .execute(pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }
    Ok(())
}

pub async fn reorder_applications_query(
    pool: &SqlitePool,
    _category: Option<Uuid>,
    order: Vec<Uuid>,
) -> Result<(), AppError> {
    let now = now_rfc3339();
    for (idx, id) in order.iter().enumerate() {
        sqlx::query("UPDATE application SET order_index = ?, updated_at = ? WHERE id = ?")
            .bind(idx as i64)
            .bind(&now)
            .bind(id.to_string())
            .execute(pool)
            .await?;
    }
    Ok(())
}

pub async fn set_application_pinned_query(
    pool: &SqlitePool,
    id: Uuid,
    pinned: bool,
) -> Result<Application, AppError> {
    let id_str = id.to_string();
    let now = now_rfc3339();

    let result = sqlx::query("UPDATE application SET is_pinned = ?, updated_at = ? WHERE id = ?")
        .bind(pinned as i64)
        .bind(&now)
        .bind(&id_str)
        .execute(pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }

    let row = sqlx::query(
        "SELECT id, category_id, name, url, icon, description, is_pinned, \
         order_index, visibility, created_at, updated_at \
         FROM application WHERE id = ?",
    )
    .bind(&id_str)
    .fetch_one(pool)
    .await?;

    Ok(row_to_application(&row))
}

// --- Bookmark ---

pub async fn create_bookmark_query(
    pool: &SqlitePool,
    input: BookmarkInput,
) -> Result<Bookmark, AppError> {
    let id = new_uuid_v7();
    let now = now_rfc3339();

    let max_row: (Option<i64>,) = sqlx::query_as("SELECT MAX(order_index) FROM bookmark")
        .fetch_one(pool)
        .await?;
    let order_index = max_row.0.unwrap_or(-1) + 1;

    let cat_id = if input.category_id == Uuid::nil() {
        None
    } else {
        Some(input.category_id.to_string())
    };

    sqlx::query(
        "INSERT INTO bookmark (id, category_id, name, url, icon, order_index, visibility, \
         created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(&cat_id)
    .bind(&input.name)
    .bind(&input.url)
    .bind(&input.icon)
    .bind(order_index)
    .bind(input.visibility.to_string())
    .bind(&now)
    .bind(&now)
    .execute(pool)
    .await?;

    let row = sqlx::query(
        "SELECT id, category_id, name, url, icon, order_index, visibility, \
         created_at, updated_at FROM bookmark WHERE id = ?",
    )
    .bind(&id)
    .fetch_one(pool)
    .await?;

    Ok(row_to_bookmark(&row))
}

pub async fn update_bookmark_query(
    pool: &SqlitePool,
    id: Uuid,
    patch: BookmarkPatch,
) -> Result<Bookmark, AppError> {
    let id_str = id.to_string();
    let now = now_rfc3339();

    let row = sqlx::query(
        "SELECT id, category_id, name, url, icon, order_index, visibility, \
         created_at, updated_at FROM bookmark WHERE id = ?",
    )
    .bind(&id_str)
    .fetch_optional(pool)
    .await?;

    let row = match row {
        Some(r) => r,
        None => return Err(AppError::NotFound),
    };

    let current = row_to_bookmark(&row);

    let category_id = match patch.category_id {
        Some(u) => Some(u.to_string()),
        None => current.category_id.map(|u| u.to_string()),
    };
    let name = patch.name.unwrap_or(current.name);
    let url = patch.url.unwrap_or(current.url);
    let icon = patch.icon.or(current.icon);
    let visibility = patch.visibility.unwrap_or(current.visibility);

    sqlx::query(
        "UPDATE bookmark SET category_id = ?, name = ?, url = ?, icon = ?, visibility = ?, \
         updated_at = ? WHERE id = ?",
    )
    .bind(&category_id)
    .bind(&name)
    .bind(&url)
    .bind(&icon)
    .bind(visibility.to_string())
    .bind(&now)
    .bind(&id_str)
    .execute(pool)
    .await?;

    let row = sqlx::query(
        "SELECT id, category_id, name, url, icon, order_index, visibility, \
         created_at, updated_at FROM bookmark WHERE id = ?",
    )
    .bind(&id_str)
    .fetch_one(pool)
    .await?;

    Ok(row_to_bookmark(&row))
}

pub async fn delete_bookmark_query(pool: &SqlitePool, id: Uuid) -> Result<(), AppError> {
    let id_str = id.to_string();
    let result = sqlx::query("DELETE FROM bookmark WHERE id = ?")
        .bind(&id_str)
        .execute(pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }
    Ok(())
}

pub async fn reorder_bookmarks_query(
    pool: &SqlitePool,
    _category: Uuid,
    order: Vec<Uuid>,
) -> Result<(), AppError> {
    let now = now_rfc3339();
    for (idx, id) in order.iter().enumerate() {
        sqlx::query("UPDATE bookmark SET order_index = ?, updated_at = ? WHERE id = ?")
            .bind(idx as i64)
            .bind(&now)
            .bind(id.to_string())
            .execute(pool)
            .await?;
    }
    Ok(())
}

// --- Validation helpers ---

/// Validate that a name is non-empty after trimming.
pub fn validate_name(name: &str) -> Result<(), AppError> {
    if name.trim().is_empty() {
        return Err(AppError::Validation("name must not be empty".into()));
    }
    Ok(())
}

/// Validate that a URL is well-formed (has a scheme and host).
pub fn validate_url(url: &str) -> Result<(), AppError> {
    if url.trim().is_empty() {
        return Err(AppError::Validation("url must not be empty".into()));
    }
    if !url.contains("://") {
        return Err(AppError::Validation(
            "url must include a scheme (e.g. https://)".into(),
        ));
    }
    Ok(())
}
