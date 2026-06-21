//! Rate limiting via tower_governor with per-route policies.
//! Login/token/import routes get stricter limits.

use governor::middleware::NoOpMiddleware;
use tower_governor::GovernorLayer;
use tower_governor::governor::GovernorConfigBuilder;
use tower_governor::key_extractor::PeerIpKeyExtractor;

type DefaultGovernorLayer = GovernorLayer<PeerIpKeyExtractor, NoOpMiddleware>;

/// Default rate limit: 60 requests per minute per IP (general).
pub fn default_governor() -> DefaultGovernorLayer {
    let config = GovernorConfigBuilder::default()
        .per_second(60)
        .burst_size(60)
        .finish()
        .expect("valid governor config");
    GovernorLayer {
        config: config.into(),
    }
}

/// Strict rate limit for login: 10 requests per minute per IP.
pub fn login_governor() -> DefaultGovernorLayer {
    let config = GovernorConfigBuilder::default()
        .per_second(10)
        .burst_size(10)
        .finish()
        .expect("valid governor config");
    GovernorLayer {
        config: config.into(),
    }
}

/// Strict rate limit for token operations: 20 requests per minute per IP.
pub fn token_governor() -> DefaultGovernorLayer {
    let config = GovernorConfigBuilder::default()
        .per_second(20)
        .burst_size(20)
        .finish()
        .expect("valid governor config");
    GovernorLayer {
        config: config.into(),
    }
}

/// Strict rate limit for import: 5 requests per minute per IP.
pub fn import_governor() -> DefaultGovernorLayer {
    let config = GovernorConfigBuilder::default()
        .per_second(5)
        .burst_size(5)
        .finish()
        .expect("valid governor config");
    GovernorLayer {
        config: config.into(),
    }
}
