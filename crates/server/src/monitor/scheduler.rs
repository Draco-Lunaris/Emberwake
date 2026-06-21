//! Scheduler: background task that runs health checks on configured intervals.
//! Reads services with monitor_enabled=true from DB, runs checks concurrently,
//! writes StatusReading + StatusHistory, emits SSE events on state change.
//! Disabled services make NO outbound calls.

use std::sync::Arc;
use std::time::Duration;

use sqlx::SqlitePool;
use tokio::time::interval;

use crate::sse::SseHub;

/// Default check interval if a service has no monitor_interval_s set (seconds).
/// Scheduler tick: how often to scan for monitored services (seconds).
const SCHEDULER_TICK_S: u64 = 30;

/// Spawn the monitor scheduler as a background task.
/// This runs for the lifetime of the server.
pub fn spawn_scheduler(pool: SqlitePool, sse_hub: Arc<SseHub>) {
    tokio::spawn(async move {
        let mut tick = interval(Duration::from_secs(SCHEDULER_TICK_S));

        loop {
            tick.tick().await;

            let services = match app::server::monitor_queries::list_monitored_services(&pool).await
            {
                Ok(s) => s,
                Err(e) => {
                    tracing::warn!("monitor scheduler: failed to list services: {e}");
                    continue;
                }
            };

            if services.is_empty() {
                continue;
            }

            // Spawn concurrent checks for each monitored service.
            for svc in services {
                let pool = pool.clone();
                let hub = sse_hub.clone();
                tokio::spawn(async move {
                    crate::monitor::check_service(&pool, &hub, &svc).await;
                });
            }
        }
    });
}
