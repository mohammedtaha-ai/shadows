//! Postgres-specific query builders (tsvector).

pub fn encode_tsquery(text: &str) -> String {
    text.split_whitespace()
        .map(|w| w.replace(|c: char| !c.is_alphanumeric(), ""))
        .filter(|w| !w.is_empty())
        .map(|w| format!("{w}:*"))
        .collect::<Vec<_>>()
        .join(" & ")
}
