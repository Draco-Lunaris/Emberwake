//! Import parsers for JSON, HTML (Netscape bookmarks), and OPML.
//! All parsers are sync functions that enforce size and derivation depth limits.
//! They never panic on malformed input — all errors are returned as `AppError`.
//! The server functions wrap these in `spawn_blocking` to avoid blocking the executor.

#![cfg(feature = "ssr")]

pub mod html;
pub mod json;
pub mod opml;

use crate::domain::ParsedData;
use crate::error::AppError;

/// Default maximum import file size: 10 MB.
pub const MAX_IMPORT_SIZE: usize = 10 * 1024 * 1024;

/// Default maximum derivation/nesting depth: 100 levels.
pub const MAX_DERIVATION_DEPTH: usize = 100;

/// Check that input data is within the size limit.
pub fn check_size(data: &[u8]) -> Result<(), AppError> {
    if data.len() > MAX_IMPORT_SIZE {
        return Err(AppError::Validation(format!(
            "import file exceeds maximum size ({} bytes)",
            MAX_IMPORT_SIZE
        )));
    }
    Ok(())
}

/// Dispatch parsing based on import kind.
pub fn parse(data: &[u8], kind: crate::domain::ImportKind) -> Result<ParsedData, AppError> {
    check_size(data)?;
    match kind {
        crate::domain::ImportKind::Json => json::parse_json(data),
        crate::domain::ImportKind::HtmlBookmarks => html::parse_html(data),
        crate::domain::ImportKind::Opml => opml::parse_opml(data),
    }
}
