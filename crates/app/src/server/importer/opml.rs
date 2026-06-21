//! OPML import parser — parses OPML XML into bookmarks.
//! Uses `quick-xml` for streaming XML parsing.
//! Enforces size and derivation depth limits. Never panics on malformed input.

#![cfg(feature = "ssr")]

use quick_xml::Reader;
use quick_xml::events::Event;

use crate::domain::{ParsedBookmark, ParsedData, Visibility};
use crate::error::AppError;
use crate::server::importer::MAX_DERIVATION_DEPTH;

/// Collect all attributes from an XML element into a Vec of (name, value) pairs.
fn collect_attrs<'a>(
    attrs: quick_xml::events::attributes::Attributes<'a>,
) -> Vec<(String, String)> {
    attrs
        .flatten()
        .map(|a| {
            (
                String::from_utf8_lossy(a.key.as_ref()).to_string(),
                String::from_utf8_lossy(a.value.as_ref()).to_string(),
            )
        })
        .collect()
}

/// Look up an attribute value by key (case-insensitive).
fn find_attr(attrs: &[(String, String)], key: &str) -> Option<String> {
    attrs
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case(key))
        .map(|(_, v)| v.clone())
}

/// Parse an OPML XML file into ParsedData (bookmarks only).
pub fn parse_opml(data: &[u8]) -> Result<ParsedData, AppError> {
    let xml_str = std::str::from_utf8(data)
        .map_err(|_| AppError::Validation("input is not valid UTF-8".into()))?;

    let mut reader = Reader::from_str(xml_str);
    reader.config_mut().trim_text(true);

    let mut bookmarks: Vec<ParsedBookmark> = Vec::new();
    let mut depth: usize = 0;
    let mut category_stack: Vec<String> = Vec::new();
    let mut depth_is_category: Vec<bool> = Vec::new();
    let mut buf = Vec::new();

    loop {
        let event = reader
            .read_event_into(&mut buf)
            .map_err(|e| AppError::Validation(format!("XML parse error: {e}")))?;

        match event {
            Event::Start(ref e) if e.name().as_ref() == b"outline" => {
                depth += 1;
                if depth > MAX_DERIVATION_DEPTH {
                    return Err(AppError::Validation(
                        "OPML nesting exceeds maximum derivation depth".into(),
                    ));
                }

                let attrs = collect_attrs(e.attributes());
                let xml_url = find_attr(&attrs, "xmlUrl").or_else(|| find_attr(&attrs, "xmlurl"));

                if let Some(url) = xml_url {
                    if !url.is_empty() {
                        let name = find_attr(&attrs, "title")
                            .or_else(|| find_attr(&attrs, "text"))
                            .unwrap_or_else(|| url.clone());

                        bookmarks.push(ParsedBookmark {
                            name,
                            url,
                            icon: None,
                            category_name: category_stack.last().cloned(),
                            visibility: Visibility::Public,
                        });
                    }
                    depth_is_category.push(false);
                } else {
                    let title = find_attr(&attrs, "title").or_else(|| find_attr(&attrs, "text"));

                    if let Some(t) = title
                        && !t.is_empty()
                    {
                        category_stack.push(t);
                    }
                    depth_is_category.push(true);
                }
            }
            Event::End(ref e) if e.name().as_ref() == b"outline" => {
                depth = depth.saturating_sub(1);
                if let Some(was_category) = depth_is_category.pop()
                    && was_category
                    && !category_stack.is_empty()
                {
                    category_stack.pop();
                }
            }
            Event::Empty(ref e) if e.name().as_ref() == b"outline" => {
                let attrs = collect_attrs(e.attributes());
                let xml_url = find_attr(&attrs, "xmlUrl").or_else(|| find_attr(&attrs, "xmlurl"));

                if let Some(url) = xml_url
                    && !url.is_empty()
                {
                    let name = find_attr(&attrs, "title")
                        .or_else(|| find_attr(&attrs, "text"))
                        .unwrap_or_else(|| url.clone());

                    bookmarks.push(ParsedBookmark {
                        name,
                        url,
                        icon: None,
                        category_name: category_stack.last().cloned(),
                        visibility: Visibility::Public,
                    });
                }
            }
            Event::Eof => break,
            _ => {}
        }
        buf.clear();
    }

    Ok(ParsedData {
        categories: Vec::new(),
        bookmarks,
        services: Vec::new(),
        themes: Vec::new(),
        settings: None,
    })
}
