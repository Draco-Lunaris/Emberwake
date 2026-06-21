//! SSE hub: broadcast channel for status, weather, and discovery events.
//! Connected clients subscribe via the `/events` endpoint.

pub mod handler;

use std::sync::Arc;
use tokio::sync::broadcast;

use app::domain::{MonitorState, SseDiscoveryEvent, SseStatusEvent, Visibility};
use uuid::Uuid;

/// Events pushed to connected SSE clients.
#[derive(Debug, Clone)]
pub enum SseEvent {
    /// A service status change event.
    Status(SseStatusEvent),
    /// A weather reading update (used by US7).
    Weather(serde_json::Value),
    /// A discovery event — service added or removed (US8).
    Discovery(SseDiscoveryEvent),
}

/// SSE hub wrapping a broadcast channel.
/// Cloning is cheap (inner is Arc). Stored in AppState.
#[derive(Clone)]
pub struct SseHub {
    tx: broadcast::Sender<SseEvent>,
}

impl SseHub {
    /// Create a new SSE hub with the given channel capacity.
    pub fn new(capacity: usize) -> Arc<Self> {
        let (tx, _rx) = broadcast::channel(capacity);
        Arc::new(Self { tx })
    }

    /// Subscribe to the broadcast channel.
    pub fn subscribe(&self) -> broadcast::Receiver<SseEvent> {
        self.tx.subscribe()
    }

    /// Broadcast a status event to all connected clients.
    pub fn broadcast_status(
        &self,
        service_id: Uuid,
        state: MonitorState,
        latency_ms: Option<i64>,
        visibility: Visibility,
    ) {
        let event = SseEvent::Status(SseStatusEvent {
            service_id,
            state,
            latency_ms,
            visibility,
        });
        // Ignore send errors — no subscribers is fine.
        let _ = self.tx.send(event);
    }

    /// Broadcast a weather event to all connected clients.
    pub fn broadcast_weather(&self, data: serde_json::Value) {
        let _ = self.tx.send(SseEvent::Weather(data));
    }

    /// Broadcast a discovery event to all connected clients.
    pub fn broadcast_discovery(&self, event: SseDiscoveryEvent) {
        let _ = self.tx.send(SseEvent::Discovery(event));
    }
}
