//! SSE `/events` endpoint handler.
//! Public stream carries only public-service status and weather.
//! Authenticated session upgrades to include private-service status and discovery events.

use std::convert::Infallible;
use std::time::Duration;

use axum::extract::State;
use axum::http::HeaderMap;
use axum::response::sse::{Event, KeepAlive, Sse};
use futures_util::stream::Stream;
use tokio_stream::StreamExt;
use tokio_stream::wrappers::BroadcastStream;

use app::domain::{SseStatusEvent, Visibility};

use crate::sse::SseEvent;
use crate::state::AppState;

/// Caller session level for SSE filtering.
#[derive(Clone, Copy, PartialEq, Eq)]
enum SseAuthLevel {
    Anonymous,
    Authenticated,
    Admin,
}

/// Determine the caller's auth level by checking the session cookie.
async fn auth_level(state: &AppState, headers: &HeaderMap) -> SseAuthLevel {
    let cookie_header = headers.get("cookie").and_then(|v| v.to_str().ok());
    if let Some(cookie) = cookie_header
        && let Some(token) = app::server::auth_queries::parse_session_cookie(Some(cookie))
        && let Ok(Some(info)) = app::server::auth_queries::lookup_session(&state.db, &token).await
    {
        if info.role == app::domain::Role::Admin {
            return SseAuthLevel::Admin;
        }
        return SseAuthLevel::Authenticated;
    }
    SseAuthLevel::Anonymous
}

/// SSE `/events` handler.
/// Public stream: only public-service status + weather.
/// Session-upgraded: includes private-service status + discovery events.
pub async fn events_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let level = auth_level(&state, &headers).await;
    let rx = state.sse_hub.subscribe();

    let stream = BroadcastStream::new(rx).filter_map(move |item| {
        let lvl = level;
        item.ok().and_then(move |event| {
            match &event {
                SseEvent::Status(se) => {
                    // Anonymous: only public-service status.
                    // Authenticated: includes private-service status.
                    // Admin: includes restricted-service status.
                    let allowed = match se.visibility {
                        Visibility::Public => true,
                        Visibility::Private => lvl != SseAuthLevel::Anonymous,
                        Visibility::Restricted => lvl == SseAuthLevel::Admin,
                    };
                    if !allowed {
                        return None;
                    }
                    let json = serde_json::to_string(&SseStatusEvent {
                        service_id: se.service_id,
                        state: se.state,
                        latency_ms: se.latency_ms,
                        visibility: se.visibility,
                    })
                    .unwrap_or_default();
                    Some(Ok(Event::default().event("status").data(json)))
                }
                SseEvent::Weather(data) => {
                    let json = serde_json::to_string(data).unwrap_or_default();
                    Some(Ok(Event::default().event("weather").data(json)))
                }
                SseEvent::Discovery(de) => {
                    // Discovery events are admin-only.
                    if lvl != SseAuthLevel::Admin {
                        return None;
                    }
                    let json = serde_json::to_string(de).unwrap_or_default();
                    Some(Ok(Event::default().event("discovery").data(json)))
                }
            }
        })
    });

    Sse::new(stream).keep_alive(
        KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text("keepalive"),
    )
}

/// Build SSE routes for the Axum router.
pub fn sse_routes() -> axum::Router<AppState> {
    axum::Router::new().route("/events", axum::routing::get(events_handler))
}
