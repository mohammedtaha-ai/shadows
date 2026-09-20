//! Postgres-specific SQL. tsvector, to_tsquery, etc.

/// Build a Postgres tsquery from a plain phrase.
pub fn encode_tsquery(text: &str) -> String {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    // naive: word tokens ANDed
    trimmed
        .split_whitespace()
        .map(|w| w.replace(|c: char| !c.is_alphanumeric(), ""))
        .filter(|w| !w.is_empty())
        .map(|w| format!("{w}:*"))
        .collect::<Vec<_>>()
        .join(" & ")
}

/// `now() at time zone 'utc'` SQL helper.
pub fn now_sql() -> &'static str {
    "now() at time zone 'utc'"
}
