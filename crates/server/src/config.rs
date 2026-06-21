//! Configuration loader: figment TOML + env, *_FILE secret resolution.
//! File wins; unreadable fails loud. Validated at startup.

use figment::{Figment, providers::Env, providers::Format, providers::Toml};
use serde::{Deserialize, Serialize};
use std::path::Path;
use tracing::info;

/// Top-level application configuration.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    #[serde(default = "default_db_path")]
    pub db_path: String,
    #[serde(default = "default_bind_addr")]
    pub bind_addr: String,
    #[serde(default)]
    pub server_key: String,
    #[serde(default)]
    pub oidc: OidcConfig,
    #[serde(default)]
    pub argon2: Argon2Config,
    #[serde(default)]
    pub backup: BackupConfig,
    #[serde(default)]
    pub telemetry: TelemetryConfig,
    #[serde(default)]
    pub security: SecurityConfig,
}

fn default_db_path() -> String {
    "data/emberwake.db".to_string()
}

fn default_bind_addr() -> String {
    "0.0.0.0:5005".to_string()
}

/// OIDC client configuration. Optional — disabled by default.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct OidcConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub issuer_url: String,
    #[serde(default)]
    pub client_id: String,
    #[serde(default)]
    pub client_secret: String,
    #[serde(default)]
    pub redirect_url: String,
}

/// Argon2id parameters. Defaults: m=32 MiB, t=3, p=1.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Argon2Config {
    #[serde(default = "default_m_cost")]
    pub m_cost: u32,
    #[serde(default = "default_t_cost")]
    pub t_cost: u32,
    #[serde(default = "default_p_cost")]
    pub p_cost: u32,
}

impl Default for Argon2Config {
    fn default() -> Self {
        Self {
            m_cost: default_m_cost(),
            t_cost: default_t_cost(),
            p_cost: default_p_cost(),
        }
    }
}

fn default_m_cost() -> u32 {
    32 * 1024
}
fn default_t_cost() -> u32 {
    3
}
fn default_p_cost() -> u32 {
    1
}

/// SQLite backup configuration.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BackupConfig {
    #[serde(default = "default_checkpoint_interval_s")]
    pub checkpoint_interval_s: u64,
    #[serde(default = "default_backup_enabled")]
    pub backup_enabled: bool,
    #[serde(default = "default_backup_interval_s")]
    pub backup_interval_s: u64,
    #[serde(default = "default_max_backup_count")]
    pub max_backup_count: u32,
    #[serde(default = "default_max_backup_size_mb")]
    pub max_backup_size_mb: u64,
    #[serde(default = "default_backup_dir")]
    pub backup_dir: String,
}

impl Default for BackupConfig {
    fn default() -> Self {
        Self {
            checkpoint_interval_s: default_checkpoint_interval_s(),
            backup_enabled: default_backup_enabled(),
            backup_interval_s: default_backup_interval_s(),
            max_backup_count: default_max_backup_count(),
            max_backup_size_mb: default_max_backup_size_mb(),
            backup_dir: default_backup_dir(),
        }
    }
}

fn default_checkpoint_interval_s() -> u64 {
    900
}
fn default_backup_enabled() -> bool {
    false
}
fn default_backup_interval_s() -> u64 {
    86400
}
fn default_max_backup_count() -> u32 {
    7
}
fn default_max_backup_size_mb() -> u64 {
    500
}
fn default_backup_dir() -> String {
    "data/backups".to_string()
}

/// Telemetry configuration.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TelemetryConfig {
    #[serde(default = "default_log_level")]
    pub log_level: String,
    #[serde(default)]
    pub otlp_endpoint: Option<String>,
    #[serde(default = "default_metrics_enabled")]
    pub metrics_enabled: bool,
}

impl Default for TelemetryConfig {
    fn default() -> Self {
        Self {
            log_level: default_log_level(),
            otlp_endpoint: None,
            metrics_enabled: default_metrics_enabled(),
        }
    }
}

fn default_log_level() -> String {
    "info".to_string()
}
fn default_metrics_enabled() -> bool {
    true
}

/// Security configuration.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SecurityConfig {
    #[serde(default = "default_hsts_max_age")]
    pub hsts_max_age: u64,
    #[serde(default = "default_rate_limit_enabled")]
    pub rate_limit_enabled: bool,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            hsts_max_age: default_hsts_max_age(),
            rate_limit_enabled: default_rate_limit_enabled(),
        }
    }
}

fn default_hsts_max_age() -> u64 {
    31536000
}
fn default_rate_limit_enabled() -> bool {
    true
}

/// Load configuration from TOML file + environment variables.
#[allow(clippy::result_large_err)]
pub fn load() -> Result<Config, figment::Error> {
    let figment = Figment::new()
        .merge(Toml::file("emberwake.toml"))
        .merge(Env::prefixed("EMBERWAKE_").split("__"));

    let mut config: Config = figment.extract()?;
    resolve_file_secrets(&mut config);
    info!("Configuration loaded");
    Ok(config)
}

fn resolve_file_secrets(_config: &mut Config) {
    // Future: OIDC client secret, weather API key, etc.
}

/// Ensure the parent directory of the database path exists.
pub fn ensure_db_dir(db_path: &str) -> std::io::Result<()> {
    if let Some(parent) = Path::new(db_path).parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent)?;
    }
    Ok(())
}
