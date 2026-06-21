//! Security middleware: CSP nonce, HSTS, nosniff, frame-deny, referrer policy.
//! Rate limiting lives in `rate_limit.rs`.

pub mod headers;
pub mod rate_limit;
