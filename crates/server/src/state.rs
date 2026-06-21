//! Shared application state — accessible to all Axum handlers.

use axum::extract::FromRef;
use leptos::prelude::LeptosOptions;
use sqlx::SqlitePool;
use std::sync::Arc;

use crate::audit::AuditWriter;
use crate::config::Config;
use crate::sse::SseHub;

/// Application state shared across all Axum handlers.
/// Contains LeptosOptions so the router state can provide it via FromRef.
#[derive(Clone)]
pub struct AppState {
    pub leptos_options: LeptosOptions,
    pub db: SqlitePool,
    pub config: Arc<Config>,
    pub audit: Arc<AuditWriter>,
    pub sse_hub: Arc<SseHub>,
}

/// Implement FromRef so leptos_axum can extract LeptosOptions from AppState.
impl FromRef<AppState> for LeptosOptions {
    fn from_ref(state: &AppState) -> Self {
        state.leptos_options.clone()
    }
}

/// Implement FromRef so server functions can extract the SqlitePool from AppState.
impl FromRef<AppState> for SqlitePool {
    fn from_ref(state: &AppState) -> Self {
        state.db.clone()
    }
}
