## Task 2: SQLite open, connection policy, and the seven-table migration

**Files:**
- Create: `migrations/0001_milestone0.sql`, `src/storage/mod.rs`, `src/storage/sqlite/mod.rs`
- Modify: `src/lib.rs` (add `pub mod storage;`), `Cargo.toml` (no change expected)
- Test: `tests/storage_contract.rs`

**Interfaces:**
- Consumes: `Config` from Task 1.
- Produces: `shadows::storage::Storage::open(db_path: &Path) -> Result<Storage, StorageError>`; `Storage::reader(&self) -> &SqlitePool`; `StorageError` enum.

- [ ] **Step 1: Write the failing test**

`tests/storage_contract.rs`:

```rust
use shadows::storage::Storage;

#[tokio::test]
async fn fresh_database_migrates_and_applies_the_connection_policy() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("shadows.sqlite3");
    let storage = Storage::open(&db).await.expect("open should succeed");

    let journal: String = sqlx::query_scalar("PRAGMA journal_mode")
        .fetch_one(storage.reader())
        .await
        .unwrap();
    assert_eq!(journal.to_lowercase(), "wal", "spec §6.23 requires WAL");

    let fk: i64 = sqlx::query_scalar("PRAGMA foreign_keys")
        .fetch_one(storage.reader())
        .await
        .unwrap();
    assert_eq!(fk, 1, "spec §6.23 requires foreign_keys = ON");

    let mut tables: Vec<String> = sqlx::query_scalar(
        "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' \
         AND name <> '_sqlx_migrations' ORDER BY name",
    )
    .fetch_all(storage.reader())
    .await
    .unwrap();
    tables.sort();

    assert_eq!(
        tables,
        vec![
            "command_record",
            "durable_event",
            "operation",
            "planning_thread",
            "project",
            "runtime_instance",
            "thread_entry",
        ],
        "spec §7.1: the first migration carries only the milestone's tables"
    );
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test storage_contract`
Expected: FAIL — `shadows::storage` does not exist.

- [ ] **Step 3: Write `migrations/0001_milestone0.sql`**

Transcribed from spec §6.3, §6.4, §6.5, §6.13, §6.14, §6.18, §6.19. Columns for later features are absent by design.

```sql
CREATE TABLE project (
    id                  TEXT PRIMARY KEY,
    slug                TEXT NOT NULL UNIQUE,
    name                TEXT NOT NULL,
    default_config_ref  TEXT NULL,
    created_at          TEXT NOT NULL
);

CREATE TABLE planning_thread (
    id                  TEXT PRIMARY KEY,
    project_id          TEXT NOT NULL REFERENCES project(id) ON DELETE RESTRICT,
    title               TEXT NOT NULL,
    status              TEXT NOT NULL CHECK (status IN ('Open','Closed')),
    next_entry_ordinal  INTEGER NOT NULL DEFAULT 1 CHECK (next_entry_ordinal > 0),
    created_at          TEXT NOT NULL
);
CREATE INDEX idx_thread_by_project ON planning_thread(project_id, created_at, id);

CREATE TABLE thread_entry (
    id           TEXT PRIMARY KEY,
    thread_id    TEXT NOT NULL REFERENCES planning_thread(id) ON DELETE RESTRICT,
    ordinal      INTEGER NOT NULL CHECK (ordinal > 0),
    kind         TEXT NOT NULL,
    author_kind  TEXT NOT NULL,
    author_id    TEXT NOT NULL,
    body         TEXT NOT NULL,
    refs_json    TEXT NOT NULL DEFAULT '[]',
    created_at   TEXT NOT NULL,
    UNIQUE (thread_id, ordinal)
);

CREATE TABLE runtime_instance (
    id          TEXT PRIMARY KEY,
    version     TEXT NOT NULL,
    started_at  TEXT NOT NULL,
    stopped_at  TEXT NULL,
    stop_kind   TEXT NULL CHECK (stop_kind IS NULL OR stop_kind IN ('Graceful','Escalated')),
    CHECK ((stopped_at IS NULL) = (stop_kind IS NULL))
);

CREATE TABLE operation (
    id                        TEXT PRIMARY KEY,
    kind                      TEXT NOT NULL CHECK (kind IN ('PlannerTurn')),
    status_kind               TEXT NOT NULL DEFAULT 'Pending'
        CHECK (status_kind IN ('Pending','Running','Completed','Failed','Cancelled','Interrupted')),
    thread_id                 TEXT NULL REFERENCES planning_thread(id) ON DELETE RESTRICT,
    runtime_instance_id       TEXT NOT NULL REFERENCES runtime_instance(id) ON DELETE RESTRICT,
    outcome_json              TEXT NULL,
    failure_stage             TEXT NULL,
    failure_reason            TEXT NULL,
    interrupt_reason          TEXT NULL,
    cancel_requested_at       TEXT NULL,
    cancel_requested_by_kind  TEXT NULL,
    cancel_requested_by_id    TEXT NULL,
    created_at                TEXT NOT NULL,
    started_at                TEXT NULL,
    finished_at               TEXT NULL,

    -- Spec §6.14 state-shape constraints, one CHECK per status.
    CHECK (status_kind <> 'Pending' OR (
        started_at IS NULL AND finished_at IS NULL AND outcome_json IS NULL
        AND failure_stage IS NULL AND failure_reason IS NULL AND interrupt_reason IS NULL)),
    CHECK (status_kind <> 'Running' OR (
        started_at IS NOT NULL AND finished_at IS NULL AND outcome_json IS NULL
        AND failure_stage IS NULL AND failure_reason IS NULL AND interrupt_reason IS NULL)),
    CHECK (status_kind <> 'Completed' OR (
        started_at IS NOT NULL AND finished_at IS NOT NULL AND outcome_json IS NOT NULL
        AND failure_stage IS NULL AND failure_reason IS NULL AND interrupt_reason IS NULL)),
    CHECK (status_kind <> 'Failed' OR (
        finished_at IS NOT NULL AND failure_stage IS NOT NULL AND failure_reason IS NOT NULL
        AND outcome_json IS NULL AND interrupt_reason IS NULL)),
    CHECK (status_kind <> 'Cancelled' OR (
        finished_at IS NOT NULL AND outcome_json IS NULL
        AND failure_stage IS NULL AND interrupt_reason IS NULL)),
    CHECK (status_kind <> 'Interrupted' OR (
        finished_at IS NOT NULL AND outcome_json IS NULL
        AND failure_stage IS NULL AND interrupt_reason IS NOT NULL)),

    -- Spec §6.14: the three cancellation columns are all NULL or all NOT NULL.
    CHECK ((cancel_requested_at IS NULL) = (cancel_requested_by_kind IS NULL)),
    CHECK ((cancel_requested_at IS NULL) = (cancel_requested_by_id IS NULL))
);
CREATE INDEX idx_operation_by_thread ON operation(thread_id, created_at, id);
CREATE INDEX idx_operation_non_terminal
    ON operation(runtime_instance_id)
    WHERE status_kind IN ('Pending','Running');

CREATE TABLE durable_event (
    seq             INTEGER PRIMARY KEY AUTOINCREMENT,
    event_id        TEXT NOT NULL UNIQUE,
    kind            TEXT NOT NULL,
    project_id      TEXT NULL REFERENCES project(id) ON DELETE RESTRICT,
    thread_id       TEXT NULL REFERENCES planning_thread(id) ON DELETE RESTRICT,
    operation_id    TEXT NULL REFERENCES operation(id) ON DELETE RESTRICT,
    actor_kind      TEXT NULL,
    actor_id        TEXT NULL,
    causation_kind  TEXT NULL,
    causation_ref   TEXT NULL,
    correlation_id  TEXT NULL,
    payload_json    TEXT NOT NULL,
    created_at      TEXT NOT NULL,
    CHECK ((actor_kind IS NULL) = (actor_id IS NULL)),
    CHECK ((causation_kind IS NULL) = (causation_ref IS NULL))
);
CREATE INDEX idx_event_project   ON durable_event(project_id, seq);
CREATE INDEX idx_event_thread    ON durable_event(thread_id, seq);
CREATE INDEX idx_event_operation ON durable_event(operation_id, seq);

CREATE TABLE command_record (
    principal_kind       TEXT NOT NULL,
    principal_id         TEXT NOT NULL,
    command_scope_kind   TEXT NOT NULL,
    command_scope_key    TEXT NOT NULL,
    command_id           TEXT NOT NULL,
    command_kind         TEXT NOT NULL,
    command_schema_ver   INTEGER NOT NULL CHECK (command_schema_ver > 0),
    request_fingerprint  TEXT NOT NULL,
    outcome_kind         TEXT NOT NULL CHECK (outcome_kind IN ('Entity','NoContent')),
    entity_kind          TEXT NULL,
    outcome_ref          TEXT NULL,
    recorded_at          TEXT NOT NULL,
    PRIMARY KEY (principal_kind, principal_id, command_scope_kind, command_scope_key, command_id),
    CHECK (outcome_kind <> 'NoContent' OR (entity_kind IS NULL AND outcome_ref IS NULL)),
    CHECK (outcome_kind <> 'Entity'    OR (entity_kind IS NOT NULL AND outcome_ref IS NOT NULL))
);
```

Note the `operation.kind` CHECK lists only `PlannerTurn`. Milestone 0 has no other kind, and a CHECK that permits kinds nothing can create is a claim the schema cannot back. Later milestones widen it in their own migration.

- [ ] **Step 4: Write `src/storage/sqlite/mod.rs`**

The writer strategy is fixed by measurement (spec §6.23, evidence `WAL_VALIDATION.md`): one serialized write connection, `BEGIN IMMEDIATE`, `busy_timeout` as a backstop, a separate read pool.

```rust
use std::path::Path;
use std::str::FromStr;
use std::time::Duration;

use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous};
use sqlx::{SqliteConnection, SqlitePool};
use tokio::sync::Mutex;

#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("storage is unavailable: {0}")]
    Unavailable(String),
    #[error("migration failed: {0}")]
    Migration(#[from] sqlx::migrate::MigrateError),
    #[error("constraint violated: {0}")]
    Constraint(String),
    #[error("not found: {0}")]
    NotFound(&'static str),
    #[error("transition conflict: expected {expected}, found {found}")]
    TransitionConflict { expected: String, found: String },
    #[error("command conflict: the same command id was reused with a different request")]
    CommandConflict,
    #[error(transparent)]
    Database(#[from] sqlx::Error),
}

/// All writes are serialized through `write`. All reads use `read`.
///
/// This is not caution, it is measurement. Deferred `BEGIN` succeeded on 3-27%
/// of read-then-write transactions and `busy_timeout` did not rescue it;
/// `BEGIN IMMEDIATE` on its own left 25-55 failures per 1600 transactions; one
/// serialized write connection had zero failures at higher throughput.
/// See `docs/evidence/persistence/WAL_VALIDATION.md`.
pub struct Storage {
    read: SqlitePool,
    write: Mutex<SqliteConnection>,
}

impl Storage {
    pub async fn open(db_path: &Path) -> Result<Self, StorageError> {
        let url = format!("sqlite://{}", db_path.to_string_lossy().replace('\\', "/"));
        let opts = SqliteConnectOptions::from_str(&url)
            .map_err(|e| StorageError::Unavailable(e.to_string()))?
            .create_if_missing(true)
            .foreign_keys(true)
            .journal_mode(SqliteJournalMode::Wal)
            .synchronous(SqliteSynchronous::Normal)
            .busy_timeout(Duration::from_millis(5000));

        let read = SqlitePoolOptions::new()
            .max_connections(8)
            .connect_with(opts.clone())
            .await?;

        sqlx::migrate!("./migrations").run(&read).await?;

        let write = sqlx::ConnectOptions::connect(&opts).await?;
        Ok(Self { read, write: Mutex::new(write) })
    }

    pub fn reader(&self) -> &SqlitePool {
        &self.read
    }
}
```

- [ ] **Step 5: Write `src/storage/mod.rs` and register the module**

```rust
mod sqlite;

pub use sqlite::{Storage, StorageError};
```

Add `pub mod storage;` to `src/lib.rs`.

- [ ] **Step 6: Run test to verify it passes**

Run: `cargo test --test storage_contract`
Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add migrations src/storage src/lib.rs tests/storage_contract.rs
git commit -m "feat(storage): open SQLite under the measured writer policy with the seven milestone tables"
```

---

