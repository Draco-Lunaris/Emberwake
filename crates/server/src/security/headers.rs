//! Security headers middleware — applied to ALL responses.
//! CSP with nonce is set via Leptos Meta tag in the App component (per-response nonce).
//! This module provides layers for the remaining security headers (HSTS, nosniff, frame-deny, referrer).

use axum::http::{HeaderName, HeaderValue};
use tower_http::set_header::SetResponseHeaderLayer;

/// Build a tower layer that sets HSTS header.
pub fn hsts_layer(max_age: u64) -> SetResponseHeaderLayer<HeaderValue> {
    SetResponseHeaderLayer::overriding(
        HeaderName::from_static("strict-transport-security"),
        HeaderValue::from_str(&format!("max-age={max_age}; includeSubDomains"))
            .expect("valid HSTS header"),
    )
}

/// Build a tower layer that sets X-Content-Type-Options: nosniff.
pub fn nosniff_layer() -> SetResponseHeaderLayer<HeaderValue> {
    SetResponseHeaderLayer::overriding(
        HeaderName::from_static("x-content-type-options"),
        HeaderValue::from_static("nosniff"),
    )
}

/// Build a tower layer that sets X-Frame-Options: DENY.
pub fn frame_deny_layer() -> SetResponseHeaderLayer<HeaderValue> {
    SetResponseHeaderLayer::overriding(
        HeaderName::from_static("x-frame-options"),
        HeaderValue::from_static("DENY"),
    )
}

/// Build a tower layer that sets Referrer-Policy: no-referrer.
pub fn referrer_policy_layer() -> SetResponseHeaderLayer<HeaderValue> {
    SetResponseHeaderLayer::overriding(
        HeaderName::from_static("referrer-policy"),
        HeaderValue::from_static("no-referrer"),
    )
}
