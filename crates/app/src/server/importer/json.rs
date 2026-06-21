//! JSON import parser — parses ExportDocument JSON into ParsedData.
//! Enforces size and derivation depth limits. Never panics on malformed input.

#![cfg(feature = "ssr")]

use crate::domain::{ParsedBookmark, ParsedCategory, ParsedData, ParsedService, ParsedTheme};
use crate::error::AppError;
use crate::server::importer::MAX_DERIVATION_DEPTH;

/// Parse a JSON export document into ParsedData.
pub fn parse_json(data: &[u8]) -> Result<ParsedData, AppError> {
    // Check derivation depth: count consecutive opening braces/brackets as a depth proxy.
    check_json_depth(data)?;

    let doc: crate::domain::ExportDocument = serde_json::from_slice(data)
        .map_err(|e| AppError::Validation(format!("invalid JSON: {e}")))?;

    if doc.version != "1.0" {
        return Err(AppError::Validation(format!(
            "unsupported export version: {} (expected 1.0)",
            doc.version
        )));
    }

    let categories = doc
        .categories
        .into_iter()
        .map(|c| ParsedCategory {
            name: c.name,
            icon: c.icon,
            visibility: c.visibility,
        })
        .collect();

    let bookmarks = doc
        .bookmarks
        .into_iter()
        .map(|b| ParsedBookmark {
            name: b.name,
            url: b.url,
            icon: b.icon,
            category_name: b.category_name,
            visibility: b.visibility,
        })
        .collect();

    let services = doc
        .services
        .into_iter()
        .map(|s| ParsedService {
            name: s.name,
            url: s.url,
            icon: s.icon,
            description: s.description,
            category_name: s.category_name,
            is_pinned: s.is_pinned,
            visibility: s.visibility,
            monitor_enabled: s.monitor_enabled,
            monitor_kind: s.monitor_kind,
            monitor_target: s.monitor_target,
            monitor_interval_s: s.monitor_interval_s,
        })
        .collect();

    let themes = doc
        .themes
        .into_iter()
        .map(|t| ParsedTheme {
            name: t.name,
            tokens: t.tokens,
            custom_css: t.custom_css,
        })
        .collect();

    Ok(ParsedData {
        categories,
        bookmarks,
        services,
        themes,
        settings: doc.settings,
    })
}

/// Check JSON nesting depth to prevent deeply nested structures (zip-bomb defense).
fn check_json_depth(data: &[u8]) -> Result<(), AppError> {
    let mut max_consecutive = 0usize;
    let mut current = 0usize;
    for &byte in data {
        if byte == b'{' || byte == b'[' {
            current += 1;
            if current > max_consecutive {
                max_consecutive = current;
            }
        } else if byte == b'}' || byte == b']' {
            current = current.saturating_sub(1);
        }
    }

    if max_consecutive > MAX_DERIVATION_DEPTH {
        return Err(AppError::Validation(
            "JSON nesting exceeds maximum derivation depth".into(),
        ));
    }

    Ok(())
}
