//! HTML import parser — parses Netscape bookmark format (browser export).
//! Uses the `scraper` crate for HTML DOM parsing.
//! Extracts folders (categories) and links (bookmarks) with nesting depth limits.
//! Never panics on malformed input — all errors are returned as `AppError`.

#![cfg(feature = "ssr")]

use scraper::{ElementRef, Html, Selector};

use crate::domain::{ParsedBookmark, ParsedCategory, ParsedData, Visibility};
use crate::error::AppError;
use crate::server::importer::MAX_DERIVATION_DEPTH;

/// Parse a Netscape-format HTML bookmark file into ParsedData.
pub fn parse_html(data: &[u8]) -> Result<ParsedData, AppError> {
    let html_str = std::str::from_utf8(data)
        .map_err(|_| AppError::Validation("input is not valid UTF-8".into()))?;

    let document = Html::parse_document(html_str);

    let dl_selector = Selector::parse("dl").map_err(|_| AppError::Internal)?;

    // Check derivation depth: count nested dl elements
    let dl_count = document.select(&dl_selector).count();
    if dl_count > MAX_DERIVATION_DEPTH {
        return Err(AppError::Validation(
            "HTML bookmark nesting exceeds maximum derivation depth".into(),
        ));
    }

    let mut categories: Vec<ParsedCategory> = Vec::new();
    let mut bookmarks: Vec<ParsedBookmark> = Vec::new();
    let mut current_category: Option<String> = None;

    // Walk the DOM tree in document order to map bookmarks to their categories.
    // When an <h3> is encountered, it starts a new category. Subsequent <a> elements
    // belong to that category until the next <h3> is found.
    walk_element(
        document.root_element(),
        &mut current_category,
        &mut categories,
        &mut bookmarks,
    );

    Ok(ParsedData {
        categories,
        bookmarks,
        services: Vec::new(),
        themes: Vec::new(),
        settings: None,
    })
}

/// Recursively walk the DOM tree in document order.
/// Updates `current_category` when an `<h3>` is found, and assigns bookmarks
/// to `current_category` when an `<a href>` is found.
fn walk_element(
    element: ElementRef,
    current_category: &mut Option<String>,
    categories: &mut Vec<ParsedCategory>,
    bookmarks: &mut Vec<ParsedBookmark>,
) {
    for child_node in element.children() {
        let Some(child_el) = ElementRef::wrap(child_node) else {
            continue;
        };

        match child_el.value().name() {
            "h3" => {
                let name = child_el.text().collect::<String>().trim().to_string();
                if !name.is_empty() {
                    categories.push(ParsedCategory {
                        name: name.clone(),
                        icon: None,
                        visibility: Visibility::Public,
                    });
                    *current_category = Some(name);
                }
            }
            "a" => {
                let href = child_el.value().attr("href").unwrap_or("").to_string();
                let name = child_el.text().collect::<String>().trim().to_string();
                if !name.is_empty() && !href.is_empty() {
                    bookmarks.push(ParsedBookmark {
                        name,
                        url: href,
                        icon: None,
                        category_name: current_category.clone(),
                        visibility: Visibility::Public,
                    });
                }
            }
            _ => {
                walk_element(child_el, current_category, categories, bookmarks);
            }
        }
    }
}
