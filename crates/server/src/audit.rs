//! Append-only audit writer for security-relevant events.
//! No update/delete paths are exposed — enforced by convention and access layer.

use chrono::Utc;
use sqlx::SqlitePool;
use tracing::warn;
use uuid::Uuid;

/// Audit event action types.
#[derive(Debug, Clone)]
pub struct AuditEvent {
    pub actor_id: Option<Uuid>,
    pub action: String,
    pub target: Option<String>,
    pub ip: Option<String>,
    pub user_agent: Option<String>,
    pub result: AuditResult,
}

#[derive(Debug, Clone, Copy)]
pub enum AuditResult {
    Success,
    Failure,
}

impl AuditResult {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Success => "success",
            Self::Failure => "failure",
        }
    }
}

/// Append-only audit writer. Writes are best-effort.
pub struct AuditWriter {
    pool: SqlitePool,
}

impl AuditWriter {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Write an audit event. Best-effort: logs warning on failure.
    pub async fn write(&self, event: AuditEvent) {
        let id = Uuid::now_v7().to_string();
        let ts = Utc::now().to_rfc3339();
        let actor_id = event.actor_id.map(|u| u.to_string());
        let result = event.result.as_str();

        let res = sqlx::query(
            "INSERT INTO audit_event (id, ts, actor_id, action, target, ip, user_agent, result) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&id)
        .bind(&ts)
        .bind(&actor_id)
        .bind(&event.action)
        .bind(&event.target)
        .bind(&event.ip)
        .bind(&event.user_agent)
        .bind(result)
        .execute(&self.pool)
        .await;

        if let Err(e) = res {
            warn!("Failed to write audit event: {e}");
        }
    }
}
