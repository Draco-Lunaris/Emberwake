//! Fuzzy matcher: subsequence-based scoring with bonus for consecutive matches.
//! Client-side only — no network round-trip.

/// A single search result with a score and the matched item's name.
#[derive(Debug, Clone, PartialEq)]
pub struct FuzzyResult {
    pub name: String,
    pub url: String,
    pub score: i64,
}

/// Fuzzy match a query against a list of (name, url) items.
/// Returns results sorted by score (descending). Items with no match are excluded.
pub fn fuzzy_match(query: &str, items: &[(String, String)]) -> Vec<FuzzyResult> {
    if query.is_empty() {
        return items
            .iter()
            .map(|(name, url)| FuzzyResult {
                name: name.clone(),
                url: url.clone(),
                score: 0,
            })
            .collect();
    }

    let query_lower = query.to_lowercase();
    let mut results: Vec<FuzzyResult> = Vec::new();

    for (name, url) in items {
        let name_lower = name.to_lowercase();
        if let Some(score) = score_subsequence(&query_lower, &name_lower) {
            results.push(FuzzyResult {
                name: name.clone(),
                url: url.clone(),
                score,
            });
        }
    }

    results.sort_by_key(|r| std::cmp::Reverse(r.score));
    results
}

/// Score a query as a subsequence of text.
/// Returns Some(score) if all query chars appear in order in text, None otherwise.
/// Scoring: +10 per consecutive match, +5 for matching at word boundaries,
/// +1 per matched character, -1 per skipped character.
fn score_subsequence(query: &str, text: &str) -> Option<i64> {
    let query_chars: Vec<char> = query.chars().collect();
    let text_chars: Vec<char> = text.chars().collect();

    if query_chars.is_empty() {
        return Some(0);
    }
    if query_chars.len() > text_chars.len() {
        return None;
    }

    let mut score: i64 = 0;
    let mut qi = 0;
    let mut prev_match: Option<usize> = None;

    for (ti, tc) in text_chars.iter().enumerate() {
        if qi >= query_chars.len() {
            break;
        }
        if *tc == query_chars[qi] {
            // Base match score
            score += 1;

            // Consecutive match bonus
            if let Some(prev) = prev_match
                && ti == prev + 1
            {
                score += 10;
            }

            // Word boundary bonus (start of text or after space/dash/underscore)
            if ti == 0 || matches!(text_chars[ti - 1], ' ' | '-' | '_' | '.') {
                score += 5;
            }

            prev_match = Some(ti);
            qi += 1;
        } else {
            // Penalty for skipped characters between matches
            if prev_match.is_some() {
                score -= 1;
            }
        }
    }

    if qi == query_chars.len() {
        // Bonus for matching at the start of text
        if text_chars.first() == query_chars.first() {
            score += 10;
        }
        Some(score)
    } else {
        None
    }
}

#[cfg(all(test, target_arch = "wasm32"))]
mod search_test {
    use wasm_bindgen_test::wasm_bindgen_test;

    use super::fuzzy_match;

    /// T020: Fuzzy matcher ranks a misspelled query correctly.
    #[wasm_bindgen_test]
    fn fuzzy_matcher_ranks_misspelled_query() {
        let items = vec![
            ("GitHub".to_string(), "https://github.com".to_string()),
            ("GitLab".to_string(), "https://gitlab.com".to_string()),
            ("Gmail".to_string(), "https://mail.google.com".to_string()),
            ("Reddit".to_string(), "https://reddit.com".to_string()),
        ];

        // Misspelled "gthub" should rank "GitHub" at the top
        let results = fuzzy_match("gthub", &items);
        assert!(!results.is_empty(), "should have results");
        assert_eq!(
            results[0].name,
            "GitHub",
            "GitHub should rank first for misspelled 'gthub', got: {:?}",
            results.iter().map(|r| &r.name).collect::<Vec<_>>()
        );
    }

    /// Fuzzy matcher returns all items for empty query.
    #[wasm_bindgen_test]
    fn fuzzy_matcher_empty_query_returns_all() {
        let items = vec![
            ("GitHub".to_string(), "https://github.com".to_string()),
            ("Reddit".to_string(), "https://reddit.com".to_string()),
        ];

        let results = fuzzy_match("", &items);
        assert_eq!(results.len(), 2, "empty query should return all items");
    }

    /// Fuzzy matcher excludes non-matching items.
    #[wasm_bindgen_test]
    fn fuzzy_matcher_excludes_non_matching() {
        let items = vec![
            ("GitHub".to_string(), "https://github.com".to_string()),
            ("Reddit".to_string(), "https://reddit.com".to_string()),
        ];

        let results = fuzzy_match("xyz", &items);
        assert!(results.is_empty(), "non-matching query should return empty");
    }
}
