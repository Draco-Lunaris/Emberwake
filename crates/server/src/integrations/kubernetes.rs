//! Kubernetes integration: list + watch Ingress resources (read-only).
//! Uses kube-rs for Kubernetes API access. Only list/watch calls —
//! mutation is impossible by omission (Principle II).
//! Config-gated: inert when disabled (no connection, no calls).
//! Handles K8s API unavailable gracefully: reports unavailable, does not crash.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use kube::Api;
use kube::ResourceExt;
use kube::api::ListParams;
use kube::runtime::watcher;
use kube::runtime::watcher::Config;
use tokio::time::interval;

use app::domain::{DiscoveryAction, DiscoverySource, SseDiscoveryEvent};
use app::server::discovery::DiscoveryCache;

use crate::integrations::labels;
use crate::sse::SseHub;

/// Scheduler tick: how often to check if K8s discovery is enabled (seconds).
const SCHEDULER_TICK_S: u64 = 60;

/// List all Ingress resources with annotations, parse into DiscoveredService entries.
/// Read-only: only calls list, never create/update/delete.
async fn list_ingresses(client: &kube::Client) -> Vec<app::domain::DiscoveredService> {
    let api: Api<k8s_openapi::api::networking::v1::Ingress> = Api::all(client.clone());
    match api.list(&ListParams::default()).await {
        Ok(list) => list
            .items
            .iter()
            .filter_map(|ingress| {
                let annotations = ingress.metadata.annotations.as_ref()?;
                let name = ingress.name_any();
                let ns = ingress.metadata.namespace.as_deref().unwrap_or("");
                let source_id = format!("{ns}/{name}");
                let map: HashMap<String, String> = annotations
                    .iter()
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect();
                Some(labels::parse_labels(
                    &map,
                    DiscoverySource::Kubernetes,
                    &source_id,
                ))
            })
            .flatten()
            .collect(),
        Err(e) => {
            tracing::warn!("k8s list_ingresses error: {e}");
            Vec::new()
        }
    }
}

/// Watch Ingress changes. On add/update/delete, update cache and emit SSE.
/// Read-only: only consumes the watch stream, never calls mutating endpoints.
async fn watch_ingresses(client: kube::Client, cache: &DiscoveryCache, sse_hub: &SseHub) {
    use futures_util::StreamExt;
    let api: Api<k8s_openapi::api::networking::v1::Ingress> = Api::all(client);
    let stream = watcher(api, Config::default());
    let mut pinned = std::pin::pin!(stream);
    while let Some(event) = pinned.next().await {
        match event {
            Ok(watcher::Event::Apply(ingress)) => {
                if let Some(annotations) = &ingress.metadata.annotations {
                    let name = ingress.name_any();
                    let ns = ingress.metadata.namespace.as_deref().unwrap_or("");
                    let source_id = format!("{ns}/{name}");
                    let map: HashMap<String, String> = annotations
                        .iter()
                        .map(|(k, v)| (k.clone(), v.clone()))
                        .collect();
                    let services =
                        labels::parse_labels(&map, DiscoverySource::Kubernetes, &source_id);
                    for svc in services {
                        cache.add_k8s(svc.clone());
                        sse_hub.broadcast_discovery(SseDiscoveryEvent {
                            service_id: svc.source_id,
                            action: DiscoveryAction::Added,
                            name: svc.name,
                            url: svc.url,
                        });
                    }
                }
            }
            Ok(watcher::Event::Delete(ingress)) => {
                let name = ingress.name_any();
                let ns = ingress.metadata.namespace.as_deref().unwrap_or("");
                let source_id = format!("{ns}/{name}");
                cache.remove_k8s(&source_id);
                sse_hub.broadcast_discovery(SseDiscoveryEvent {
                    service_id: source_id,
                    action: DiscoveryAction::Removed,
                    name: String::new(),
                    url: String::new(),
                });
            }
            Err(e) => {
                tracing::warn!("k8s watch stream error: {e}");
                break;
            }
            _ => {}
        }
    }
}

/// Spawn the K8s discovery scheduler as a background task.
/// Checks if K8s discovery is enabled on each tick; if so, connects,
/// lists ingresses, populates cache, and watches for live deltas.
/// When disabled: no connection to K8s API, no calls.
pub fn spawn_scheduler(pool: sqlx::SqlitePool, sse_hub: Arc<SseHub>, cache: DiscoveryCache) {
    tokio::spawn(async move {
        let mut tick = interval(Duration::from_secs(SCHEDULER_TICK_S));
        loop {
            tick.tick().await;

            let integrations =
                match app::server::settings_queries::get_integrations_typed(&pool).await {
                    Ok(i) => i,
                    Err(e) => {
                        tracing::warn!("k8s scheduler: failed to read settings: {e}");
                        continue;
                    }
                };

            if !integrations.k8s_enabled {
                continue; // disabled — no calls
            }

            // Try to connect to K8s API (in-cluster or kubeconfig).
            let client = match kube::Client::try_default().await {
                Ok(c) => c,
                Err(e) => {
                    tracing::debug!("k8s scheduler: API unavailable: {e}");
                    continue;
                }
            };

            // Initial population: list all ingresses.
            let services = list_ingresses(&client).await;
            cache.set_k8s(services);

            // Watch for changes (blocks until error/disconnect, then retry on next tick).
            watch_ingresses(client, &cache, &sse_hub).await;
        }
    });
}
