//! Live status tile component: shows up/down/degraded indicator with latency.
//! Uses EventSource on the client to listen to /events for real-time updates.

use leptos::prelude::*;

use crate::domain::{MonitorState, Service};

/// Live status tile: renders the service tile with a status indicator.
/// On the client (hydrate), an EventSource connects to /events and updates
/// the status indicator in real-time without page refresh.
#[component]
pub fn StatusTile(service: Service) -> impl IntoView {
    let state_signal = RwSignal::new(MonitorState::Up);
    let latency_signal = RwSignal::new(None::<i64>);

    // Client-side: connect EventSource and listen for status events.
    #[cfg(feature = "hydrate")]
    {
        use wasm_bindgen::JsCast;
        use wasm_bindgen::prelude::*;

        let service_id = service.id;
        Effect::new(move || {
            let id = service_id.to_string();
            let on_message = Closure::wrap(Box::new(move |event: web_sys::MessageEvent| {
                if let Some(data_str) = event.data().as_string()
                    && let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&data_str)
                    && let Some(sid) = parsed.get("serviceId").and_then(|v| v.as_str())
                    && sid == id
                {
                    if let Some(state_str) = parsed.get("state").and_then(|v| v.as_str()) {
                        match state_str {
                            "up" => state_signal.set(MonitorState::Up),
                            "down" => state_signal.set(MonitorState::Down),
                            "degraded" => state_signal.set(MonitorState::Degraded),
                            _ => {}
                        }
                    }
                    if let Some(lat) = parsed.get("latencyMs").and_then(|v| v.as_i64()) {
                        latency_signal.set(Some(lat));
                    } else if parsed.get("latencyMs").is_some() {
                        latency_signal.set(None);
                    }
                }
            }) as Box<dyn FnMut(web_sys::MessageEvent)>);

            if web_sys::window().is_some()
                && let Ok(es) = web_sys::EventSource::new("/events")
            {
                let es_clone = es.clone();
                let on_open = Closure::wrap(Box::new(move |_| {
                    // SSE connected
                }) as Box<dyn FnMut(JsValue)>);
                es.set_onopen(Some(on_open.as_ref().unchecked_ref()));
                on_open.forget();

                es.add_event_listener_with_callback("status", on_message.as_ref().unchecked_ref())
                    .ok();
                on_message.forget();

                // Keep the EventSource alive — store in a leak to prevent drop.
                std::mem::forget(es_clone);
            }
        });
    }

    view! {
        <a class="tile status-tile" href=service.url.clone()>
            {if let Some(icon) = &service.icon {
                view! { <img class="tile-icon" src=icon.clone() alt=service.name.clone() /> }.into_any()
            } else {
                view! { <span class="tile-icon-placeholder">{service.name.chars().next().unwrap_or('E')}</span> }.into_any()
            }}
            <span class="tile-name">{service.name.clone()}</span>
            <span class="status-indicator" data-state=move || state_signal.get().to_string()>
                {move || match state_signal.get() {
                    MonitorState::Up => "●",
                    MonitorState::Down => "○",
                    MonitorState::Degraded => "◐",
                }}
            </span>
            {move || {
                if let Some(ms) = latency_signal.get() {
                    view! { <span class="status-latency">{format!("{ms}ms")}</span> }.into_any()
                } else {
                    ().into_any()
                }
            }}
        </a>
    }
}
