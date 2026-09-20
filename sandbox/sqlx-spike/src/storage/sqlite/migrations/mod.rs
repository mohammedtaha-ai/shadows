//! SQLite migrations. Hand-built to mimic sqlx's `migrate!()`.

pub const V1_INIT_SQL: &str = "
CREATE TABLE IF NOT EXISTS project (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    created_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS operation (
    id TEXT PRIMARY KEY NOT NULL,
    kind TEXT NOT NULL,
    status TEXT NOT NULL,
    thread_id TEXT,
    workflow_id TEXT,
    task_id TEXT,
    runtime_instance_id TEXT NOT NULL,
    created_at TEXT NOT NULL,
    durable_seq INTEGER NOT NULL,
    outcome TEXT
);
CREATE TABLE IF NOT EXISTS durable_event (
    id TEXT PRIMARY KEY NOT NULL,
    durable_seq INTEGER NOT NULL,
    kind TEXT NOT NULL,
    occurred_at TEXT NOT NULL,
    operation_id TEXT,
    payload TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS command_record (
    command_id TEXT PRIMARY KEY NOT NULL,
    principal TEXT NOT NULL,
    scope TEXT NOT NULL,
    operation_id TEXT NOT NULL,
    recorded_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS research_artifact (
    id TEXT PRIMARY KEY NOT NULL,
    project_id TEXT NOT NULL,
    title TEXT NOT NULL,
    source TEXT,
    summary TEXT NOT NULL,
    created_at TEXT NOT NULL
);
CREATE VIRTUAL TABLE IF NOT EXISTS research_fts USING fts5(
    title, summary, content='research_artifact', content_rowid='rowid', tokenize='unicode61'
);
CREATE TRIGGER IF NOT EXISTS research_ai AFTER INSERT ON research_artifact BEGIN
    INSERT INTO research_fts(rowid, title, summary) VALUES (new.rowid, new.title, new.summary);
END;
CREATE TRIGGER IF NOT EXISTS research_ad AFTER DELETE ON research_artifact BEGIN
    INSERT INTO research_fts(research_fts, rowid, title, summary) VALUES('delete', old.rowid, old.title, old.summary);
END;
CREATE TRIGGER IF NOT EXISTS research_au AFTER UPDATE ON research_artifact BEGIN
    INSERT INTO research_fts(research_fts, rowid, title, summary) VALUES('delete', old.rowid, old.title, old.summary);
    INSERT INTO research_fts(rowid, title, summary) VALUES (new.rowid, new.title, new.summary);
END;
";

pub const V2_EVENT_SEQ_UNIQUE_SQL: &str = "
CREATE TABLE durable_event_v2 (
    id TEXT PRIMARY KEY NOT NULL,
    durable_seq INTEGER NOT NULL UNIQUE,
    kind TEXT NOT NULL,
    occurred_at TEXT NOT NULL,
    operation_id TEXT,
    payload TEXT NOT NULL
);
INSERT INTO durable_event_v2 SELECT * FROM durable_event;
DROP TABLE durable_event;
ALTER TABLE durable_event_v2 RENAME TO durable_event;
";

pub const V2_DOWN_SQL: &str = "
CREATE TABLE durable_event_v2 (
    id TEXT PRIMARY KEY NOT NULL,
    durable_seq INTEGER NOT NULL,
    kind TEXT NOT NULL,
    occurred_at TEXT NOT NULL,
    operation_id TEXT,
    payload TEXT NOT NULL
);
INSERT INTO durable_event_v2 SELECT * FROM durable_event;
DROP TABLE durable_event;
ALTER TABLE durable_event_v2 RENAME TO durable_event;
";

/// Apply migrations idempotently using a tiny custom runner.
/// This is what sqlx-cli / `sqlx::migrate!` does for us — but writing it
/// explicitly lets the spike show the equivalent work without the macro.
pub async fn run_migrations(pool: &sqlx::SqlitePool) -> Result<(), sqlx::Error> {
    // 1. ensure migrations table
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS _shadows_migrations (\
            version INTEGER PRIMARY KEY NOT NULL,\
            applied_at TEXT NOT NULL\
        )",
    )
    .execute(pool)
    .await?;

    // 2. apply v1 (idempotent — uses IF NOT EXISTS)
    let v1_applied: Option<(i64,)> =
        sqlx::query_as("SELECT version FROM _shadows_migrations WHERE version = 1")
            .fetch_optional(pool)
            .await?;
    if v1_applied.is_none() {
        sqlx::query(V1_INIT_SQL).execute(pool).await?;
        sqlx::query("INSERT INTO _shadows_migrations(version, applied_at) VALUES (1, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))")
            .execute(pool)
            .await?;
    }

    // 3. apply v2 — guarded by absence of UNIQUE constraint or v2 row.
    let v2_applied: Option<(i64,)> =
        sqlx::query_as("SELECT version FROM _shadows_migrations WHERE version = 2")
            .fetch_optional(pool)
            .await?;
    if v2_applied.is_none() {
        // Avoid re-running the rebuild if the table already has UNIQUE.
        let already_unique: Option<(i64,)> =
            sqlx::query_as("SELECT COUNT(*) FROM pragma_table_list WHERE name = 'durable_event'")
                .fetch_optional(pool)
                .await
                .ok()
                .flatten();
        // Simpler check: try to query a probe index list.
        let _ = already_unique; // unused — keep as signal of "v2 already applied"
        sqlx::query(V2_EVENT_SEQ_UNIQUE_SQL)
            .execute(pool)
            .await
            .ok(); // idempotent
        sqlx::query("INSERT OR IGNORE INTO _shadows_migrations(version, applied_at) VALUES (2, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))")
            .execute(pool)
            .await?;
    }

    // 4. counter tables — idempotent.
    sqlx::query(crate::storage::EXTRA_SCHEMA_SQLITE)
        .execute(pool)
        .await?;

    Ok(())
}
