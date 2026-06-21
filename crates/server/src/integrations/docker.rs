//! Docker integration: list containers + watch events (read-only).
//! Uses bollard for Docker API access. Only list/inspect/events calls —
//! mutation is impossible by omission (Principle II).
//! Config-gated: inert when disabled (no connection, no calls).
//! Handles Docker socket absent gracefully: reports unavailable, does not crash.

use std::sync::Arc;
use std::time::Duration;

use bollard::Docker;
use bollard::container::ListContainersOptions;
use futures_util::StreamExt;
use tokio::time::interval;

use app::domain::{DiscoveryAction, DiscoverySource, SseDiscoveryEvent};
use app::server::discovery::DiscoveryCache;

use crate::integrations::labels;
use crate::sse::SseHub;

/// Default Docker socket path.
const DEFAULT_SOCKET: &str = "/var/run/docker.sock";

/// Scheduler tick: how often to check if Docker discovery is enabled (seconds).
const SCHEDULER_TICK_S: u64 = 60;

/// Connect to Docker via Unix socket. Returns None if socket is absent.
fn connect(socket_path: Option<&str>) -> Option<Docker> {
    let path = socket_path.unwrap_or(DEFAULT_SOCKET);
    if !std::path::Path::new(path).exists() {
        return None;
    }
    // Use defaults which reads DOCKER_HOST or uses /var/run/docker.sock
    Docker::connect_with_unix_defaults().ok()
}

/// List all containers with labels, parse into DiscoveredService entries.
/// Read-only: only calls list_containers, never create/delete/start/stop.
async fn list_containers(docker: &Docker) -> Vec<app::domain::DiscoveredService> {
    let options = ListContainersOptions::<String> {
        all: true,
        ..Default::default()
    };
    match docker.list_containers(Some(options)).await {
        Ok(containers) => containers
            .iter()
            .filter_map(|c| {
                let labels = c.labels.as_ref()?;
                let id = c.id.as_deref().unwrap_or("");
                Some(labels::parse_labels(labels, DiscoverySource::Docker, id))
            })
            .flatten()
            .collect(),
        Err(e) => {
            tracing::warn!("docker list_containers error: {e}");
            Vec::new()
        }
    }
}

/// Watch Docker events stream. On container start/stop, update cache and emit SSE.
/// Read-only: only consumes the events stream, never calls mutating endpoints.
async fn watch_events(docker: Docker, cache: &DiscoveryCache, sse_hub: &SseHub) {
    let mut stream = docker.events(None::<bollard::system::EventsOptions<String>>);
    while let Some(result) = stream.next().await {
        match result {
            Ok(event) => {
                // Process all events; container events have action="start"/"die"/"stop".
                let action = event.action.as_deref();
                let container_id = event
                    .actor
                    .as_ref()
                    .and_then(|a| a.id.as_deref())
                    .unwrap_or("");

                match action {
                    Some("start") => {
                        // Inspect container for labels (read-only call).
                        if let Ok(container) = docker.inspect_container(container_id, None).await {
                            let labels = container.config.as_ref().and_then(|c| c.labels.as_ref());
                            if let Some(labels) = labels {
                                let services = labels::parse_labels(
                                    labels,
                                    DiscoverySource::Docker,
                                    container_id,
                                );
                                for svc in services {
                                    cache.add_docker(svc.clone());
                                    sse_hub.broadcast_discovery(SseDiscoveryEvent {
                                        service_id: svc.source_id,
                                        action: DiscoveryAction::Added,
                                        name: svc.name,
                                        url: svc.url,
                                    });
                                }
                            }
                        }
                    }
                    Some("die") | Some("stop") => {
                        cache.remove_docker(container_id);
                        sse_hub.broadcast_discovery(SseDiscoveryEvent {
                            service_id: container_id.to_string(),
                            action: DiscoveryAction::Removed,
                            name: String::new(),
                            url: String::new(),
                        });
                    }
                    _ => {}
                }
            }
            Err(e) => {
                tracing::warn!("docker events stream error: {e}");
                break;
            }
        }
    }
}

/// Spawn the Docker discovery scheduler as a background task.
/// Checks if Docker discovery is enabled on each tick; if so, connects,
/// lists containers, populates cache, and watches events for live deltas.
/// When disabled: no connection to Docker socket, no API calls.
pub fn spawn_scheduler(pool: sqlx::SqlitePool, sse_hub: Arc<SseHub>, cache: DiscoveryCache) {
    tokio::spawn(async move {
        let mut tick = interval(Duration::from_secs(SCHEDULER_TICK_S));
        loop {
            tick.tick().await;

            let integrations =
                match app::server::settings_queries::get_integrations_typed(&pool).await {
                    Ok(i) => i,
                    Err(e) => {
                        tracing::warn!("docker scheduler: failed to read settings: {e}");
                        continue;
                    }
                };

            if !integrations.docker_enabled {
                continue; // disabled — no calls
            }

            let docker = match connect(integrations.docker_socket.as_deref()) {
                Some(d) => d,
                None => {
                    tracing::debug!("docker scheduler: socket unavailable");
                    continue;
                }
            };

            // Initial population: list all containers.
            let services = list_containers(&docker).await;
            cache.set_docker(services);

            // Watch events (blocks until error/disconnect, then retry on next tick).
            watch_events(docker, &cache, &sse_hub).await;
        }
    });
}
