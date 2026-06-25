//! SQL query functions for content reads.
//! All reads exclude private rows for anonymous/unauthorized callers in SQL.
//! Uses static SQL strings (no dynamic format!()) to satisfy sqlx 0.9 SqlSafeStr.

use std::str::FromStr;

use sqlx::{Row, SqlitePool};
use uuid::Uuid;

use crate::domain::{
    Application, Bookmark, Category, CategoryWithBookmarks, CategoryWithItems, DashboardSettings,
    DashboardView, Service, VisibilityFilter,
};

fn parse_uuid(s: &str) -> Uuid {
    Uuid::from_str(s).unwrap_or_default()
}

pub(crate) fn row_to_service(row: &sqlx::sqlite::SqliteRow) -> Service {
    Service {
        id: parse_uuid(row.get("id")),
        category_id: row
            .get::<Option<String>, _>("category_id")
            .map(|s| parse_uuid(&s)),
        name: row.get("name"),
        url: row.get("url"),
        icon: row.get("icon"),
        description: row.get("description"),
        is_pinned: row.get::<i64, _>("is_pinned") != 0,
        order_index: row.get("order_index"),
        visibility: row
            .get::<String, _>("visibility")
            .parse()
            .unwrap_or_default(),
        monitor_enabled: row.get::<i64, _>("monitor_enabled") != 0,
        monitor_kind: row.get("monitor_kind"),
        monitor_target: row.get("monitor_target"),
        monitor_interval_s: row.get("monitor_interval_s"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    }
}

pub(crate) fn row_to_application(row: &sqlx::sqlite::SqliteRow) -> Application {
    Application {
        id: parse_uuid(row.get("id")),
        category_id: row
            .get::<Option<String>, _>("category_id")
            .map(|s| parse_uuid(&s)),
        name: row.get("name"),
        url: row.get("url"),
        icon: row.get("icon"),
        description: row.get("description"),
        is_pinned: row.get::<i64, _>("is_pinned") != 0,
        order_index: row.get("order_index"),
        visibility: row
            .get::<String, _>("visibility")
            .parse()
            .unwrap_or_default(),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    }
}

pub(crate) fn row_to_bookmark(row: &sqlx::sqlite::SqliteRow) -> Bookmark {
    Bookmark {
        id: parse_uuid(row.get("id")),
        category_id: row
            .get::<Option<String>, _>("category_id")
            .map(|s| parse_uuid(&s)),
        name: row.get("name"),
        url: row.get("url"),
        icon: row.get("icon"),
        order_index: row.get("order_index"),
        visibility: row
            .get::<String, _>("visibility")
            .parse()
            .unwrap_or_default(),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    }
}

pub(crate) fn row_to_category(row: &sqlx::sqlite::SqliteRow) -> Category {
    Category {
        id: parse_uuid(row.get("id")),
        name: row.get("name"),
        icon: row.get("icon"),
        order_index: row.get("order_index"),
        visibility: row
            .get::<String, _>("visibility")
            .parse()
            .unwrap_or_default(),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    }
}

pub async fn list_dashboard_query(
    pool: &SqlitePool,
    filter: VisibilityFilter,
) -> Result<DashboardView, sqlx::Error> {
    let service_rows = match filter {
        VisibilityFilter::PublicOnly => sqlx::query(
            "SELECT id, category_id, name, url, icon, description, is_pinned, \
             order_index, visibility, monitor_enabled, monitor_kind, monitor_target, \
             monitor_interval_s, created_at, updated_at \
             FROM service WHERE is_pinned = 1 AND visibility = 'public' ORDER BY order_index",
        ),
        VisibilityFilter::All => sqlx::query(
            "SELECT id, category_id, name, url, icon, description, is_pinned, \
             order_index, visibility, monitor_enabled, monitor_kind, monitor_target, \
             monitor_interval_s, created_at, updated_at \
             FROM service WHERE is_pinned = 1 AND visibility IN ('public', 'private') ORDER BY order_index",
        ),
        VisibilityFilter::AllIncludingRestricted => sqlx::query(
            "SELECT id, category_id, name, url, icon, description, is_pinned, \
             order_index, visibility, monitor_enabled, monitor_kind, monitor_target, \
             monitor_interval_s, created_at, updated_at \
             FROM service WHERE is_pinned = 1 ORDER BY order_index",
        ),
    }
    .fetch_all(pool)
    .await?;

    let pinned_services: Vec<Service> = service_rows.iter().map(row_to_service).collect();

    let category_rows = match filter {
        VisibilityFilter::PublicOnly => sqlx::query(
            "SELECT id, name, icon, order_index, visibility, created_at, updated_at \
             FROM category WHERE visibility = 'public' ORDER BY order_index",
        ),
        VisibilityFilter::All => sqlx::query(
            "SELECT id, name, icon, order_index, visibility, created_at, updated_at \
             FROM category WHERE visibility IN ('public', 'private') ORDER BY order_index",
        ),
        VisibilityFilter::AllIncludingRestricted => sqlx::query(
            "SELECT id, name, icon, order_index, visibility, created_at, updated_at \
             FROM category ORDER BY order_index",
        ),
    }
    .fetch_all(pool)
    .await?;

    let mut pinned_categories: Vec<CategoryWithBookmarks> = Vec::new();
    for cat_row in &category_rows {
        let category = row_to_category(cat_row);
        let cat_id: String = cat_row.get("id");
        let bookmark_rows = match filter {
            VisibilityFilter::PublicOnly => sqlx::query(
                "SELECT id, category_id, name, url, icon, order_index, visibility, \
                 created_at, updated_at FROM bookmark \
                 WHERE category_id = ? AND visibility = 'public' ORDER BY order_index",
            ),
            VisibilityFilter::All => sqlx::query(
                "SELECT id, category_id, name, url, icon, order_index, visibility, \
                 created_at, updated_at FROM bookmark \
                 WHERE category_id = ? AND visibility IN ('public', 'private') ORDER BY order_index",
            ),
            VisibilityFilter::AllIncludingRestricted => sqlx::query(
                "SELECT id, category_id, name, url, icon, order_index, visibility, \
                 created_at, updated_at FROM bookmark \
                 WHERE category_id = ? ORDER BY order_index",
            ),
        }
        .bind(&cat_id)
        .fetch_all(pool)
        .await?;

        let bookmarks: Vec<Bookmark> = bookmark_rows.iter().map(row_to_bookmark).collect();
        pinned_categories.push(CategoryWithBookmarks {
            category,
            bookmarks,
        });
    }

    let application_rows = match filter {
        VisibilityFilter::PublicOnly => sqlx::query(
            "SELECT id, category_id, name, url, icon, description, is_pinned, \
             order_index, visibility, created_at, updated_at \
             FROM application WHERE is_pinned = 1 AND visibility = 'public' ORDER BY order_index",
        ),
        VisibilityFilter::All => sqlx::query(
            "SELECT id, category_id, name, url, icon, description, is_pinned, \
             order_index, visibility, created_at, updated_at \
             FROM application WHERE is_pinned = 1 AND visibility IN ('public', 'private') ORDER BY order_index",
        ),
        VisibilityFilter::AllIncludingRestricted => sqlx::query(
            "SELECT id, category_id, name, url, icon, description, is_pinned, \
             order_index, visibility, created_at, updated_at \
             FROM application WHERE is_pinned = 1 ORDER BY order_index",
        ),
    }
    .fetch_all(pool)
    .await?;

    let applications: Vec<Application> = application_rows.iter().map(row_to_application).collect();

    Ok(DashboardView {
        pinned_services,
        pinned_categories,
        applications,
        settings: DashboardSettings::default(),
    })
}

pub async fn list_categories_query(
    pool: &SqlitePool,
    filter: VisibilityFilter,
) -> Result<Vec<CategoryWithItems>, sqlx::Error> {
    let rows = match filter {
        VisibilityFilter::PublicOnly => sqlx::query(
            "SELECT c.id, c.name, c.icon, c.order_index, c.visibility, c.created_at, c.updated_at, \
             (SELECT COUNT(*) FROM service s WHERE s.category_id = c.id AND s.visibility = 'public') AS service_count, \
             (SELECT COUNT(*) FROM bookmark b WHERE b.category_id = c.id AND b.visibility = 'public') AS bookmark_count \
             FROM category c WHERE c.visibility = 'public' ORDER BY c.order_index",
        ),
        VisibilityFilter::All => sqlx::query(
            "SELECT c.id, c.name, c.icon, c.order_index, c.visibility, c.created_at, c.updated_at, \
             (SELECT COUNT(*) FROM service s WHERE s.category_id = c.id AND s.visibility IN ('public', 'private')) AS service_count, \
             (SELECT COUNT(*) FROM bookmark b WHERE b.category_id = c.id AND b.visibility IN ('public', 'private')) AS bookmark_count \
             FROM category c WHERE c.visibility IN ('public', 'private') ORDER BY c.order_index",
        ),
        VisibilityFilter::AllIncludingRestricted => sqlx::query(
            "SELECT c.id, c.name, c.icon, c.order_index, c.visibility, c.created_at, c.updated_at, \
             (SELECT COUNT(*) FROM service s WHERE s.category_id = c.id) AS service_count, \
             (SELECT COUNT(*) FROM bookmark b WHERE b.category_id = c.id) AS bookmark_count \
             FROM category c ORDER BY c.order_index",
        ),
    }
    .fetch_all(pool)
    .await?;

    Ok(rows
        .iter()
        .map(|row| CategoryWithItems {
            id: parse_uuid(row.get("id")),
            name: row.get("name"),
            icon: row.get("icon"),
            order_index: row.get("order_index"),
            visibility: row
                .get::<String, _>("visibility")
                .parse()
                .unwrap_or_default(),
            service_count: row.get("service_count"),
            bookmark_count: row.get("bookmark_count"),
        })
        .collect())
}

pub async fn list_services_query(
    pool: &SqlitePool,
    category_id: Option<Uuid>,
    filter: VisibilityFilter,
) -> Result<Vec<Service>, sqlx::Error> {
    let rows = match (category_id, filter) {
        (Some(cat_id), VisibilityFilter::PublicOnly) => {
            sqlx::query(
                "SELECT id, category_id, name, url, icon, description, is_pinned, \
                 order_index, visibility, monitor_enabled, monitor_kind, monitor_target, \
                 monitor_interval_s, created_at, updated_at \
                 FROM service WHERE category_id = ? AND visibility = 'public' ORDER BY order_index",
            )
            .bind(cat_id.to_string())
            .fetch_all(pool)
            .await?
        }
        (Some(cat_id), VisibilityFilter::All) => {
            sqlx::query(
                "SELECT id, category_id, name, url, icon, description, is_pinned, \
                 order_index, visibility, monitor_enabled, monitor_kind, monitor_target, \
                 monitor_interval_s, created_at, updated_at \
                 FROM service WHERE category_id = ? AND visibility IN ('public', 'private') ORDER BY order_index",
            )
            .bind(cat_id.to_string())
            .fetch_all(pool)
            .await?
        }
        (Some(cat_id), VisibilityFilter::AllIncludingRestricted) => {
            sqlx::query(
                "SELECT id, category_id, name, url, icon, description, is_pinned, \
                 order_index, visibility, monitor_enabled, monitor_kind, monitor_target, \
                 monitor_interval_s, created_at, updated_at \
                 FROM service WHERE category_id = ? ORDER BY order_index",
            )
            .bind(cat_id.to_string())
            .fetch_all(pool)
            .await?
        }
        (None, VisibilityFilter::PublicOnly) => {
            sqlx::query(
                "SELECT id, category_id, name, url, icon, description, is_pinned, \
                 order_index, visibility, monitor_enabled, monitor_kind, monitor_target, \
                 monitor_interval_s, created_at, updated_at \
                 FROM service WHERE visibility = 'public' ORDER BY order_index",
            )
            .fetch_all(pool)
            .await?
        }
        (None, VisibilityFilter::All) => {
            sqlx::query(
                "SELECT id, category_id, name, url, icon, description, is_pinned, \
                 order_index, visibility, monitor_enabled, monitor_kind, monitor_target, \
                 monitor_interval_s, created_at, updated_at \
                 FROM service WHERE visibility IN ('public', 'private') ORDER BY order_index",
            )
            .fetch_all(pool)
            .await?
        }
        (None, VisibilityFilter::AllIncludingRestricted) => {
            sqlx::query(
                "SELECT id, category_id, name, url, icon, description, is_pinned, \
                 order_index, visibility, monitor_enabled, monitor_kind, monitor_target, \
                 monitor_interval_s, created_at, updated_at \
                 FROM service ORDER BY order_index",
            )
            .fetch_all(pool)
            .await?
        }
    };

    Ok(rows.iter().map(row_to_service).collect())
}

pub async fn list_applications_query(
    pool: &SqlitePool,
    category_id: Option<Uuid>,
    filter: VisibilityFilter,
) -> Result<Vec<Application>, sqlx::Error> {
    let rows = match (category_id, filter) {
        (Some(cat_id), VisibilityFilter::PublicOnly) => {
            sqlx::query(
                "SELECT id, category_id, name, url, icon, description, is_pinned, \
                 order_index, visibility, created_at, updated_at \
                 FROM application WHERE category_id = ? AND visibility = 'public' ORDER BY order_index",
            )
            .bind(cat_id.to_string())
            .fetch_all(pool)
            .await?
        }
        (Some(cat_id), VisibilityFilter::All) => {
            sqlx::query(
                "SELECT id, category_id, name, url, icon, description, is_pinned, \
                 order_index, visibility, created_at, updated_at \
                 FROM application WHERE category_id = ? AND visibility IN ('public', 'private') ORDER BY order_index",
            )
            .bind(cat_id.to_string())
            .fetch_all(pool)
            .await?
        }
        (Some(cat_id), VisibilityFilter::AllIncludingRestricted) => {
            sqlx::query(
                "SELECT id, category_id, name, url, icon, description, is_pinned, \
                 order_index, visibility, created_at, updated_at \
                 FROM application WHERE category_id = ? ORDER BY order_index",
            )
            .bind(cat_id.to_string())
            .fetch_all(pool)
            .await?
        }
        (None, VisibilityFilter::PublicOnly) => {
            sqlx::query(
                "SELECT id, category_id, name, url, icon, description, is_pinned, \
                 order_index, visibility, created_at, updated_at \
                 FROM application WHERE visibility = 'public' ORDER BY order_index",
            )
            .fetch_all(pool)
            .await?
        }
        (None, VisibilityFilter::All) => {
            sqlx::query(
                "SELECT id, category_id, name, url, icon, description, is_pinned, \
                 order_index, visibility, created_at, updated_at \
                 FROM application WHERE visibility IN ('public', 'private') ORDER BY order_index",
            )
            .fetch_all(pool)
            .await?
        }
        (None, VisibilityFilter::AllIncludingRestricted) => {
            sqlx::query(
                "SELECT id, category_id, name, url, icon, description, is_pinned, \
                 order_index, visibility, created_at, updated_at \
                 FROM application ORDER BY order_index",
            )
            .fetch_all(pool)
            .await?
        }
    };

    Ok(rows.iter().map(row_to_application).collect())
}

pub async fn list_bookmarks_query(
    pool: &SqlitePool,
    category_id: Option<Uuid>,
    filter: VisibilityFilter,
) -> Result<Vec<Bookmark>, sqlx::Error> {
    let rows = match (category_id, filter) {
        (Some(cat_id), VisibilityFilter::PublicOnly) => {
            sqlx::query(
                "SELECT id, category_id, name, url, icon, order_index, visibility, \
                 created_at, updated_at FROM bookmark \
                 WHERE category_id = ? AND visibility = 'public' ORDER BY order_index",
            )
            .bind(cat_id.to_string())
            .fetch_all(pool)
            .await?
        }
        (Some(cat_id), VisibilityFilter::All) => {
            sqlx::query(
                "SELECT id, category_id, name, url, icon, order_index, visibility, \
                 created_at, updated_at FROM bookmark \
                 WHERE category_id = ? AND visibility IN ('public', 'private') ORDER BY order_index",
            )
            .bind(cat_id.to_string())
            .fetch_all(pool)
            .await?
        }
        (Some(cat_id), VisibilityFilter::AllIncludingRestricted) => {
            sqlx::query(
                "SELECT id, category_id, name, url, icon, order_index, visibility, \
                 created_at, updated_at FROM bookmark \
                 WHERE category_id = ? ORDER BY order_index",
            )
            .bind(cat_id.to_string())
            .fetch_all(pool)
            .await?
        }
        (None, VisibilityFilter::PublicOnly) => {
            sqlx::query(
                "SELECT id, category_id, name, url, icon, order_index, visibility, \
                 created_at, updated_at FROM bookmark \
                 WHERE visibility = 'public' ORDER BY order_index",
            )
            .fetch_all(pool)
            .await?
        }
        (None, VisibilityFilter::All) => {
            sqlx::query(
                "SELECT id, category_id, name, url, icon, order_index, visibility, \
                 created_at, updated_at FROM bookmark \
                 WHERE visibility IN ('public', 'private') ORDER BY order_index",
            )
            .fetch_all(pool)
            .await?
        }
        (None, VisibilityFilter::AllIncludingRestricted) => {
            sqlx::query(
                "SELECT id, category_id, name, url, icon, order_index, visibility, \
                 created_at, updated_at FROM bookmark ORDER BY order_index",
            )
            .fetch_all(pool)
            .await?
        }
    };

    Ok(rows.iter().map(row_to_bookmark).collect())
}

pub async fn get_search_providers_query(
    pool: &SqlitePool,
) -> Result<crate::domain::SearchProviderConfig, sqlx::Error> {
    let row: Option<(String,)> =
        sqlx::query_as("SELECT value FROM setting WHERE key = 'search.providers'")
            .fetch_optional(pool)
            .await?;

    Ok(match row {
        Some((json,)) => serde_json::from_str(&json).unwrap_or_default(),
        None => crate::domain::SearchProviderConfig::default(),
    })
}
