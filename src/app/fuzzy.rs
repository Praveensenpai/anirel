//! Fuzzy title matching for the filter bar.

/// Lowercase and keep only letters and digits, so spacing and punctuation
/// never block a match (`Yuru Camp△` and `yurcamp` normalise alike).
fn normalise(value: &str) -> String {
    value
        .chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(|c| c.to_lowercase())
        .collect()
}

/// Edit budget grows with the query so short filters stay precise.
fn budget(len: usize) -> usize {
    match len {
        0..=2 => 0,
        3..=5 => 1,
        _ => 2,
    }
}

/// Semi-global edit distance: the query may start and end anywhere in the
/// text, so `camp` still matches `Yuru Camp Season 2`.
fn within_budget(text: &[char], query: &[char], budget: usize) -> bool {
    let mut previous = vec![0usize; text.len() + 1];
    for needle in query {
        let mut current = vec![usize::MAX; text.len() + 1];
        for (index, candidate) in text.iter().enumerate() {
            let substitute = previous[index].saturating_add(usize::from(needle != candidate));
            let insert = current[index].saturating_add(1);
            let delete = previous[index + 1].saturating_add(1);
            current[index + 1] = substitute.min(insert).min(delete);
        }
        previous = current;
    }
    previous.iter().any(|&distance| distance <= budget)
}

/// Does `filter` match `title`? Exact substrings win; dropped letters and
/// typos still match once the query is long enough.
pub fn matches(title: &str, filter: &str) -> bool {
    let query: Vec<char> = normalise(filter).chars().collect();
    if query.is_empty() {
        return true;
    }
    let text: Vec<char> = normalise(title).chars().collect();
    if text.windows(query.len()).any(|window| window == query) {
        return true;
    }
    within_budget(&text, &query, budget(query.len()))
}
