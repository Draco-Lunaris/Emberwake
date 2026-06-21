//! External integrations module.
//! Houses the weather API client + scheduler (US7),
//! Docker discovery (US8), Kubernetes discovery (US8),
//! and the shared label/annotation parser.

pub mod docker;
pub mod kubernetes;
pub mod labels;
pub mod weather;
