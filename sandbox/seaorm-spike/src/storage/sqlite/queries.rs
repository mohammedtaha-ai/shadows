//! SQL-only constructs for SQLite. This is where FTS5 / strftime live.

use sea_orm::DbBackend;

/// Build a sqlite FTS5 MATCH expression.
/// Doubled quotes for FTS5 phrase syntax.
pub fn encode_fts5_literal_query(text: &str) -> String {
    let escaped = text.replace('"', "\"\"");
    format!("\"{}\"", escaped)
}

pub fn fts_create_table_sql() -> &'static str {
    "CREATE VIRTUAL TABLE IF NOT EXISTS research_fts USING fts5(\
        title, summary, content='research_artifact', content_rowid='rowid', tokenize='unicode61')"
}

pub fn fts_insert_trigger_sql() -> &'static str {
    "CREATE TRIGGER IF NOT EXISTS research_fts_ai AFTER INSERT ON research_artifact BEGIN \
        INSERT INTO research_fts(rowid, title, summary) VALUES (new.rowid, new.title, new.summary); \
     END"
}

/// Uses sqlite rowid (NOT portable to Postgres).
pub fn order_by_rowid_sql() -> String {
    "ORDER BY created_at ASC, rowid ASC".to_string()
}

pub fn now_sql() -> &'static str {
    "strftime('%Y-%m-%dT%H:%M:%fZ', 'now')"
}

pub fn fetch_search_stmt_sql(limit: u32) -> String {
    format!(
        "SELECT rowid AS ref_rowid, snippet(research_fts, 1, '[', ']', '…', 10) AS snip \
         FROM research_fts WHERE research_fts MATCH ?1 LIMIT {limit}"
    )
}

#[allow(dead_code)]
pub fn backend() -> DbBackend {
    DbBackend::Sqlite
}
