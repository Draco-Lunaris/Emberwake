//! Weather widget component: shows current conditions (temp, condition, is-day).
//! Uses EventSource on the client to listen to /events for real-time weather updates.
//! Shows nothing/inert when no weather data available (no error display to user).

use leptos::prelude::*;

use crate::domain::WeatherReading;

/// Weather widget: renders current conditions from cached reading.
/// On the client (hydrate), an EventSource connects to /events and updates
/// the weather display in real-time without page refresh.
/// Shows nothing when no weather data is available.
#[component]
pub fn WeatherWidget(initial: Option<WeatherReading>) -> impl IntoView {
    let temp_signal = RwSignal::new(initial.as_ref().and_then(|w| w.temp));
    let condition_signal = RwSignal::new(initial.as_ref().and_then(|w| w.condition.clone()));
    let is_day_signal = RwSignal::new(initial.as_ref().and_then(|w| w.is_day));

    // Client-side: connect EventSource and listen for weather events.
    #[cfg(feature = "hydrate")]
    {
        use wasm_bindgen::JsCast;
        use wasm_bindgen::prelude::*;

        Effect::new(move || {
            let on_message = Closure::wrap(Box::new(move |event: web_sys::MessageEvent| {
                if let Some(data_str) = event.data().as_string()
                    && let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&data_str)
                {
                    if let Some(temp) = parsed.get("tempC").and_then(|v| v.as_f64()) {
                        temp_signal.set(Some(temp));
                    } else if parsed.get("tempC").is_none() {
                        temp_signal.set(None);
                    }

                    if let Some(cond) = parsed.get("conditionCode").and_then(|v| v.as_str()) {
                        condition_signal.set(Some(cond.to_string()));
                    }

                    if let Some(is_day) = parsed.get("isDay").and_then(|v| v.as_bool()) {
                        is_day_signal.set(Some(is_day));
                    }
                }
            }) as Box<dyn FnMut(web_sys::MessageEvent)>);

            if web_sys::window().is_some()
                && let Ok(es) = web_sys::EventSource::new("/events")
            {
                let es_clone = es.clone();
                let on_open = Closure::wrap(Box::new(move |_| {
                    // SSE connected (weather)
                }) as Box<dyn FnMut(JsValue)>);
                es.set_onopen(Some(on_open.as_ref().unchecked_ref()));
                on_open.forget();

                es.add_event_listener_with_callback("weather", on_message.as_ref().unchecked_ref())
                    .ok();
                on_message.forget();

                // Keep the EventSource alive — store in a leak to prevent drop.
                std::mem::forget(es_clone);
            }
        });
    }

    // Only render when we have some weather data.
    move || {
        let temp = temp_signal.get();
        let condition = condition_signal.get();

        if temp.is_none() && condition.is_none() {
            return ().into_any(); // inert — show nothing
        }

        view! {
            <div class="weather-widget">
                {move || {
                    if let Some(t) = temp_signal.get() {
                        view! { <span class="weather-temp">{format!("{t:.0}°C")}</span> }.into_any()
                    } else {
                        ().into_any()
                    }
                }}
                {move || {
                    if let Some(c) = condition_signal.get() {
                        view! { <span class="weather-condition">{c}</span> }.into_any()
                    } else {
                        ().into_any()
                    }
                }}
                {move || {
                    if let Some(day) = is_day_signal.get() {
                        let icon = if day { "☀" } else { "☾" };
                        view! { <span class="weather-day-indicator">{icon}</span> }.into_any()
                    } else {
                        ().into_any()
                    }
                }}
            </div>
        }
        .into_any()
    }
}
