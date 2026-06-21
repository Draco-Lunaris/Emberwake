//! Server-side modules: query functions (ssr-only) and server function definitions.

#[cfg(feature = "ssr")]
pub mod auth_queries;

#[cfg(feature = "ssr")]
pub mod auth_helper;

pub mod auth;

#[cfg(feature = "ssr")]
pub mod content_queries;

#[cfg(feature = "ssr")]
pub mod content_write_queries;

pub mod content_read;

#[cfg(feature = "ssr")]
pub mod settings_read;

#[cfg(feature = "ssr")]
pub mod settings_queries;

#[cfg(feature = "ssr")]
pub mod extended_auth_queries;

pub mod content_write;

pub mod extended_auth;

pub mod settings;

#[cfg(feature = "ssr")]
pub mod monitor_queries;

pub mod monitor_read;

#[cfg(feature = "ssr")]
pub mod weather_queries;

pub mod weather_read;

pub mod discovery;

#[cfg(feature = "ssr")]
pub mod importer;

#[cfg(feature = "ssr")]
pub mod export_queries;

pub mod import_export;
