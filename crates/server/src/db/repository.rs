//! Repository trait + SQLite implementation.
//! Read methods delegate to app::server::content_queries for shared SQL logic.
//! Future Postgres swap is enabled by the trait abstraction.

use async_trait::async_trait;
use sqlx::SqlitePool;
use uuid::Uuid;

use app::domain::{
    Bookmark, BookmarkInput, BookmarkPatch, Category, CategoryInput, CategoryPatch,
    CategoryWithItems, DashboardView, Service, ServiceInput, ServicePatch, VisibilityFilter,
};

/// Repository trait — abstracts data access for future Postgres swap.
/// Methods will be added as user stories are implemented (Phase 3+).
#[async_trait]
pub trait Repository: Send + Sync {
    /// Check if initial admin setup has been completed.
    async fn is_setup_complete(&self) -> Result<bool, sqlx::Error>;

    /// Count users in the database.
    async fn user_count(&self) -> Result<i64, sqlx::Error>;

    /// List dashboard: pinned services + pinned categories with bookmarks.
    async fn list_dashboard(&self, filter: VisibilityFilter) -> Result<DashboardView, sqlx::Error>;

    /// List categories with item counts.
    async fn list_categories(
        &self,
        filter: VisibilityFilter,
    ) -> Result<Vec<CategoryWithItems>, sqlx::Error>;

    /// List services with optional category filter.
    async fn list_services(
        &self,
        category_id: Option<Uuid>,
        filter: VisibilityFilter,
    ) -> Result<Vec<Service>, sqlx::Error>;

    /// List bookmarks optionally filtered by category.
    async fn list_bookmarks(
        &self,
        category_id: Option<Uuid>,
        filter: VisibilityFilter,
    ) -> Result<Vec<Bookmark>, sqlx::Error>;

    // --- Write methods (T028) ---

    /// Create a category with generated UUIDv7 and auto order_index.
    async fn create_category(&self, input: CategoryInput)
    -> Result<Category, app::error::AppError>;

    /// Update a category by applying a partial patch.
    async fn update_category(
        &self,
        id: Uuid,
        patch: CategoryPatch,
    ) -> Result<Category, app::error::AppError>;

    /// Delete a category. Services/bookmarks under it get category_id set to NULL.
    async fn delete_category(&self, id: Uuid) -> Result<(), app::error::AppError>;

    /// Reorder categories by updating order_index for each id in the given list.
    async fn reorder_categories(&self, order: Vec<Uuid>) -> Result<(), app::error::AppError>;

    /// Create a service with generated UUIDv7 and auto order_index.
    async fn create_service(&self, input: ServiceInput) -> Result<Service, app::error::AppError>;

    /// Update a service by applying a partial patch.
    async fn update_service(
        &self,
        id: Uuid,
        patch: ServicePatch,
    ) -> Result<Service, app::error::AppError>;

    /// Delete a service.
    async fn delete_service(&self, id: Uuid) -> Result<(), app::error::AppError>;

    /// Reorder services by updating order_index for each id in the given list.
    async fn reorder_services(
        &self,
        category: Option<Uuid>,
        order: Vec<Uuid>,
    ) -> Result<(), app::error::AppError>;

    /// Toggle a service's pinned state.
    async fn set_service_pinned(
        &self,
        id: Uuid,
        pinned: bool,
    ) -> Result<Service, app::error::AppError>;

    /// Create a bookmark with generated UUIDv7 and auto order_index.
    async fn create_bookmark(&self, input: BookmarkInput)
    -> Result<Bookmark, app::error::AppError>;

    /// Update a bookmark by applying a partial patch.
    async fn update_bookmark(
        &self,
        id: Uuid,
        patch: BookmarkPatch,
    ) -> Result<Bookmark, app::error::AppError>;

    /// Delete a bookmark.
    async fn delete_bookmark(&self, id: Uuid) -> Result<(), app::error::AppError>;

    /// Reorder bookmarks by updating order_index for each id in the given list.
    async fn reorder_bookmarks(
        &self,
        category: Uuid,
        order: Vec<Uuid>,
    ) -> Result<(), app::error::AppError>;
}

/// SQLite implementation of the Repository trait.
pub struct SqliteRepository {
    pool: SqlitePool,
}

impl SqliteRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Direct access to the pool for queries not yet abstracted.
    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }
}

#[async_trait]
impl Repository for SqliteRepository {
    async fn is_setup_complete(&self) -> Result<bool, sqlx::Error> {
        let row: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM setting WHERE key = 'setup_complete'")
                .fetch_one(&self.pool)
                .await?;
        Ok(row.0 > 0)
    }

    async fn user_count(&self) -> Result<i64, sqlx::Error> {
        let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users")
            .fetch_one(&self.pool)
            .await?;
        Ok(row.0)
    }

    async fn list_dashboard(&self, filter: VisibilityFilter) -> Result<DashboardView, sqlx::Error> {
        app::server::content_queries::list_dashboard_query(&self.pool, filter).await
    }

    async fn list_categories(
        &self,
        filter: VisibilityFilter,
    ) -> Result<Vec<CategoryWithItems>, sqlx::Error> {
        app::server::content_queries::list_categories_query(&self.pool, filter).await
    }

    async fn list_services(
        &self,
        category_id: Option<Uuid>,
        filter: VisibilityFilter,
    ) -> Result<Vec<Service>, sqlx::Error> {
        app::server::content_queries::list_services_query(&self.pool, category_id, filter).await
    }

    async fn list_bookmarks(
        &self,
        category_id: Option<Uuid>,
        filter: VisibilityFilter,
    ) -> Result<Vec<Bookmark>, sqlx::Error> {
        app::server::content_queries::list_bookmarks_query(&self.pool, category_id, filter).await
    }

    // --- Write methods (T028) ---

    async fn create_category(
        &self,
        input: CategoryInput,
    ) -> Result<Category, app::error::AppError> {
        app::server::content_write_queries::create_category_query(&self.pool, input).await
    }

    async fn update_category(
        &self,
        id: Uuid,
        patch: CategoryPatch,
    ) -> Result<Category, app::error::AppError> {
        app::server::content_write_queries::update_category_query(&self.pool, id, patch).await
    }

    async fn delete_category(&self, id: Uuid) -> Result<(), app::error::AppError> {
        app::server::content_write_queries::delete_category_query(&self.pool, id).await
    }

    async fn reorder_categories(&self, order: Vec<Uuid>) -> Result<(), app::error::AppError> {
        app::server::content_write_queries::reorder_categories_query(&self.pool, order).await
    }

    async fn create_service(&self, input: ServiceInput) -> Result<Service, app::error::AppError> {
        app::server::content_write_queries::create_service_query(&self.pool, input).await
    }

    async fn update_service(
        &self,
        id: Uuid,
        patch: ServicePatch,
    ) -> Result<Service, app::error::AppError> {
        app::server::content_write_queries::update_service_query(&self.pool, id, patch).await
    }

    async fn delete_service(&self, id: Uuid) -> Result<(), app::error::AppError> {
        app::server::content_write_queries::delete_service_query(&self.pool, id).await
    }

    async fn reorder_services(
        &self,
        category: Option<Uuid>,
        order: Vec<Uuid>,
    ) -> Result<(), app::error::AppError> {
        app::server::content_write_queries::reorder_services_query(&self.pool, category, order)
            .await
    }

    async fn set_service_pinned(
        &self,
        id: Uuid,
        pinned: bool,
    ) -> Result<Service, app::error::AppError> {
        app::server::content_write_queries::set_service_pinned_query(&self.pool, id, pinned).await
    }

    async fn create_bookmark(
        &self,
        input: BookmarkInput,
    ) -> Result<Bookmark, app::error::AppError> {
        app::server::content_write_queries::create_bookmark_query(&self.pool, input).await
    }

    async fn update_bookmark(
        &self,
        id: Uuid,
        patch: BookmarkPatch,
    ) -> Result<Bookmark, app::error::AppError> {
        app::server::content_write_queries::update_bookmark_query(&self.pool, id, patch).await
    }

    async fn delete_bookmark(&self, id: Uuid) -> Result<(), app::error::AppError> {
        app::server::content_write_queries::delete_bookmark_query(&self.pool, id).await
    }

    async fn reorder_bookmarks(
        &self,
        category: Uuid,
        order: Vec<Uuid>,
    ) -> Result<(), app::error::AppError> {
        app::server::content_write_queries::reorder_bookmarks_query(&self.pool, category, order)
            .await
    }
}
