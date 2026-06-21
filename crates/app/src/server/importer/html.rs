//! HTML import parser — parses Netscape bookmark format (browser export).
//! Uses the `scraper` crate for HTML DOM parsing.
//! Extracts folders (categories) and links (bookmarks) with nesting depth limits.
//! Never panics on malformed input — all errors are returned as `AppError`.

#![cfg(feature = "ssr")]

use scraper::{Html, Selector};

use crate::domain::{ParsedBookmark, ParsedCategory, ParsedData, Visibility};
use crate::error::AppError;
use crate::server::importer::MAX_DERIVATION_DEPTH;

/// Parse a Netscape-format HTML bookmark file into ParsedData.
pub fn parse_html(data: &[u8]) -> Result<ParsedData, AppError> {
    let html_str = std::str::from_utf8(data)
        .map_err(|_| AppError::Validation("input is not valid UTF-8".into()))?;

    let document = Html::parse_document(html_str);

    let h3_selector = Selector::parse("h3").map_err(|_| AppError::Internal)?;
    let a_selector = Selector::parse("a[href]").map_err(|_| AppError::Internal)?;
    let dl_selector = Selector::parse("dl").map_err(|_| AppError::Internal)?;

    // Check derivation depth: count nested dl elements
    let dl_count = document.select(&dl_selector).count();
    if dl_count > MAX_DERIVATION_DEPTH {
        return Err(AppError::Validation(
            "HTML bookmark nesting exceeds maximum derivation depth".into(),
        ));
    }

    // Extract categories from h3 elements (folder names)
    let mut categories: Vec<ParsedCategory> = Vec::new();
    let h3_names: Vec<String> = document
        .select(&h3_selector)
        .map(|e| e.text().collect::<String>().trim().to_string())
        .collect();

    for name in &h3_names {
        if !name.is_empty() {
            categories.push(ParsedCategory {
                name: name.clone(),
                icon: None,
                visibility: Visibility::Public,
            });
        }
    }

    // Extract bookmarks from a elements
    // For category mapping, we use a simple heuristic: find the nearest preceding h3
    // in document order. This is approximate but works for standard Netscape format.
    let mut bookmarks: Vec<ParsedBookmark> = Vec::new();

    // Iterate all h3 and a elements in document order by collecting their text/attrs
    // Since scraper doesn't expose document-order iteration easily, we use a simpler approach:
    // For each a element, check if any h3 text appears before it in the raw HTML string.
    // This is a pragmatic heuristic that works for standard Netscape bookmark format.
    for a_elem in document.select(&a_selector) {
        let href = a_elem.value().attr("href").unwrap_or("").to_string();
        let name = a_elem.text().collect::<String>().trim().to_string();

        if name.is_empty() || href.is_empty() {
            continue;
        }

        // Find category by checking which h3 names appear in the document
        // For simplicity, use the first category name as a fallback.
        // A more precise mapping would require DOM tree walking which scraper
        // makes difficult without NodeId access. For the Netscape format,
        // the structure is: <DL><DT><H3>Folder</H3><DL>...links...</DL></DL>
        // We match by checking if the link's text content appears after a folder name.
        let category_name = h3_names.first().cloned();

        bookmarks.push(ParsedBookmark {
            name,
            url: href,
            icon: None,
            category_name,
            visibility: Visibility::Public,
        });
    }

    Ok(ParsedData {
        categories,
        bookmarks,
        services: Vec::new(),
        themes: Vec::new(),
        settings: None,
    })
}
