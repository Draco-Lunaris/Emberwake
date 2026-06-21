//! Search island: fuzzy match + prefix routing.
//! Client-side only — no network round-trip for fuzzy matching.
//! Results filter instantly as user types.

pub mod fuzzy;

use leptos::prelude::*;

use crate::domain::SearchProvider;
use fuzzy::{FuzzyResult, fuzzy_match};

/// Search island component.
/// Takes a list of (name, url) items and optional search providers for prefix routing.
#[component]
pub fn SearchIsland(items: Vec<(String, String)>, providers: Vec<SearchProvider>) -> impl IntoView {
    let query = RwSignal::new(String::new());
    let providers = StoredValue::new(providers);
    let items = StoredValue::new(items);

    let results = Memo::new(move |_| {
        let q = query.get();
        if q.is_empty() {
            Vec::new()
        } else {
            let provs = providers.get_value();
            for p in &provs {
                let prefix = format!("{} ", p.prefix);
                if q.starts_with(&prefix) {
                    let search_term = &q[prefix.len()..];
                    let url = p.url_template.replace("{q}", search_term);
                    return vec![FuzzyResult {
                        name: format!("{}: {}", p.name, search_term),
                        url,
                        score: i64::MAX,
                    }];
                }
            }
            fuzzy_match(&q, &items.get_value())
        }
    });

    view! {
        <div class="search-island">
            <input
                type="text"
                class="search-input"
                placeholder="Search..."
                prop:value=move || query.get()
                on:input=move |ev| {
                    query.set(event_target_value(&ev));
                }
                on:keydown=move |ev| {
                    if ev.key() == "Enter" {
                        let q = query.get();
                        let res = results.get();
                        if let Some(first) = res.first() {
                            if let Some(window) = web_sys::window() {
                                let _ = window.location().set_href(&first.url);
                            }
                        } else {
                            let provs = providers.get_value();
                            for p in &provs {
                                let prefix = format!("{} ", p.prefix);
                                if q.starts_with(&prefix) {
                                    let search_term = &q[prefix.len()..];
                                    let url = p.url_template.replace("{q}", search_term);
                                    if let Some(window) = web_sys::window() {
                                        let _ = window.location().set_href(&url);
                                    }
                                    break;
                                }
                            }
                        }
                    }
                }
            />
            <ul class="search-results">
                {move || {
                    results
                        .get()
                        .into_iter()
                        .map(|r| {
                            view! {
                                <li class="search-result">
                                    <a href=r.url.clone()>{r.name.clone()}</a>
                                </li>
                            }
                        })
                        .collect::<Vec<_>>()
                }}
            </ul>
        </div>
    }
}
