//! SQLx storage adapters.

pub mod postgres;
pub mod sqlite;

pub const EXTRA_SCHEMA_SQLITE: &str = "
CREATE TABLE IF NOT EXISTS durable_seq_counter (n INTEGER PRIMARY KEY, _dummy INTEGER);
INSERT OR IGNORE INTO durable_seq_counter(n, _dummy) VALUES (0, 0);
CREATE TABLE IF NOT EXISTS workflow_status (
    workflow_id TEXT PRIMARY KEY NOT NULL,
    status TEXT NOT NULL,
    set_at TEXT NOT NULL
);
";
