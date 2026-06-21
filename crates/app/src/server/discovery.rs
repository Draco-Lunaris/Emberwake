//! Discovery server functions and cache (US8).
//! DiscoveryCache is shared via Axum Extension between background tasks
//! (which populate it) and server functions (which read it).
//! Both discover_docker and discover_kubernetes are admin-gated and
//! return empty vec when the integration is disabled (no calls made).

use std::sync::{Arc, RwLock};

use crate::domain::DiscoveredService;

/// Thread-safe cache for discovered services from Docker and K8s.
/// Cloneable (inner is Arc) — shared between background tasks and server functions.
#[derive(Clone)]
pub struct DiscoveryCache {
    docker: Arc<RwLock<Vec<DiscoveredService>>>,
    k8s: Arc<RwLock<Vec<DiscoveredService>>>,
}

impl Default for DiscoveryCache {
    fn default() -> Self {
        Self::new()
    }
}

impl DiscoveryCache {
    /// Create a new empty discovery cache.
    pub fn new() -> Self {
        Self {
            docker: Arc::new(RwLock::new(Vec::new())),
            k8s: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Get all discovered Docker services (clone).
    pub fn get_docker(&self) -> Vec<DiscoveredService> {
        self.docker.read().unwrap().clone()
    }

    /// Get all discovered K8s services (clone).
    pub fn get_k8s(&self) -> Vec<DiscoveredService> {
        self.k8s.read().unwrap().clone()
    }

    /// Replace all Docker discovered services.
    pub fn set_docker(&self, services: Vec<DiscoveredService>) {
        *self.docker.write().unwrap() = services;
    }

    /// Replace all K8s discovered services.
    pub fn set_k8s(&self, services: Vec<DiscoveredService>) {
        *self.k8s.write().unwrap() = services;
    }

    /// Add a single Docker discovered service.
    pub fn add_docker(&self, service: DiscoveredService) {
        self.docker.write().unwrap().push(service);
    }

    /// Add a single K8s discovered service.
    pub fn add_k8s(&self, service: DiscoveredService) {
        self.k8s.write().unwrap().push(service);
    }

    /// Remove Docker discovered services by source_id.
    pub fn remove_docker(&self, source_id: &str) {
        self.docker
            .write()
            .unwrap()
            .retain(|s| s.source_id != source_id);
    }

    /// Remove K8s discovered services by source_id.
    pub fn remove_k8s(&self, source_id: &str) {
        self.k8s
            .write()
            .unwrap()
            .retain(|s| s.source_id != source_id);
    }
}

// --- Server functions ---

use leptos::server_fn::ServerFnError;

use crate::error::AppError;

/// Extract the SqlitePool from Axum Extension.
#[cfg(feature = "ssr")]
async fn get_pool() -> Result<sqlx::SqlitePool, AppError> {
    use axum::Extension;
    Ok(leptos_axum::extract::<Extension<sqlx::SqlitePool>>()
        .await
        .map_err(|_| AppError::Internal)?
        .0)
}

/// Extract the DiscoveryCache from Axum Extension.
#[cfg(feature = "ssr")]
async fn get_cache() -> Result<DiscoveryCache, AppError> {
    use axum::Extension;
    Ok(leptos_axum::extract::<Extension<DiscoveryCache>>()
        .await
        .map_err(|_| AppError::Internal)?
        .0)
}

/// Require admin session for discovery reads.
#[cfg(feature = "ssr")]
async fn require_admin(pool: &sqlx::SqlitePool) -> Result<(), AppError> {
    use crate::domain::Role;
    let info = crate::server::auth_helper::require_session(pool).await?;
    if info.role != Role::Admin {
        return Err(AppError::Forbidden);
    }
    Ok(())
}

/// Discover Docker services. Admin-gated. Returns empty vec when disabled (no calls).
#[leptos::server]
pub async fn discover_docker() -> Result<Vec<DiscoveredService>, ServerFnError<AppError>> {
    #[cfg(feature = "ssr")]
    {
        let pool = get_pool().await?;
        require_admin(&pool).await?;

        // Check if Docker discovery is enabled.
        let integrations = crate::server::settings_queries::get_integrations_typed(&pool).await?;
        if !integrations.docker_enabled {
            return Ok(Vec::new()); // disabled = no calls
        }

        // Read from cache (populated by background task).
        let cache = get_cache().await?;
        Ok(cache.get_docker())
    }
    #[cfg(not(feature = "ssr"))]
    {
        Err(ServerFnError::from(AppError::Internal))
    }
}

/// Discover Kubernetes services. Admin-gated. Returns empty vec when disabled (no calls).
#[leptos::server]
pub async fn discover_kubernetes() -> Result<Vec<DiscoveredService>, ServerFnError<AppError>> {
    #[cfg(feature = "ssr")]
    {
        let pool = get_pool().await?;
        require_admin(&pool).await?;

        // Check if K8s discovery is enabled.
        let integrations = crate::server::settings_queries::get_integrations_typed(&pool).await?;
        if !integrations.k8s_enabled {
            return Ok(Vec::new()); // disabled = no calls
        }

        // Read from cache (populated by background task).
        let cache = get_cache().await?;
        Ok(cache.get_k8s())
    }
    #[cfg(not(feature = "ssr"))]
    {
        Err(ServerFnError::from(AppError::Internal))
    }
}
