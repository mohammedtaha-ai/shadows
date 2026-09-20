//! SQLite-specific SQL: FTS5, strftime, etc.

/// Doubled quotes for FTS5 phrase syntax.
pub fn encode_fts5_literal_query(text: &str) -> String {
    let escaped = text.replace('"', "\"\"");
    format!("\"{}\"", escaped)
}

pub fn fts_create_table_sql() -> &'static str {
    "CREATE VIRTUAL TABLE IF NOT EXISTS research_fts USING fts5(\
        title, summary, content='research_artifact', content_rowid='rowid', tokenize='unicode61')"
}

pub fn now_sql() -> &'static str {
    "strftime('%Y-%m-%dT%H:%M:%fZ', 'now')"
}
