//! Server library crate — exposes modules for integration tests.
//! The binary entry point is in `main.rs`.

pub mod audit;
pub mod auth;
pub mod config;
pub mod db;
pub mod integrations;
pub mod monitor;
pub mod public_api;
pub mod security;
pub mod sse;
pub mod state;
pub mod telemetry;
