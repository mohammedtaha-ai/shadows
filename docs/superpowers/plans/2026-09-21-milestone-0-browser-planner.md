# Milestone 0 — Runnable Browser Planner Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Start `shadows serve`, open the printed URL in any browser, pick a local project, create or resume a planning thread, run one real Claude Planner turn with live output, stop it with confirmed process-tree termination, restart the daemon, and see the conversation still there.

**Architecture:** One Rust crate with a library and a single binary. Modules appear only when this milestone needs them. All durable writes go through one serialized write connection opening `BEGIN IMMEDIATE`; reads use a separate pool. The browser client is one hand-written HTML page embedded with `include_str!` and served by `axum`, which also carries commands over HTTP and output over SSE. Claude runs as a managed child process under OS-level containment, and its newline-delimited JSON stream is split into transient deltas that are forwarded and never stored, and durable entries that are written before they are forwarded.

**Tech Stack:** Rust 1.94+ (validated on 1.96), `sqlx` 0.9 (sqlite), `tokio`, `axum` 0.8, `tower-http`, `tracing` + `tracing-subscriber`, `clap`, `thiserror`, `anyhow` (bootstrap only), `serde`/`serde_json`, `uuid`, `time`, `process-wrap`, `windows` (Job Objects).

**Spec:** [`docs/superpowers/specs/README.md`](../specs/README.md) — the index. This plan implements §11.1 and draws on §1, §2, §5, §6, §7, §8.

**Evidence this plan depends on:**
- [`docs/evidence/persistence/WAL_VALIDATION.md`](../../evidence/persistence/WAL_VALIDATION.md) — the writer strategy in Task 3 is not a guess.
- [`docs/evidence/harness/SERVE_STREAM_SPIKE.md`](../../evidence/harness/SERVE_STREAM_SPIKE.md) — the stream contract in Task 8 is measured, including exact flags and line shapes.

---

## Global Constraints

Every task's requirements implicitly include this section. Values are copied verbatim from the spec.

- **Rust floor:** 1.94+ minimum for the selected SQLx 0.9 line. Development validation on 1.96. Set `rust-version = "1.94"` in `Cargo.toml`.
- **One crate.** Module boundaries are cheap; crate boundaries are expensive. Do not create a workspace, a `shadows-core`, or a `shadows-storage`.
- **Only `storage/` may import `sqlx`.** Domain and application modules carry no persistence imports (`sqlx`, `Row`, `Pg*`, `Sqlite*`).
- **Only `process/` may call `tokio::process` or `process-wrap`.** `agent/` constructs a `ProcessSpec` and hands it over.
- **Only `protocol/` owns HTTP and SSE types.** They do not appear in domain or application signatures.
- **Ordering is explicit.** Never use `rowid`, physical insertion order, or an unordered `SELECT`. Order by `durable_event.seq` or `thread_entry.ordinal`.
- **Durable state mutation + durable event is atomic**, in one transaction. External mutations add a `CommandRecord` to that same transaction.
- **No public raw `append_event`.** Events are appended only inside a capability that also writes state.
- **Secrets are references until spawn.** Milestone 0 resolves no secrets — Claude authenticates itself — but never log a child's full environment.
- **`shadows serve` never opens a browser.** It prints one local address and stops there.
- **Ids are UUID-v4 newtypes.** Timestamps are RFC3339 UTC `TEXT`.
- **No module is created before the task that fills it.** Do not scaffold `workflow/`, `scheduler/`, `execution/`, `verification/`, or `mcp/` in this milestone. An empty module documenting its own absence is a defect.

### Modules this milestone builds

```text
cli/  runtime/  project/  thread/  command/  operation/  planner/
agent/  process/  storage/  events/  protocol/
config/  error/  tracing/
```

### Modules this milestone does not create

```text
workflow/  scheduler/  execution/  verification/  mcp/  secrets/
```

---

## File Structure

```text
Cargo.toml                          crate manifest, rust-version, deps
src/main.rs                         binary entry; clap; delegates to cli/
src/lib.rs                          module declarations only
src/error.rs                        AppFailure, ErrorCode, FailureClass, RetryClass
src/tracing.rs                      subscriber init, correlation field helpers
src/config.rs                       Config: db path, bind address, harness path
src/cli/mod.rs                      serve command wiring
src/runtime/mod.rs                  Runtime: owns instance id, startup, shutdown
src/runtime/recovery.rs             orphan reconciliation at startup
src/project/mod.rs                  Project domain type + local-directory selection
src/thread/mod.rs                   PlanningThread, ThreadEntry domain types
src/command/mod.rs                  CommandContext, CommandId, request fingerprint
src/operation/mod.rs                Operation, OperationStatus, OperationKind
src/planner/mod.rs                  planner turn orchestration (start, stream, stop)
src/agent/mod.rs                    AgentHarness trait, AgentInvocation
src/agent/claude.rs                 Claude harness: flags, stream classification
src/process/mod.rs                  ProcessSpec, ProcessHandle, spawn, terminate
src/process/containment_windows.rs  Job Object containment
src/process/containment_unix.rs     parent-death + tree cleanup
src/events/mod.rs                   DurableEvent, EventCursor, live bus
src/storage/mod.rs                  Storage facade: capabilities only
src/storage/sqlite/mod.rs           pool setup, connection policy, write serialization
src/storage/sqlite/project.rs       project capabilities
src/storage/sqlite/thread.rs        thread + entry capabilities, ordinal allocation
src/storage/sqlite/operation.rs     operation capabilities, CAS transitions
src/storage/sqlite/runtime.rs       runtime_instance + orphan scan
src/storage/sqlite/events.rs        durable_event append (private), cursor reads
src/protocol/mod.rs                 axum router, command handlers
src/protocol/sse.rs                 SSE stream: durable replay then live handoff
src/protocol/index.html             the entire web client
migrations/0001_milestone0.sql      the seven milestone tables and their indexes
tests/storage_contract.rs           atomicity, idempotency, ordinal, CAS
tests/recovery.rs                   crash recovery and orphan reconciliation
tests/containment.rs                daemon -> child -> grandchild termination
tests/harness_stream.rs             stream classification against captured fixtures
tests/fixtures/claude_turn.jsonl    a real captured Claude turn
```

**Seven tables only.** `project`, `planning_thread`, `thread_entry`, `runtime_instance`, `operation`, `durable_event`, `command_record`. The other ten tables and the FTS5 table in spec §6.1 belong to later milestones and must not appear in `0001_milestone0.sql`.

---

## Task 1: Crate scaffold, `shadows serve`, structured tracing

**Files:**
- Create: `Cargo.toml`, `src/main.rs`, `src/lib.rs`, `src/config.rs`, `src/tracing.rs`, `src/cli/mod.rs`, `src/error.rs`
- Create: `tests/serve_smoke.rs`

**Interfaces:**
- Consumes: nothing.
- Produces: `shadows::config::Config { db_path: PathBuf, bind: SocketAddr, harness_path: PathBuf }`; `shadows::tracing::init(verbose: bool)`; `shadows::cli::serve(config: Config) -> anyhow::Result<()>`; `shadows::error::{AppFailure, ErrorCode, FailureClass, RetryClass}`.

- [ ] **Step 1: Write the failing test**

`tests/serve_smoke.rs`:

```rust
use std::process::{Command, Stdio};
use std::io::{BufRead, BufReader};

/// `shadows serve` must print exactly one local address and must not open a
/// browser. Spec §1.0 and §11.1 both require the daemon to stop at printing.
#[test]
fn serve_prints_one_local_address_and_does_not_open_a_browser() {
    let exe = env!("CARGO_BIN_EXE_shadows");
    let tmp = tempfile::tempdir().unwrap();
    let mut child = Command::new(exe)
        .arg("serve")
        .arg("--db")
        .arg(tmp.path().join("shadows.sqlite3"))
        .arg("--bind")
        .arg("127.0.0.1:0")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("daemon should start");

    let stdout = child.stdout.take().unwrap();
    let mut lines = BufReader::new(stdout).lines();
    let first = lines.next().expect("expected a line").unwrap();

    assert!(
        first.starts_with("shadows serve listening on http://127.0.0.1:"),
        "unexpected first line: {first}"
    );

    child.kill().unwrap();
    child.wait().unwrap();
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test serve_smoke`
Expected: FAIL — the crate does not exist yet, so this does not compile.

- [ ] **Step 3: Write `Cargo.toml`**

```toml
[package]
name = "shadows"
version = "0.1.0"
edition = "2024"
rust-version = "1.94"
publish = false

[dependencies]
anyhow = "1"
axum = "0.8"
clap = { version = "4", features = ["derive"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
sqlx = { version = "0.9", features = ["runtime-tokio", "sqlite", "migrate"] }
thiserror = "2"
time = { version = "0.3", features = ["formatting", "parsing", "macros"] }
tokio = { version = "1", features = ["full"] }
tokio-stream = "0.1"
tower-http = { version = "0.6", features = ["trace"] }
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "json"] }
uuid = { version = "1", features = ["v4", "serde"] }

[dev-dependencies]
tempfile = "3"
```

- [ ] **Step 4: Write `src/error.rs`**

This is spec §3.2 and §3.4 made concrete. `AppFailure` is public-safe; the causal chain stays in `FailureReport` and cannot accidentally serialize.

```rust
use std::fmt;

/// Stable codes clients pattern-match on. Never match on human text.
/// Spec §3.4. `Blocked`/`Rejected` are domain outcomes and never appear here.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ErrorCode {
    ProcessSpawnFailed,
    ProcessTerminated,
    StorageUnavailable,
    StorageMigrationFailed,
    StorageConstraintViolation,
    CommandConflict,
    IdempotencyKeyRequired,
    InvalidCommand,
    InvalidCursor,
    AgentAuthFailed,
    AgentUnsupportedProfile,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FailureClass {
    Client,
    Infrastructure,
    Agent,
    Storage,
}

/// Classifying a failure as retryable does not authorize an automatic retry.
/// Spec §3.2.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RetryClass {
    Never,
    Immediate,
    Backoff,
    AfterReconfiguration,
    AfterUserAction,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct AppFailure {
    pub code: ErrorCode,
    pub class: FailureClass,
    pub retry: RetryClass,
    pub public_details: String,
}

impl fmt::Display for AppFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}: {}", self.code, self.public_details)
    }
}

impl std::error::Error for AppFailure {}

/// The internal causal chain. Deliberately not `Serialize` — spec §3.2 requires
/// that it cannot accidentally reach a client.
#[derive(Debug)]
pub struct FailureReport {
    pub failure: AppFailure,
    pub source: Option<Box<dyn std::error::Error + Send + Sync>>,
}
```

- [ ] **Step 5: Write `src/config.rs`**

`harness_path` is explicit, never resolved from `PATH` — spec §1.4, and the reason is in the harness evidence report.

```rust
use std::net::SocketAddr;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct Config {
    pub db_path: PathBuf,
    pub bind: SocketAddr,
    /// Spec §1.4: resolved from configuration, never from `PATH`. This machine
    /// carries more than one `claude-code` installation at different versions.
    pub harness_path: PathBuf,
}
```

- [ ] **Step 6: Write `src/tracing.rs`**

```rust
/// Spec §8.7. Durable transition logs are emitted only after their transaction
/// commits; that discipline lives at the call sites, not here.
pub fn init(verbose: bool) {
    use tracing_subscriber::{EnvFilter, fmt};
    let default = if verbose { "shadows=debug,info" } else { "shadows=info,warn" };
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(default));
    fmt().with_env_filter(filter).with_target(true).init();
}
```

- [ ] **Step 7: Write `src/lib.rs`**

```rust
pub mod cli;
pub mod config;
pub mod error;
pub mod tracing;
```

- [ ] **Step 8: Write `src/cli/mod.rs`**

```rust
use std::net::SocketAddr;

use crate::config::Config;

/// Binds, prints exactly one address, and serves. Spec §1.0: it never opens a
/// browser. The user chooses which browser to use.
pub async fn serve(config: Config) -> anyhow::Result<()> {
    let listener = tokio::net::TcpListener::bind(config.bind).await?;
    let addr: SocketAddr = listener.local_addr()?;
    println!("shadows serve listening on http://{addr}");
    tracing::info!(%addr, "daemon bound");

    let app = axum::Router::new().route("/health", axum::routing::get(|| async { "ok" }));
    axum::serve(listener, app).await?;
    Ok(())
}
```

- [ ] **Step 9: Write `src/main.rs`**

`anyhow` is used here and only here — spec §3.6 limits it to the bootstrap boundary.

```rust
use std::net::SocketAddr;
use std::path::PathBuf;

use clap::{Parser, Subcommand};
use shadows::config::Config;

#[derive(Parser)]
#[command(name = "shadows")]
struct Cli {
    #[arg(long, global = true)]
    verbose: bool,
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run the local daemon.
    Serve {
        #[arg(long, default_value = "shadows.sqlite3")]
        db: PathBuf,
        #[arg(long, default_value = "127.0.0.1:4318")]
        bind: SocketAddr,
        /// Path to the Claude Code executable. Spec §1.4 forbids PATH lookup.
        #[arg(long, default_value = "claude")]
        harness: PathBuf,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    shadows::tracing::init(cli.verbose);
    match cli.command {
        Commands::Serve { db, bind, harness } => {
            shadows::cli::serve(Config { db_path: db, bind, harness_path: harness }).await
        }
    }
}
```

- [ ] **Step 10: Run test to verify it passes**

Run: `cargo test --test serve_smoke`
Expected: PASS.

Then run: `cargo clippy --all-targets -- -D warnings` and `cargo fmt --check`
Expected: both clean.

- [ ] **Step 11: Commit**

```bash
git add Cargo.toml Cargo.lock src tests
git commit -m "feat: shadows serve binds, prints one address, and opens no browser"
```

---

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

## Task 3: Serialized write transactions and atomic state + event

**Files:**
- Modify: `src/storage/sqlite/mod.rs`
- Create: `src/storage/sqlite/events.rs`, `src/events/mod.rs`
- Modify: `src/lib.rs` (add `pub mod events;`)
- Test: `tests/storage_contract.rs`

**Interfaces:**
- Consumes: `Storage` from Task 2.
- Produces: `Storage::write_txn<F, T>(&self, f: F) -> Result<T, StorageError>` where `F: for<'a> FnOnce(&'a mut SqliteConnection) -> BoxFuture<'a, Result<T, StorageError>>`; `events::DurableEvent { event_id, kind, project_id, thread_id, operation_id, actor, causation, correlation_id, payload_json }`; `events::EventCursor(i64)`; private `append_event(conn, &DurableEvent, now) -> Result<i64, StorageError>`.

- [ ] **Step 1: Write the failing tests**

Append to `tests/storage_contract.rs`:

```rust
use shadows::events::{Actor, DurableEvent};

/// Spec §2.4 and cross-cutting rule 5: state and event commit together or not
/// at all. A live publication failure must never roll back committed truth, and
/// a rolled-back transaction must leave no event behind.
#[tokio::test]
async fn state_and_event_commit_atomically_or_not_at_all() {
    let tmp = tempfile::tempdir().unwrap();
    let storage = Storage::open(&tmp.path().join("s.sqlite3")).await.unwrap();

    // A transaction that fails after appending its event leaves nothing behind.
    let outcome = storage
        .write_txn(|conn| {
            Box::pin(async move {
                sqlx::query(
                    "INSERT INTO project (id, slug, name, created_at) VALUES (?,?,?,?)",
                )
                .bind("p-1").bind("demo").bind("Demo").bind("2026-09-21T00:00:00Z")
                .execute(&mut *conn)
                .await?;
                shadows::storage::test_support::append_event_for_test(
                    conn,
                    &DurableEvent::new("ProjectCreated", Actor::system())
                        .with_project("p-1")
                        .with_payload(serde_json::json!({})),
                    "2026-09-21T00:00:00Z",
                )
                .await?;
                Err::<(), _>(shadows::storage::StorageError::NotFound("forced"))
            })
        })
        .await;
    assert!(outcome.is_err());

    let projects: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM project")
        .fetch_one(storage.reader()).await.unwrap();
    let events: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM durable_event")
        .fetch_one(storage.reader()).await.unwrap();
    assert_eq!((projects, events), (0, 0), "rollback must leave neither behind");
}

/// The measured writer policy, encoded as a regression test. Concurrent
/// read-then-write transactions must all succeed. If someone later replaces the
/// serialized write connection with a pool, this test is what fails.
#[tokio::test]
async fn concurrent_read_then_write_transactions_all_succeed() {
    let tmp = tempfile::tempdir().unwrap();
    let storage = std::sync::Arc::new(
        Storage::open(&tmp.path().join("s.sqlite3")).await.unwrap(),
    );

    let mut handles = Vec::new();
    for w in 0..16 {
        let storage = storage.clone();
        handles.push(tokio::spawn(async move {
            for i in 0..25 {
                let id = format!("p-{w}-{i}");
                storage
                    .write_txn(|conn| {
                        let id = id.clone();
                        Box::pin(async move {
                            // Read first, then write: this is the shape that
                            // forces a lock upgrade under deferred BEGIN.
                            let _: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM project")
                                .fetch_one(&mut *conn).await?;
                            sqlx::query(
                                "INSERT INTO project (id, slug, name, created_at) VALUES (?,?,?,?)",
                            )
                            .bind(&id).bind(&id).bind("x").bind("2026-09-21T00:00:00Z")
                            .execute(&mut *conn).await?;
                            Ok(())
                        })
                    })
                    .await
                    .expect("no write transaction may fail under the serialized policy");
            }
        }));
    }
    for h in handles { h.await.unwrap(); }

    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM project")
        .fetch_one(storage.reader()).await.unwrap();
    assert_eq!(count, 400);
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --test storage_contract`
Expected: FAIL — `write_txn`, `events`, and `test_support` do not exist.

- [ ] **Step 3: Write `src/events/mod.rs`**

```rust
/// Spec §6.18. `seq` is assigned by the INSERT, which on SQLite can only run
/// while holding the write lock, so assignment order equals commit order. That
/// property is SQLite-specific — see the OPEN block in §6.18 before writing
/// backend-neutral cursor code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
pub struct EventCursor(pub i64);

#[derive(Debug, Clone)]
pub struct Actor {
    pub kind: String,
    pub id: String,
}

impl Actor {
    pub fn system() -> Self {
        Self { kind: "System".into(), id: "daemon".into() }
    }
    pub fn user(id: impl Into<String>) -> Self {
        Self { kind: "User".into(), id: id.into() }
    }
}

#[derive(Debug, Clone)]
pub struct DurableEvent {
    pub event_id: String,
    pub kind: String,
    pub project_id: Option<String>,
    pub thread_id: Option<String>,
    pub operation_id: Option<String>,
    pub actor: Actor,
    pub correlation_id: Option<String>,
    pub payload_json: String,
}

impl DurableEvent {
    pub fn new(kind: impl Into<String>, actor: Actor) -> Self {
        Self {
            event_id: uuid::Uuid::new_v4().to_string(),
            kind: kind.into(),
            project_id: None,
            thread_id: None,
            operation_id: None,
            actor,
            correlation_id: None,
            payload_json: "{}".into(),
        }
    }
    pub fn with_project(mut self, id: impl Into<String>) -> Self {
        self.project_id = Some(id.into());
        self
    }
    pub fn with_thread(mut self, id: impl Into<String>) -> Self {
        self.thread_id = Some(id.into());
        self
    }
    pub fn with_operation(mut self, id: impl Into<String>) -> Self {
        self.operation_id = Some(id.into());
        self
    }
    pub fn with_payload(mut self, v: serde_json::Value) -> Self {
        self.payload_json = v.to_string();
        self
    }
}
```

- [ ] **Step 4: Write `src/storage/sqlite/events.rs`**

```rust
use sqlx::SqliteConnection;

use crate::events::DurableEvent;
use super::StorageError;

/// Private on purpose. Cross-cutting rule 10 forbids a public raw
/// `append_event`: an event is appended only inside a capability that also
/// writes the state it describes.
pub(super) async fn append_event(
    conn: &mut SqliteConnection,
    event: &DurableEvent,
    now: &str,
) -> Result<i64, StorageError> {
    let seq: i64 = sqlx::query_scalar(
        "INSERT INTO durable_event
           (event_id, kind, project_id, thread_id, operation_id,
            actor_kind, actor_id, payload_json, created_at)
         VALUES (?,?,?,?,?,?,?,?,?)
         RETURNING seq",
    )
    .bind(&event.event_id)
    .bind(&event.kind)
    .bind(&event.project_id)
    .bind(&event.thread_id)
    .bind(&event.operation_id)
    .bind(&event.actor.kind)
    .bind(&event.actor.id)
    .bind(&event.payload_json)
    .bind(now)
    .fetch_one(&mut *conn)
    .await?;
    Ok(seq)
}
```

- [ ] **Step 5: Add `write_txn` to `src/storage/sqlite/mod.rs`**

```rust
use futures_core::future::BoxFuture;

impl Storage {
    /// Every write goes through here. `BEGIN IMMEDIATE` takes the write lock up
    /// front so no transaction has to upgrade mid-flight; the mutex is what
    /// removes contention entirely. Both are required — see §6.23.
    pub async fn write_txn<F, T>(&self, f: F) -> Result<T, StorageError>
    where
        F: for<'a> FnOnce(&'a mut SqliteConnection) -> BoxFuture<'a, Result<T, StorageError>>,
    {
        use sqlx::Executor;
        let mut conn = self.write.lock().await;
        conn.execute("BEGIN IMMEDIATE").await?;
        match f(&mut conn).await {
            Ok(v) => {
                conn.execute("COMMIT").await?;
                Ok(v)
            }
            Err(e) => {
                let _ = conn.execute("ROLLBACK").await;
                Err(e)
            }
        }
    }
}
```

Add `futures-core = "0.3"` to `[dependencies]`.

- [ ] **Step 6: Expose a test-only hook**

In `src/storage/mod.rs`:

```rust
/// Test-only access to a private capability. Not compiled into the library for
/// consumers, and not a public API.
#[doc(hidden)]
pub mod test_support {
    use sqlx::SqliteConnection;
    use crate::events::DurableEvent;
    use super::StorageError;

    pub async fn append_event_for_test(
        conn: &mut SqliteConnection,
        event: &DurableEvent,
        now: &str,
    ) -> Result<i64, StorageError> {
        super::sqlite::events::append_event(conn, event, now).await
    }
}
```

- [ ] **Step 7: Run tests to verify they pass**

Run: `cargo test --test storage_contract`
Expected: PASS, both tests.

- [ ] **Step 8: Commit**

```bash
git add src/events src/storage src/lib.rs Cargo.toml tests/storage_contract.rs
git commit -m "feat(storage): serialized write transactions with atomic state and event"
```

---

## Task 4: Runtime instance lifecycle and startup orphan reconciliation

**Files:**
- Create: `src/runtime/mod.rs`, `src/runtime/recovery.rs`, `src/storage/sqlite/runtime.rs`
- Modify: `src/lib.rs`, `src/storage/mod.rs`
- Test: `tests/recovery.rs`

**Interfaces:**
- Consumes: `Storage::write_txn`, `DurableEvent` from Task 3.
- Produces: `Storage::register_runtime_instance(version: &str) -> Result<RuntimeInstanceId, StorageError>`; `Storage::stop_runtime_instance(id, StopKind) -> Result<(), StorageError>`; `Storage::reconcile_orphans(current: &RuntimeInstanceId) -> Result<ReconcileReport, StorageError>`; `ReconcileReport { interrupted: Vec<String>, anomalies: Vec<String> }`; `StopKind::{Graceful, Escalated}`.

- [ ] **Step 1: Write the failing tests**

`tests/recovery.rs`:

```rust
use shadows::storage::{Storage, StopKind};

async fn seed_operation(
    storage: &Storage,
    op_id: &str,
    runtime_id: &str,
    status: &str,
) {
    let started = if status == "Pending" { None } else { Some("2026-09-21T00:00:00Z") };
    storage
        .write_txn(|conn| {
            let (op_id, runtime_id, status) = (op_id.to_string(), runtime_id.to_string(), status.to_string());
            Box::pin(async move {
                sqlx::query(
                    "INSERT INTO operation (id, kind, status_kind, runtime_instance_id, created_at, started_at)
                     VALUES (?, 'PlannerTurn', ?, ?, '2026-09-21T00:00:00Z', ?)",
                )
                .bind(&op_id).bind(&status).bind(&runtime_id).bind(started)
                .execute(&mut *conn).await?;
                Ok(())
            })
        })
        .await
        .unwrap();
}

/// Spec §8.6. A runtime that was lost leaves its operations non-terminal; the
/// next runtime resolves them to Interrupted by exact CAS, and never claims
/// they succeeded, failed, or were cancelled.
#[tokio::test]
async fn a_lost_runtimes_operations_become_interrupted() {
    let tmp = tempfile::tempdir().unwrap();
    let storage = Storage::open(&tmp.path().join("s.sqlite3")).await.unwrap();

    let old = storage.register_runtime_instance("0.1.0-old").await.unwrap();
    seed_operation(&storage, "op-pending", &old, "Pending").await;
    seed_operation(&storage, "op-running", &old, "Running").await;
    // `old` is never stopped: it was lost.

    let new = storage.register_runtime_instance("0.1.0-new").await.unwrap();
    let report = storage.reconcile_orphans(&new).await.unwrap();

    assert_eq!(report.interrupted.len(), 2);
    assert!(report.anomalies.is_empty());

    let rows: Vec<(String, String, Option<String>)> = sqlx::query_as(
        "SELECT id, status_kind, interrupt_reason FROM operation ORDER BY id",
    )
    .fetch_all(storage.reader()).await.unwrap();
    assert_eq!(rows[0].1, "Interrupted");
    assert_eq!(rows[0].2.as_deref(), Some("PreviousRuntimeEndedBeforeStart"));
    assert_eq!(rows[1].1, "Interrupted");
    assert_eq!(rows[1].2.as_deref(), Some("PreviousRuntimeEndedDuringRun"));
}

/// Spec §8.5 and §8.6. An Escalated shutdown records `stopped_at` on purpose
/// and leaves work non-terminal. Recovery selects by ownership, not by
/// `stopped_at`, so that work must still be reconciled. This is the bug the
/// review caught; the test is what stops it coming back.
#[tokio::test]
async fn an_escalated_shutdowns_operations_are_not_stranded() {
    let tmp = tempfile::tempdir().unwrap();
    let storage = Storage::open(&tmp.path().join("s.sqlite3")).await.unwrap();

    let old = storage.register_runtime_instance("0.1.0-old").await.unwrap();
    seed_operation(&storage, "op-abandoned", &old, "Running").await;
    storage.stop_runtime_instance(&old, StopKind::Escalated).await.unwrap();

    let new = storage.register_runtime_instance("0.1.0-new").await.unwrap();
    let report = storage.reconcile_orphans(&new).await.unwrap();

    assert_eq!(report.interrupted, vec!["op-abandoned".to_string()]);
    let status: String = sqlx::query_scalar("SELECT status_kind FROM operation WHERE id='op-abandoned'")
        .fetch_one(storage.reader()).await.unwrap();
    assert_eq!(status, "Interrupted");
}

/// Spec §8.5: `Graceful` is a claim only written when true. §8.6: finding one
/// that owns a non-terminal Operation is a defect, reconciled and reported,
/// never stranded and never passed over silently.
#[tokio::test]
async fn a_graceful_runtime_owning_unfinished_work_is_reported_as_an_anomaly() {
    let tmp = tempfile::tempdir().unwrap();
    let storage = Storage::open(&tmp.path().join("s.sqlite3")).await.unwrap();

    let old = storage.register_runtime_instance("0.1.0-old").await.unwrap();
    seed_operation(&storage, "op-leaked", &old, "Running").await;
    storage.stop_runtime_instance(&old, StopKind::Graceful).await.unwrap();

    let new = storage.register_runtime_instance("0.1.0-new").await.unwrap();
    let report = storage.reconcile_orphans(&new).await.unwrap();

    assert_eq!(report.interrupted, vec!["op-leaked".to_string()]);
    assert_eq!(report.anomalies, vec!["op-leaked".to_string()]);
}

/// The current runtime's own live work is never reconciled out from under it.
#[tokio::test]
async fn the_current_runtimes_own_operations_are_left_alone() {
    let tmp = tempfile::tempdir().unwrap();
    let storage = Storage::open(&tmp.path().join("s.sqlite3")).await.unwrap();

    let me = storage.register_runtime_instance("0.1.0").await.unwrap();
    seed_operation(&storage, "op-mine", &me, "Running").await;

    let report = storage.reconcile_orphans(&me).await.unwrap();
    assert!(report.interrupted.is_empty());
    let status: String = sqlx::query_scalar("SELECT status_kind FROM operation WHERE id='op-mine'")
        .fetch_one(storage.reader()).await.unwrap();
    assert_eq!(status, "Running");
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --test recovery`
Expected: FAIL — `register_runtime_instance`, `stop_runtime_instance`, and `reconcile_orphans` do not exist.

- [ ] **Step 3: Write `src/storage/sqlite/runtime.rs`**

```rust
use sqlx::SqliteConnection;

use crate::events::{Actor, DurableEvent};
use super::{Storage, StorageError, events::append_event};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StopKind {
    Graceful,
    Escalated,
}

impl StopKind {
    fn as_str(self) -> &'static str {
        match self {
            StopKind::Graceful => "Graceful",
            StopKind::Escalated => "Escalated",
        }
    }
}

#[derive(Debug, Default)]
pub struct ReconcileReport {
    pub interrupted: Vec<String>,
    /// Operations found under a `Graceful` runtime. Spec §8.5 says that cannot
    /// happen; if it does, §8.6 requires it be reported, not silently handled.
    pub anomalies: Vec<String>,
}

fn now() -> String {
    time::OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .expect("RFC3339 formatting cannot fail")
}

impl Storage {
    pub async fn register_runtime_instance(&self, version: &str) -> Result<String, StorageError> {
        let id = uuid::Uuid::new_v4().to_string();
        let ts = now();
        let (id2, version, ts2) = (id.clone(), version.to_string(), ts.clone());
        self.write_txn(move |conn| {
            Box::pin(async move {
                sqlx::query(
                    "INSERT INTO runtime_instance (id, version, started_at) VALUES (?,?,?)",
                )
                .bind(&id2).bind(&version).bind(&ts2)
                .execute(&mut *conn).await?;
                append_event(
                    conn,
                    &DurableEvent::new("RuntimeStarted", Actor::system())
                        .with_payload(serde_json::json!({ "runtime_instance_id": id2 })),
                    &ts2,
                ).await?;
                Ok(())
            })
        })
        .await?;
        Ok(id)
    }

    pub async fn stop_runtime_instance(
        &self,
        id: &str,
        kind: StopKind,
    ) -> Result<(), StorageError> {
        let (id, ts) = (id.to_string(), now());
        self.write_txn(move |conn| {
            Box::pin(async move {
                let affected = sqlx::query(
                    "UPDATE runtime_instance SET stopped_at = ?, stop_kind = ?
                     WHERE id = ? AND stopped_at IS NULL",
                )
                .bind(&ts).bind(kind.as_str()).bind(&id)
                .execute(&mut *conn).await?
                .rows_affected();
                if affected == 0 {
                    return Err(StorageError::TransitionConflict {
                        expected: "runtime with stopped_at IS NULL".into(),
                        found: "already stopped or missing".into(),
                    });
                }
                append_event(
                    conn,
                    &DurableEvent::new("RuntimeStopped", Actor::system())
                        .with_payload(serde_json::json!({
                            "runtime_instance_id": id, "stop_kind": kind.as_str()
                        })),
                    &ts,
                ).await?;
                Ok(())
            })
        })
        .await
    }

    /// Spec §8.6. Selects on **ownership alone**: every non-terminal Operation
    /// whose owning runtime is not the current one, regardless of how that
    /// runtime ended. Filtering on `stopped_at IS NULL` would permanently
    /// strand everything an Escalated shutdown left behind.
    pub async fn reconcile_orphans(
        &self,
        current: &str,
    ) -> Result<ReconcileReport, StorageError> {
        let (current, ts) = (current.to_string(), now());
        self.write_txn(move |conn| {
            Box::pin(async move {
                let rows: Vec<(String, String, Option<String>)> = sqlx::query_as(
                    "SELECT o.id, o.status_kind, r.stop_kind
                       FROM operation o
                       JOIN runtime_instance r ON r.id = o.runtime_instance_id
                      WHERE o.status_kind IN ('Pending','Running')
                        AND o.runtime_instance_id <> ?
                      ORDER BY o.id",
                )
                .bind(&current)
                .fetch_all(&mut *conn)
                .await?;

                let mut report = ReconcileReport::default();
                for (op_id, status, stop_kind) in rows {
                    let reason = match status.as_str() {
                        "Pending" => "PreviousRuntimeEndedBeforeStart",
                        _ => "PreviousRuntimeEndedDuringRun",
                    };
                    // Exact CAS on id, expected status, and previous runtime.
                    let affected = sqlx::query(
                        "UPDATE operation
                            SET status_kind = 'Interrupted',
                                interrupt_reason = ?,
                                finished_at = ?
                          WHERE id = ? AND status_kind = ? AND runtime_instance_id <> ?",
                    )
                    .bind(reason).bind(&ts).bind(&op_id).bind(&status).bind(&current)
                    .execute(&mut *conn).await?
                    .rows_affected();
                    if affected == 0 {
                        continue;
                    }
                    append_event(
                        conn,
                        &DurableEvent::new("OperationInterrupted", Actor::system())
                            .with_operation(&op_id)
                            .with_payload(serde_json::json!({ "reason": reason })),
                        &ts,
                    ).await?;
                    report.interrupted.push(op_id.clone());
                    if stop_kind.as_deref() == Some("Graceful") {
                        report.anomalies.push(op_id);
                    }
                }
                Ok(report)
            })
        })
        .await
    }
}
```

- [ ] **Step 4: Re-export from `src/storage/mod.rs`**

```rust
mod sqlite;

pub use sqlite::{ReconcileReport, StopKind, Storage, StorageError};
```

- [ ] **Step 5: Write `src/runtime/mod.rs`**

```rust
use std::sync::Arc;

use crate::storage::{ReconcileReport, StopKind, Storage, StorageError};

/// Owns this process's runtime identity for its whole lifetime. Spec §8.1:
/// startup performs migration, ownership, containment setup, and recovery
/// before any work is accepted.
pub struct Runtime {
    pub instance_id: String,
    pub storage: Arc<Storage>,
}

impl Runtime {
    pub async fn start(storage: Arc<Storage>) -> Result<(Self, ReconcileReport), StorageError> {
        let version = env!("CARGO_PKG_VERSION");
        let instance_id = storage.register_runtime_instance(version).await?;
        let report = storage.reconcile_orphans(&instance_id).await?;

        for op in &report.interrupted {
            tracing::info!(operation_id = %op, "recovery.reconcile");
        }
        for op in &report.anomalies {
            tracing::error!(
                operation_id = %op,
                "recovery.anomaly: a Graceful runtime owned a non-terminal operation"
            );
        }
        Ok((Self { instance_id, storage }, report))
    }

    pub async fn stop(&self, kind: StopKind) -> Result<(), StorageError> {
        self.storage.stop_runtime_instance(&self.instance_id, kind).await
    }
}
```

Add `pub mod runtime;` to `src/lib.rs`.

- [ ] **Step 6: Run tests to verify they pass**

Run: `cargo test --test recovery`
Expected: PASS, all four tests.

- [ ] **Step 7: Commit**

```bash
git add src/runtime src/storage src/lib.rs tests/recovery.rs
git commit -m "feat(runtime): register the instance and reconcile orphans by ownership"
```

---

## Task 5: Local-directory Project with external command idempotency

**Files:**
- Create: `src/command/mod.rs`, `src/project/mod.rs`, `src/storage/sqlite/project.rs`
- Modify: `src/lib.rs`, `src/storage/mod.rs`, `Cargo.toml` (add `sha2`)
- Test: `tests/storage_contract.rs`

**Interfaces:**
- Consumes: `Storage::write_txn`, `append_event` from Task 3.
- Produces: `command::CommandContext { principal_kind, principal_id, command_id, command_kind, command_schema_ver, request_fingerprint }`; `command::fingerprint(kind: &str, params: &serde_json::Value) -> String`; `project::Project { id, slug, name, created_at }`; `Storage::create_project(ctx: &CommandContext, slug: &str, name: &str) -> Result<Project, StorageError>`; `Storage::list_projects() -> Result<Vec<Project>, StorageError>`; `pub(super) classify(...)` and `pub(super) record_command(...)` for reuse by later tasks.

- [ ] **Step 1: Write the failing tests**

Append to `tests/storage_contract.rs`:

```rust
use shadows::command::{fingerprint, CommandContext};

fn ctx(command_id: &str, params: &serde_json::Value) -> CommandContext {
    ctx_kind(command_id, "project.create", params)
}

fn ctx_kind(command_id: &str, kind: &str, params: &serde_json::Value) -> CommandContext {
    CommandContext {
        principal_kind: "User".into(),
        principal_id: "local".into(),
        command_id: command_id.into(),
        command_kind: kind.into(),
        command_schema_ver: 1,
        request_fingerprint: fingerprint(kind, params),
    }
}

/// Spec section 5.2 and cross-cutting rule 6: an external mutation writes its
/// CommandRecord in the same transaction. Replaying the same command id with
/// the same request returns the stored outcome and creates nothing new.
#[tokio::test]
async fn replaying_an_identical_command_returns_the_stored_outcome() {
    let tmp = tempfile::tempdir().unwrap();
    let storage = Storage::open(&tmp.path().join("s.sqlite3")).await.unwrap();
    let params = serde_json::json!({ "slug": "demo", "name": "Demo" });

    let first = storage.create_project(&ctx("cmd-1", &params), "demo", "Demo").await.unwrap();
    let second = storage.create_project(&ctx("cmd-1", &params), "demo", "Demo").await.unwrap();

    assert_eq!(first.id, second.id, "replay must return the same entity");

    let projects: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM project")
        .fetch_one(storage.reader()).await.unwrap();
    assert_eq!(projects, 1, "replay must not create a second project");

    let events: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM durable_event")
        .fetch_one(storage.reader()).await.unwrap();
    assert_eq!(events, 1, "replay must not append a second event");
}

/// Replay requires fingerprint equality. The same command id with a different
/// request is CommandConflict and mutates nothing. This is the arm a future
/// contributor most wants to weaken; it stays refused.
#[tokio::test]
async fn the_same_command_id_with_a_different_request_is_a_conflict() {
    let tmp = tempfile::tempdir().unwrap();
    let storage = Storage::open(&tmp.path().join("s.sqlite3")).await.unwrap();

    let first_params = serde_json::json!({ "slug": "demo", "name": "Demo" });
    storage.create_project(&ctx("cmd-1", &first_params), "demo", "Demo").await.unwrap();

    let other_params = serde_json::json!({ "slug": "other", "name": "Other" });
    let err = storage
        .create_project(&ctx("cmd-1", &other_params), "other", "Other")
        .await
        .expect_err("a reused command id with a different request must be refused");
    assert!(matches!(err, shadows::storage::StorageError::CommandConflict));

    let projects: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM project")
        .fetch_one(storage.reader()).await.unwrap();
    assert_eq!(projects, 1, "a conflict must mutate nothing");
}

/// Key order carries no meaning, so reordering JSON keys must not turn a
/// replay into a conflict. A different command kind must not collide.
#[test]
fn the_fingerprint_ignores_key_order_but_not_command_kind() {
    let a = serde_json::json!({ "slug": "demo", "name": "Demo" });
    let b = serde_json::json!({ "name": "Demo", "slug": "demo" });
    assert_eq!(fingerprint("project.create", &a), fingerprint("project.create", &b));
    assert_ne!(fingerprint("project.create", &a), fingerprint("thread.create", &a));
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --test storage_contract`
Expected: FAIL — `shadows::command` and `create_project` do not exist.

- [ ] **Step 3: Write `src/command/mod.rs`**

```rust
use sha2::{Digest, Sha256};

/// Spec section 5.2, external write origin: everything needed to decide whether
/// a submission is new, a replay, or a conflict.
#[derive(Debug, Clone)]
pub struct CommandContext {
    pub principal_kind: String,
    pub principal_id: String,
    pub command_id: String,
    pub command_kind: String,
    pub command_schema_ver: i64,
    pub request_fingerprint: String,
}

/// Canonicalise before hashing, so that key order — which carries no meaning —
/// cannot turn a replay into a conflict. The command kind is mixed in so the
/// same params under a different command are not interchangeable.
pub fn fingerprint(command_kind: &str, params: &serde_json::Value) -> String {
    let mut hasher = Sha256::new();
    hasher.update(command_kind.as_bytes());
    hasher.update(b"\0");
    hasher.update(canonical(params).as_bytes());
    format!("{:x}", hasher.finalize())
}

fn canonical(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::Object(map) => {
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            let inner: Vec<String> =
                keys.iter().map(|k| format!("{}:{}", k, canonical(&map[*k]))).collect();
            format!("{{{}}}", inner.join(","))
        }
        serde_json::Value::Array(items) => {
            let inner: Vec<String> = items.iter().map(canonical).collect();
            format!("[{}]", inner.join(","))
        }
        other => other.to_string(),
    }
}
```

Add `sha2 = "0.10"` to `[dependencies]`.

- [ ] **Step 4: Write `src/project/mod.rs`**

```rust
/// Spec section 11.1 requires selecting a local directory without a path
/// becoming the project's identity. The id is a UUID and the slug is the
/// stable human key; the directory is configuration, not identity.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Project {
    pub id: String,
    pub slug: String,
    pub name: String,
    pub created_at: String,
}
```

- [ ] **Step 5: Write `src/storage/sqlite/project.rs`**

```rust
use sqlx::SqliteConnection;

use crate::command::CommandContext;
use crate::events::{Actor, DurableEvent};
use crate::project::Project;
use super::{events::append_event, now, Storage, StorageError};

/// `Some(outcome_ref)` when this exact command was already recorded, `None`
/// when it is new, `Err(CommandConflict)` when the id was reused with a
/// different request. Spec section 5.2.
pub(super) async fn classify(
    conn: &mut SqliteConnection,
    ctx: &CommandContext,
    scope_kind: &str,
    scope_key: &str,
) -> Result<Option<String>, StorageError> {
    let existing: Option<(String, String, Option<String>)> = sqlx::query_as(
        "SELECT command_kind, request_fingerprint, outcome_ref FROM command_record
          WHERE principal_kind = ? AND principal_id = ?
            AND command_scope_kind = ? AND command_scope_key = ?
            AND command_id = ?",
    )
    .bind(&ctx.principal_kind).bind(&ctx.principal_id)
    .bind(scope_kind).bind(scope_key).bind(&ctx.command_id)
    .fetch_optional(&mut *conn)
    .await?;

    match existing {
        None => Ok(None),
        Some((kind, fp, outcome_ref)) => {
            if kind == ctx.command_kind && fp == ctx.request_fingerprint {
                Ok(Some(outcome_ref.ok_or(StorageError::NotFound("outcome_ref"))?))
            } else {
                Err(StorageError::CommandConflict)
            }
        }
    }
}

pub(super) async fn record_command(
    conn: &mut SqliteConnection,
    ctx: &CommandContext,
    scope_kind: &str,
    scope_key: &str,
    entity_kind: &str,
    outcome_ref: &str,
    ts: &str,
) -> Result<(), StorageError> {
    sqlx::query(
        "INSERT INTO command_record
           (principal_kind, principal_id, command_scope_kind, command_scope_key,
            command_id, command_kind, command_schema_ver, request_fingerprint,
            outcome_kind, entity_kind, outcome_ref, recorded_at)
         VALUES (?,?,?,?,?,?,?,?,'Entity',?,?,?)",
    )
    .bind(&ctx.principal_kind).bind(&ctx.principal_id)
    .bind(scope_kind).bind(scope_key)
    .bind(&ctx.command_id).bind(&ctx.command_kind).bind(ctx.command_schema_ver)
    .bind(&ctx.request_fingerprint)
    .bind(entity_kind).bind(outcome_ref).bind(ts)
    .execute(&mut *conn)
    .await?;
    Ok(())
}

impl Storage {
    pub async fn create_project(
        &self,
        ctx: &CommandContext,
        slug: &str,
        name: &str,
    ) -> Result<Project, StorageError> {
        let (ctx, slug, name, ts) = (ctx.clone(), slug.to_string(), name.to_string(), now());
        self.write_txn(move |conn| {
            Box::pin(async move {
                if let Some(existing_id) = classify(conn, &ctx, "Global", "").await? {
                    return load_project(conn, &existing_id).await;
                }
                let id = uuid::Uuid::new_v4().to_string();
                sqlx::query("INSERT INTO project (id, slug, name, created_at) VALUES (?,?,?,?)")
                    .bind(&id).bind(&slug).bind(&name).bind(&ts)
                    .execute(&mut *conn).await?;

                append_event(
                    conn,
                    &DurableEvent::new("ProjectCreated", Actor::user(&ctx.principal_id))
                        .with_project(&id)
                        .with_payload(serde_json::json!({ "slug": slug, "name": name })),
                    &ts,
                ).await?;

                record_command(conn, &ctx, "Global", "", "Project", &id, &ts).await?;
                load_project(conn, &id).await
            })
        })
        .await
    }

    pub async fn list_projects(&self) -> Result<Vec<Project>, StorageError> {
        let rows: Vec<(String, String, String, String)> = sqlx::query_as(
            "SELECT id, slug, name, created_at FROM project ORDER BY created_at, id",
        )
        .fetch_all(self.reader())
        .await?;
        Ok(rows.into_iter()
            .map(|(id, slug, name, created_at)| Project { id, slug, name, created_at })
            .collect())
    }
}

async fn load_project(conn: &mut SqliteConnection, id: &str) -> Result<Project, StorageError> {
    let row: (String, String, String, String) =
        sqlx::query_as("SELECT id, slug, name, created_at FROM project WHERE id = ?")
            .bind(id)
            .fetch_optional(&mut *conn)
            .await?
            .ok_or(StorageError::NotFound("project"))?;
    Ok(Project { id: row.0, slug: row.1, name: row.2, created_at: row.3 })
}
```

Move `now()` out of `runtime.rs` into `src/storage/sqlite/mod.rs` as `pub(super) fn now() -> String` so every module shares one implementation.

- [ ] **Step 6: Run tests to verify they pass**

Run: `cargo test --test storage_contract`
Expected: PASS, all five tests.

- [ ] **Step 7: Commit**

```bash
git add src/command src/project src/storage src/lib.rs Cargo.toml tests/storage_contract.rs
git commit -m "feat(project): create a local project under external command idempotency"
```

---

## Task 6: PlanningThread and ThreadEntry with transactional ordinal allocation

**Files:**
- Create: `src/thread/mod.rs`, `src/storage/sqlite/thread.rs`
- Modify: `src/lib.rs`, `src/storage/mod.rs`
- Test: `tests/storage_contract.rs`

**Interfaces:**
- Consumes: `CommandContext` from Task 5; `classify` and `record_command` from Task 5.
- Produces: `thread::{PlanningThread, ThreadEntry}`; `Storage::create_planning_thread(ctx, project_id, title) -> Result<PlanningThread, StorageError>`; `Storage::append_thread_entry(thread_id, kind, author_kind, author_id, body) -> Result<ThreadEntry, StorageError>`; `Storage::list_thread_entries(thread_id) -> Result<Vec<ThreadEntry>, StorageError>`; `Storage::list_threads_for_project(project_id) -> Result<Vec<PlanningThread>, StorageError>`.

- [ ] **Step 1: Write the failing tests**

Append to `tests/storage_contract.rs`:

```rust
/// Spec section 6.5: ordinals are allocated by UPDATE ... RETURNING in the same
/// transaction, never MAX(ordinal)+1. Under concurrency the result must be a
/// contiguous, gapless, duplicate-free run.
#[tokio::test]
async fn concurrent_entry_appends_allocate_contiguous_unique_ordinals() {
    let tmp = tempfile::tempdir().unwrap();
    let storage = std::sync::Arc::new(Storage::open(&tmp.path().join("s.sqlite3")).await.unwrap());
    let params = serde_json::json!({ "slug": "demo", "name": "Demo" });
    let project = storage.create_project(&ctx("cmd-p", &params), "demo", "Demo").await.unwrap();
    let thread = storage
        .create_planning_thread(&ctx_kind("cmd-t", "thread.create", &params), &project.id, "T")
        .await.unwrap();

    let mut handles = Vec::new();
    for w in 0..8 {
        let storage = storage.clone();
        let thread_id = thread.id.clone();
        handles.push(tokio::spawn(async move {
            for i in 0..25 {
                storage
                    .append_thread_entry(&thread_id, "UserMessage", "User", "local",
                                         &format!("w{w}-i{i}"))
                    .await.unwrap();
            }
        }));
    }
    for h in handles { h.await.unwrap(); }

    let ordinals: Vec<i64> = sqlx::query_scalar(
        "SELECT ordinal FROM thread_entry WHERE thread_id = ? ORDER BY ordinal",
    )
    .bind(&thread.id)
    .fetch_all(storage.reader()).await.unwrap();

    assert_eq!(ordinals.len(), 200);
    assert_eq!(ordinals, (1..=200).collect::<Vec<i64>>(),
               "ordinals must be contiguous and unique");

    let next: i64 = sqlx::query_scalar("SELECT next_entry_ordinal FROM planning_thread WHERE id = ?")
        .bind(&thread.id)
        .fetch_one(storage.reader()).await.unwrap();
    assert_eq!(next, 201);
}

/// Entries are read back in ordinal order, never in insertion order.
/// Cross-cutting rule 4.
#[tokio::test]
async fn entries_are_read_in_ordinal_order() {
    let tmp = tempfile::tempdir().unwrap();
    let storage = Storage::open(&tmp.path().join("s.sqlite3")).await.unwrap();
    let params = serde_json::json!({ "slug": "demo", "name": "Demo" });
    let project = storage.create_project(&ctx("cmd-p", &params), "demo", "Demo").await.unwrap();
    let thread = storage
        .create_planning_thread(&ctx_kind("cmd-t", "thread.create", &params), &project.id, "T")
        .await.unwrap();

    for body in ["first", "second", "third"] {
        storage.append_thread_entry(&thread.id, "UserMessage", "User", "local", body)
            .await.unwrap();
    }
    let entries = storage.list_thread_entries(&thread.id).await.unwrap();
    assert_eq!(entries.iter().map(|e| e.body.as_str()).collect::<Vec<_>>(),
               vec!["first", "second", "third"]);
    assert_eq!(entries.iter().map(|e| e.ordinal).collect::<Vec<_>>(), vec![1, 2, 3]);
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --test storage_contract`
Expected: FAIL — `create_planning_thread` and `append_thread_entry` do not exist.

- [ ] **Step 3: Write `src/thread/mod.rs`**

```rust
#[derive(Debug, Clone, serde::Serialize)]
pub struct PlanningThread {
    pub id: String,
    pub project_id: String,
    pub title: String,
    pub status: String,
    pub created_at: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ThreadEntry {
    pub id: String,
    pub thread_id: String,
    pub ordinal: i64,
    pub kind: String,
    pub author_kind: String,
    pub author_id: String,
    pub body: String,
    pub created_at: String,
}
```

- [ ] **Step 4: Write `src/storage/sqlite/thread.rs`**

```rust
use sqlx::SqliteConnection;

use crate::command::CommandContext;
use crate::events::{Actor, DurableEvent};
use crate::thread::{PlanningThread, ThreadEntry};
use super::project::{classify, record_command};
use super::{events::append_event, now, Storage, StorageError};

impl Storage {
    pub async fn create_planning_thread(
        &self,
        ctx: &CommandContext,
        project_id: &str,
        title: &str,
    ) -> Result<PlanningThread, StorageError> {
        let (ctx, project_id, title, ts) =
            (ctx.clone(), project_id.to_string(), title.to_string(), now());
        self.write_txn(move |conn| {
            Box::pin(async move {
                if let Some(id) = classify(conn, &ctx, "Project", &project_id).await? {
                    return load_thread(conn, &id).await;
                }
                let id = uuid::Uuid::new_v4().to_string();
                sqlx::query(
                    "INSERT INTO planning_thread
                       (id, project_id, title, status, next_entry_ordinal, created_at)
                     VALUES (?,?,?, 'Open', 1, ?)",
                )
                .bind(&id).bind(&project_id).bind(&title).bind(&ts)
                .execute(&mut *conn).await?;

                append_event(
                    conn,
                    &DurableEvent::new("PlanningThreadCreated", Actor::user(&ctx.principal_id))
                        .with_project(&project_id)
                        .with_thread(&id)
                        .with_payload(serde_json::json!({ "title": title })),
                    &ts,
                ).await?;

                record_command(conn, &ctx, "Project", &project_id, "PlanningThread", &id, &ts).await?;
                load_thread(conn, &id).await
            })
        })
        .await
    }

    /// Internal write: no CommandRecord. Entries appended while a turn streams
    /// are produced by the daemon, not commanded by a client. Spec section 5.2.
    pub async fn append_thread_entry(
        &self,
        thread_id: &str,
        kind: &str,
        author_kind: &str,
        author_id: &str,
        body: &str,
    ) -> Result<ThreadEntry, StorageError> {
        let (thread_id, kind, author_kind, author_id, body, ts) = (
            thread_id.to_string(), kind.to_string(), author_kind.to_string(),
            author_id.to_string(), body.to_string(), now(),
        );
        self.write_txn(move |conn| {
            Box::pin(async move {
                // Spec section 6.5: allocate inside this transaction. Never MAX+1.
                let ordinal: i64 = sqlx::query_scalar(
                    "UPDATE planning_thread
                        SET next_entry_ordinal = next_entry_ordinal + 1
                      WHERE id = ?
                  RETURNING next_entry_ordinal - 1",
                )
                .bind(&thread_id)
                .fetch_optional(&mut *conn)
                .await?
                .ok_or(StorageError::NotFound("planning_thread"))?;

                let id = uuid::Uuid::new_v4().to_string();
                sqlx::query(
                    "INSERT INTO thread_entry
                       (id, thread_id, ordinal, kind, author_kind, author_id, body, created_at)
                     VALUES (?,?,?,?,?,?,?,?)",
                )
                .bind(&id).bind(&thread_id).bind(ordinal).bind(&kind)
                .bind(&author_kind).bind(&author_id).bind(&body).bind(&ts)
                .execute(&mut *conn).await?;

                append_event(
                    conn,
                    &DurableEvent::new(
                        "ThreadEntryAppended",
                        Actor { kind: author_kind.clone(), id: author_id.clone() },
                    )
                    .with_thread(&thread_id)
                    .with_payload(serde_json::json!({ "ordinal": ordinal, "kind": kind })),
                    &ts,
                ).await?;

                Ok(ThreadEntry {
                    id, thread_id, ordinal, kind, author_kind, author_id, body, created_at: ts,
                })
            })
        })
        .await
    }

    pub async fn list_thread_entries(
        &self,
        thread_id: &str,
    ) -> Result<Vec<ThreadEntry>, StorageError> {
        let rows: Vec<(String, String, i64, String, String, String, String, String)> =
            sqlx::query_as(
                "SELECT id, thread_id, ordinal, kind, author_kind, author_id, body, created_at
                   FROM thread_entry WHERE thread_id = ? ORDER BY ordinal",
            )
            .bind(thread_id)
            .fetch_all(self.reader())
            .await?;
        Ok(rows.into_iter().map(|r| ThreadEntry {
            id: r.0, thread_id: r.1, ordinal: r.2, kind: r.3,
            author_kind: r.4, author_id: r.5, body: r.6, created_at: r.7,
        }).collect())
    }

    pub async fn list_threads_for_project(
        &self,
        project_id: &str,
    ) -> Result<Vec<PlanningThread>, StorageError> {
        let rows: Vec<(String, String, String, String, String)> = sqlx::query_as(
            "SELECT id, project_id, title, status, created_at
               FROM planning_thread WHERE project_id = ? ORDER BY created_at, id",
        )
        .bind(project_id)
        .fetch_all(self.reader())
        .await?;
        Ok(rows.into_iter().map(|r| PlanningThread {
            id: r.0, project_id: r.1, title: r.2, status: r.3, created_at: r.4,
        }).collect())
    }
}

async fn load_thread(
    conn: &mut SqliteConnection,
    id: &str,
) -> Result<PlanningThread, StorageError> {
    let r: (String, String, String, String, String) = sqlx::query_as(
        "SELECT id, project_id, title, status, created_at FROM planning_thread WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(&mut *conn)
    .await?
    .ok_or(StorageError::NotFound("planning_thread"))?;
    Ok(PlanningThread { id: r.0, project_id: r.1, title: r.2, status: r.3, created_at: r.4 })
}
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test --test storage_contract`
Expected: PASS, all seven tests.

- [ ] **Step 6: Commit**

```bash
git add src/thread src/storage src/lib.rs tests/storage_contract.rs
git commit -m "feat(thread): planning threads and entries with transactional ordinal allocation"
```

---

## Task 7: The managed process primitive and process-tree containment

**Files:**
- Create: `src/process/mod.rs`, `src/process/containment_windows.rs`, `src/process/containment_unix.rs`
- Create: `src/bin/tree_probe.rs` (test-support binary; see note below)
- Modify: `src/lib.rs`, `Cargo.toml`
- Test: `tests/containment.rs`

**Interfaces:**
- Consumes: nothing from earlier tasks. `process/` knows nothing about Role, Claude, threads, or operations — spec §1.5.
- Produces: `process::ProcessSpec { executable: PathBuf, args: Vec<String>, cwd: PathBuf, env: Vec<(String, String)>, capture_stdout: bool }`; `process::ProcessHandle` with `stdout_lines(&mut self) -> Option<Lines<BufReader<ChildStdout>>>`, `wait(&mut self) -> io::Result<ExitStatus>`, `terminate_tree(&mut self) -> io::Result<()>`, `id(&self) -> Option<u32>`; `process::spawn(spec: ProcessSpec) -> io::Result<ProcessHandle>`.

**Why a test-support binary.** The containment probe in spec §3.7 Layer 4 requires a real `daemon -> child -> grandchild` hierarchy; a direct-child-only test is explicitly insufficient. `tree_probe` spawns a grandchild and then sleeps, so the test has a genuine three-level tree to kill. It is declared as a `[[bin]]` so integration tests can find it through `CARGO_BIN_EXE_tree_probe`. It contains no product logic and ships as part of the test apparatus.

- [ ] **Step 1: Write the failing test**

`tests/containment.rs`:

```rust
use std::time::Duration;

use shadows::process::{spawn, ProcessSpec};

#[cfg(windows)]
fn is_alive(pid: u32) -> bool {
    use std::process::Command;
    let out = Command::new("powershell")
        .args(["-NoProfile", "-Command",
               &format!("if (Get-Process -Id {pid} -ErrorAction SilentlyContinue) {{ 'yes' }} else {{ 'no' }}")])
        .output()
        .expect("powershell should run");
    String::from_utf8_lossy(&out.stdout).trim() == "yes"
}

#[cfg(unix)]
fn is_alive(pid: u32) -> bool {
    std::path::Path::new(&format!("/proc/{pid}")).exists()
}

/// Spec §1.5 and §3.7 Layer 4. A managed child and every managed descendant
/// must not survive termination. A direct-child-only assertion is insufficient,
/// so the probe builds a real grandchild and checks that one too.
#[tokio::test]
async fn terminating_a_managed_tree_kills_the_grandchild_too() {
    let probe = env!("CARGO_BIN_EXE_tree_probe");
    let mut handle = spawn(ProcessSpec {
        executable: probe.into(),
        args: vec!["--spawn-grandchild".into()],
        cwd: std::env::temp_dir(),
        env: Vec::new(),
        capture_stdout: true,
    })
    .expect("spawn should succeed");

    // tree_probe prints its grandchild's pid on its first stdout line.
    let mut lines = handle.stdout_lines().expect("stdout was captured");
    let first = tokio::time::timeout(Duration::from_secs(10), async {
        use tokio::io::AsyncBufReadExt;
        lines.next_line().await
    })
    .await
    .expect("probe should report within 10s")
    .unwrap()
    .expect("probe should print a line");

    let grandchild: u32 = first
        .trim()
        .strip_prefix("grandchild=")
        .expect("probe prints grandchild=<pid>")
        .parse()
        .unwrap();
    let child = handle.id().expect("child has a pid");

    assert!(is_alive(child), "child should be alive before termination");
    assert!(is_alive(grandchild), "grandchild should be alive before termination");

    handle.terminate_tree().expect("termination should succeed");

    // Give the OS a bounded moment to reap, then assert both are gone.
    for _ in 0..50 {
        if !is_alive(child) && !is_alive(grandchild) {
            return;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    panic!("child or grandchild survived termination: child={child} grandchild={grandchild}");
}

/// Spec §1.5: the child's stdin is closed. An open stdin that never receives
/// data costs a fixed stall on every turn — measured at three seconds against
/// the real harness.
#[tokio::test]
async fn a_spawned_child_has_no_inherited_stdin() {
    let probe = env!("CARGO_BIN_EXE_tree_probe");
    let mut handle = spawn(ProcessSpec {
        executable: probe.into(),
        args: vec!["--read-stdin".into()],
        cwd: std::env::temp_dir(),
        env: Vec::new(),
        capture_stdout: true,
    })
    .unwrap();

    let status = tokio::time::timeout(Duration::from_secs(5), handle.wait())
        .await
        .expect("a child with closed stdin must see EOF immediately, not block")
        .unwrap();
    assert!(status.success());
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test containment`
Expected: FAIL — `shadows::process` does not exist.

- [ ] **Step 3: Write `src/bin/tree_probe.rs`**

```rust
//! Test-support binary. Not product code. It exists so the containment test
//! has a real daemon -> child -> grandchild hierarchy to terminate.

use std::io::Read;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.iter().any(|a| a == "--read-stdin") {
        let mut buf = String::new();
        // With stdin closed this returns Ok(0) immediately. With an inherited
        // console stdin it blocks, and the test times out.
        let _ = std::io::stdin().read_to_string(&mut buf);
        return;
    }

    if args.iter().any(|a| a == "--spawn-grandchild") {
        let me = std::env::current_exe().expect("current exe");
        let grandchild = std::process::Command::new(me)
            .arg("--sleep")
            .spawn()
            .expect("grandchild should spawn");
        println!("grandchild={}", grandchild.id());
        use std::io::Write;
        std::io::stdout().flush().unwrap();
    }

    // Both the child and the grandchild end up here and sleep until killed.
    std::thread::sleep(std::time::Duration::from_secs(600));
}
```

Declare it in `Cargo.toml`:

```toml
[[bin]]
name = "tree_probe"
path = "src/bin/tree_probe.rs"
```

- [ ] **Step 4: Write `src/process/mod.rs`**

```rust
use std::io;
use std::path::PathBuf;
use std::process::Stdio;

use tokio::io::{BufReader, Lines};
use tokio::process::{Child, ChildStdout, Command};

#[cfg(windows)]
mod containment_windows;
#[cfg(windows)]
use containment_windows as containment;

#[cfg(unix)]
mod containment_unix;
#[cfg(unix)]
use containment_unix as containment;

/// OS-level intent and nothing else. Spec §1.5: `process/` knows nothing about
/// Role, Claude, Codex, planning, workflows, or verification.
#[derive(Debug, Clone)]
pub struct ProcessSpec {
    pub executable: PathBuf,
    pub args: Vec<String>,
    pub cwd: PathBuf,
    /// Built explicitly. The daemon's own environment is never mutated for a
    /// child, and clearing wholesale is not the same as isolating: on Windows a
    /// child that loses SystemRoot, SystemDrive, ComSpec or PATHEXT fails in
    /// ways that never appear on Linux.
    pub env: Vec<(String, String)>,
    pub capture_stdout: bool,
}

pub struct ProcessHandle {
    child: Child,
    containment: containment::Containment,
    stdout: Option<Lines<BufReader<ChildStdout>>>,
}

impl ProcessHandle {
    pub fn id(&self) -> Option<u32> {
        self.child.id()
    }

    pub fn stdout_lines(&mut self) -> Option<&mut Lines<BufReader<ChildStdout>>> {
        self.stdout.as_mut()
    }

    pub async fn wait(&mut self) -> io::Result<std::process::ExitStatus> {
        self.child.wait().await
    }

    /// Terminates the whole managed tree through the containment handle that
    /// owns it. Spec §8.3: never by signalling a PID read from the database,
    /// because the operating system reuses PIDs.
    pub fn terminate_tree(&mut self) -> io::Result<()> {
        self.containment.terminate()
    }
}

/// Windows environment variables a child needs even under an otherwise
/// explicit environment. Losing any of these breaks the child in ways that
/// never reproduce on Linux.
#[cfg(windows)]
const WINDOWS_ESSENTIAL_ENV: &[&str] =
    &["SystemRoot", "SystemDrive", "ComSpec", "PATHEXT", "TEMP", "TMP", "USERPROFILE", "APPDATA"];

pub fn spawn(spec: ProcessSpec) -> io::Result<ProcessHandle> {
    let mut cmd = Command::new(&spec.executable);
    cmd.args(&spec.args)
        .current_dir(&spec.cwd)
        .env_clear()
        // Spec §1.5: stdin is closed unless the harness contract requires
        // streaming input.
        .stdin(Stdio::null())
        .stdout(if spec.capture_stdout { Stdio::piped() } else { Stdio::null() })
        .stderr(Stdio::piped());

    #[cfg(windows)]
    for key in WINDOWS_ESSENTIAL_ENV {
        if let Ok(value) = std::env::var(key) {
            cmd.env(key, value);
        }
    }
    for (k, v) in &spec.env {
        cmd.env(k, v);
    }

    containment::configure(&mut cmd);

    let mut child = cmd.spawn()?;
    let pid = child.id().ok_or_else(|| {
        io::Error::other("child exited before a pid could be observed")
    })?;
    let containment = containment::attach(pid)?;

    let stdout = child
        .stdout
        .take()
        .map(|out| {
            use tokio::io::AsyncBufReadExt;
            BufReader::new(out).lines()
        });

    Ok(ProcessHandle { child, containment, stdout })
}
```

- [ ] **Step 5: Write `src/process/containment_windows.rs`**

```rust
use std::io;

use windows::Win32::Foundation::{CloseHandle, HANDLE};
use windows::Win32::System::JobObjects::{
    AssignProcessToJobObject, CreateJobObjectW, JobObjectExtendedLimitInformation,
    SetInformationJobObject, TerminateJobObject, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
    JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
};
use windows::Win32::System::Threading::{
    OpenProcess, PROCESS_SET_QUOTA, PROCESS_TERMINATE,
};

pub struct Containment {
    job: HANDLE,
}

// The handle is owned exclusively by this struct for its lifetime.
unsafe impl Send for Containment {}
unsafe impl Sync for Containment {}

pub fn configure(_cmd: &mut tokio::process::Command) {
    // Nothing to set before spawn on Windows; the job is assigned after.
}

/// Spec §1.5: Job Object kill-on-owner-close semantics, with breakaway
/// prevented. `JOB_OBJECT_LIMIT_BREAKAWAY_OK` and
/// `JOB_OBJECT_LIMIT_SILENT_BREAKAWAY_OK` are deliberately not set, so a
/// descendant cannot leave the job.
pub fn attach(pid: u32) -> io::Result<Containment> {
    unsafe {
        let job = CreateJobObjectW(None, None).map_err(io::Error::other)?;

        let mut info = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
        info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        SetInformationJobObject(
            job,
            JobObjectExtendedLimitInformation,
            &info as *const _ as *const core::ffi::c_void,
            std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
        )
        .map_err(io::Error::other)?;

        let process = OpenProcess(PROCESS_SET_QUOTA | PROCESS_TERMINATE, false, pid)
            .map_err(io::Error::other)?;
        let assign = AssignProcessToJobObject(job, process);
        let _ = CloseHandle(process);
        assign.map_err(io::Error::other)?;

        Ok(Containment { job })
    }
}

impl Containment {
    pub fn terminate(&mut self) -> io::Result<()> {
        unsafe { TerminateJobObject(self.job, 1).map_err(io::Error::other) }
    }
}

impl Drop for Containment {
    fn drop(&mut self) {
        // Closing the last handle kills the job, which is the guarantee the
        // spec asks for: the tree does not outlive its owning runtime.
        unsafe {
            let _ = CloseHandle(self.job);
        }
    }
}
```

Add to `Cargo.toml`:

```toml
[target.'cfg(windows)'.dependencies]
windows = { version = "0.58", features = [
    "Win32_Foundation",
    "Win32_System_JobObjects",
    "Win32_System_Threading",
] }
```

- [ ] **Step 6: Write `src/process/containment_unix.rs`**

```rust
use std::io;

pub struct Containment {
    pgid: i32,
}

/// Spec §1.5: a process group alone is NOT accepted as proof that descendants
/// die when the daemon crashes. It is used here for the deliberate-termination
/// path only. The parent-death half of the contract is not implemented in
/// Milestone 0, and `tests/containment.rs` is expected to fail on Linux until
/// it is — which is the honest state, not a passing test that proves nothing.
pub fn configure(cmd: &mut tokio::process::Command) {
    use std::os::unix::process::CommandExt;
    unsafe {
        cmd.pre_exec(|| {
            // New process group, so the whole group can be signalled at once.
            if libc::setsid() == -1 {
                return Err(io::Error::last_os_error());
            }
            Ok(())
        });
    }
}

pub fn attach(pid: u32) -> io::Result<Containment> {
    Ok(Containment { pgid: pid as i32 })
}

impl Containment {
    pub fn terminate(&mut self) -> io::Result<()> {
        unsafe {
            if libc::killpg(self.pgid, libc::SIGKILL) == -1 {
                let err = io::Error::last_os_error();
                if err.raw_os_error() != Some(libc::ESRCH) {
                    return Err(err);
                }
            }
        }
        Ok(())
    }
}
```

Add to `Cargo.toml`:

```toml
[target.'cfg(unix)'.dependencies]
libc = "0.2"
```

- [ ] **Step 7: Run tests to verify they pass on Windows**

Run: `cargo test --test containment`
Expected: PASS on Windows.

On Linux, expect `terminating_a_managed_tree_kills_the_grandchild_too` to pass for deliberate termination and the parent-death half to be unimplemented. Do not gate the test off to make the suite green. Record the gap; spec §11.3 requires containment on both platforms before the runtime is called cross-platform, and Milestone 0 only claims Windows.

- [ ] **Step 8: Commit**

```bash
git add src/process src/bin Cargo.toml src/lib.rs tests/containment.rs
git commit -m "feat(process): managed spawn with Job Object tree containment on Windows"
```

---

## Task 8: The Claude harness and its stream contract

**Files:**
- Create: `src/agent/mod.rs`, `src/agent/claude.rs`
- Create: `tests/harness_stream.rs`, `tests/fixtures/claude_turn.jsonl`
- Modify: `src/lib.rs`
- Test: `tests/harness_stream.rs`

**Interfaces:**
- Consumes: `process::{ProcessSpec, ProcessHandle, spawn}` from Task 7.
- Produces: `agent::AgentInvocation { operation_id, role, harness_path, harness_version, model, prompt, cwd, resume_session_id }`; `agent::StreamItem` enum; `agent::AgentHarness` trait with `fn to_process_spec(&self, inv: &AgentInvocation) -> ProcessSpec` and `fn classify(&self, line: &str) -> StreamItem`; `agent::claude::ClaudeHarness`.

**The contract is measured, not assumed.** Flags and line shapes come from `docs/evidence/harness/SERVE_STREAM_SPIKE.md`, captured against Claude Code 2.1.278.

- [ ] **Step 1: Capture the fixture**

Run a real turn once and save its output as the test fixture:

```bash
mkdir -p tests/fixtures
claude --print --output-format stream-json --verbose --include-partial-messages \
       --permission-prompts none --safe-mode --model sonnet \
       --session-id 00000000-0000-4000-8000-000000000001 \
       "Reply with exactly: fixture" < /dev/null > tests/fixtures/claude_turn.jsonl
```

Commit the fixture. A stored real turn is what stops the classifier being tested against a hand-written idea of the format.

- [ ] **Step 2: Write the failing test**

`tests/harness_stream.rs`:

```rust
use shadows::agent::{claude::ClaudeHarness, AgentHarness, AgentInvocation, StreamItem};

fn harness() -> ClaudeHarness {
    ClaudeHarness::new("claude".into(), "2.1.278".into())
}

/// Spec §1.4 and the harness evidence report: the executable comes from
/// configuration, and the measured flags are part of the contract.
#[test]
fn the_invocation_uses_the_measured_flags_and_the_configured_executable() {
    let spec = harness().to_process_spec(&AgentInvocation {
        operation_id: "op-1".into(),
        role: "Planner".into(),
        model: "sonnet".into(),
        prompt: "hello".into(),
        cwd: std::env::temp_dir(),
        resume_session_id: None,
        session_id: "00000000-0000-4000-8000-000000000001".into(),
    });

    assert_eq!(spec.executable, std::path::PathBuf::from("claude"));
    let args = spec.args.join(" ");
    for required in [
        "--print",
        "--output-format stream-json",
        "--verbose",
        "--include-partial-messages",
        "--permission-prompts none",
        "--session-id 00000000-0000-4000-8000-000000000001",
    ] {
        assert!(args.contains(required), "missing {required} in: {args}");
    }
    assert!(!args.contains("--resume"), "a first turn must not resume");
    assert!(spec.capture_stdout);
}

/// A resumed turn carries --resume and not --session-id. `--resume` gives
/// continuity across separate OS processes and does not re-emit history.
#[test]
fn a_resumed_turn_uses_resume_instead_of_session_id() {
    let spec = harness().to_process_spec(&AgentInvocation {
        operation_id: "op-2".into(),
        role: "Planner".into(),
        model: "sonnet".into(),
        prompt: "again".into(),
        cwd: std::env::temp_dir(),
        resume_session_id: Some("00000000-0000-4000-8000-000000000001".into()),
        session_id: "00000000-0000-4000-8000-000000000001".into(),
    });
    let args = spec.args.join(" ");
    assert!(args.contains("--resume 00000000-0000-4000-8000-000000000001"));
    assert!(!args.contains("--session-id"));
}

/// The durable/transient split, against a real captured turn. A `stream_event`
/// never carries information the following `assistant` line does not also
/// carry, so deltas are forwarded and never stored.
#[test]
fn a_real_turn_classifies_into_transient_durable_and_terminal() {
    let raw = std::fs::read_to_string("tests/fixtures/claude_turn.jsonl")
        .expect("fixture must exist; capture it with the command in Step 1");
    let h = harness();

    let mut deltas = 0;
    let mut durable = 0;
    let mut terminal = 0;
    let mut session_id: Option<String> = None;

    for line in raw.lines().filter(|l| !l.trim().is_empty()) {
        match h.classify(line) {
            StreamItem::Delta { .. } => deltas += 1,
            StreamItem::Entry { .. } => durable += 1,
            StreamItem::TurnEnd { .. } => terminal += 1,
            StreamItem::Operational { session, .. } => {
                if session_id.is_none() {
                    session_id = session;
                }
            }
            StreamItem::Unparsed(l) => panic!("classifier failed on a real line: {l}"),
        }
    }

    assert!(durable >= 1, "a real turn produces at least one durable entry");
    assert_eq!(terminal, 1, "exactly one result line, always last");
    assert!(session_id.is_some(), "system/init carries the session id");
    let _ = deltas;
}

/// Turn end is an explicit line, not a heuristic, and it carries a structured
/// verdict the daemon cross-checks against the process exit.
#[test]
fn turn_end_is_the_result_line_and_carries_its_verdict() {
    let raw = std::fs::read_to_string("tests/fixtures/claude_turn.jsonl").unwrap();
    let last = raw.lines().filter(|l| !l.trim().is_empty()).next_back().unwrap();
    match harness().classify(last) {
        StreamItem::TurnEnd { subtype, stop_reason } => {
            assert_eq!(subtype, "success");
            assert_eq!(stop_reason.as_deref(), Some("end_turn"));
        }
        other => panic!("the last line must be the turn end, got {other:?}"),
    }
}
```

- [ ] **Step 3: Run tests to verify they fail**

Run: `cargo test --test harness_stream`
Expected: FAIL — `shadows::agent` does not exist.

- [ ] **Step 4: Write `src/agent/mod.rs`**

```rust
use std::path::PathBuf;

use crate::process::ProcessSpec;

pub mod claude;

/// Frozen at claim time, not read at spawn time. Spec §8.2: reading any of
/// these later would let a configuration change between claim and spawn alter
/// what the durable record says was run.
#[derive(Debug, Clone)]
pub struct AgentInvocation {
    pub operation_id: String,
    pub role: String,
    pub model: String,
    pub prompt: String,
    pub cwd: PathBuf,
    /// Present on a resumed turn. Continuity belongs to the harness, not to us.
    pub resume_session_id: Option<String>,
    /// Used on a first turn so the session id is ours to record and resume.
    pub session_id: String,
}

/// The four stream classes. Only `Entry` and `TurnEnd` ever reach storage.
#[derive(Debug, Clone)]
pub enum StreamItem {
    /// Transient. Render only, hundreds per turn, never persisted.
    Delta { text: String },
    /// Durable. Complete, final, carries a harness-assigned uuid that becomes
    /// the entry's harness-side identity.
    Entry { uuid: String, role: String, text: String },
    /// Exactly one per turn, always last.
    TurnEnd { subtype: String, stop_reason: Option<String> },
    /// Diagnostics and UI signal: system/*, rate_limit_event. Never conversation.
    Operational { label: String, session: Option<String> },
    Unparsed(String),
}

pub trait AgentHarness {
    fn to_process_spec(&self, invocation: &AgentInvocation) -> ProcessSpec;
    fn classify(&self, line: &str) -> StreamItem;
}
```

- [ ] **Step 5: Write `src/agent/claude.rs`**

```rust
use std::path::PathBuf;

use serde_json::Value;

use crate::process::ProcessSpec;
use super::{AgentHarness, AgentInvocation, StreamItem};

pub struct ClaudeHarness {
    /// Spec §1.4: resolved from configuration, never from PATH.
    executable: PathBuf,
    /// Recorded per Operation. The stream contract belongs to one installation
    /// at one version, and this machine carries more than one.
    pub version: String,
}

impl ClaudeHarness {
    pub fn new(executable: PathBuf, version: String) -> Self {
        Self { executable, version }
    }
}

impl AgentHarness for ClaudeHarness {
    fn to_process_spec(&self, inv: &AgentInvocation) -> ProcessSpec {
        let mut args: Vec<String> = vec![
            "--print".into(),
            "--output-format".into(), "stream-json".into(),
            "--verbose".into(),
            "--include-partial-messages".into(),
            "--model".into(), inv.model.clone(),
            "--permission-prompts".into(), "none".into(),
        ];
        match &inv.resume_session_id {
            Some(id) => { args.push("--resume".into()); args.push(id.clone()); }
            None => { args.push("--session-id".into()); args.push(inv.session_id.clone()); }
        }
        args.push(inv.prompt.clone());

        ProcessSpec {
            executable: self.executable.clone(),
            args,
            cwd: inv.cwd.clone(),
            env: Vec::new(),
            capture_stdout: true,
        }
    }

    fn classify(&self, line: &str) -> StreamItem {
        let Ok(v) = serde_json::from_str::<Value>(line) else {
            return StreamItem::Unparsed(line.to_string());
        };
        match v["type"].as_str() {
            Some("stream_event") => {
                let e = &v["event"];
                if e["type"] == "content_block_delta" && e["delta"]["type"] == "text_delta" {
                    return StreamItem::Delta {
                        text: e["delta"]["text"].as_str().unwrap_or("").to_string(),
                    };
                }
                StreamItem::Operational {
                    label: e["type"].as_str().unwrap_or("stream_event").to_string(),
                    session: None,
                }
            }
            Some(role @ ("assistant" | "user")) => StreamItem::Entry {
                uuid: v["uuid"].as_str().unwrap_or_default().to_string(),
                role: role.to_string(),
                text: render_content(&v["message"]["content"]),
            },
            Some("result") => StreamItem::TurnEnd {
                subtype: v["subtype"].as_str().unwrap_or("unknown").to_string(),
                stop_reason: v["stop_reason"].as_str().map(str::to_string),
            },
            Some("system") => StreamItem::Operational {
                label: format!("system/{}", v["subtype"].as_str().unwrap_or("?")),
                session: v["session_id"].as_str().map(str::to_string),
            },
            Some(other) => StreamItem::Operational {
                label: other.to_string(),
                session: None,
            },
            None => StreamItem::Unparsed(line.to_string()),
        }
    }
}

/// A durable line's content is a list of blocks. Flatten to the text a
/// ThreadEntry body holds; tool calls and results are named, not inlined.
fn render_content(content: &Value) -> String {
    let Some(blocks) = content.as_array() else { return String::new() };
    blocks
        .iter()
        .map(|b| match b["type"].as_str() {
            Some("text") => b["text"].as_str().unwrap_or("").to_string(),
            Some("thinking") => "[thinking]".to_string(),
            Some("tool_use") => format!("[tool_use: {}]", b["name"].as_str().unwrap_or("?")),
            Some("tool_result") => "[tool_result]".to_string(),
            Some(other) => format!("[{other}]"),
            None => String::new(),
        })
        .collect::<Vec<_>>()
        .join("\n")
}
```

- [ ] **Step 6: Run tests to verify they pass**

Run: `cargo test --test harness_stream`
Expected: PASS, all four tests.

- [ ] **Step 7: Commit**

```bash
git add src/agent src/lib.rs tests/harness_stream.rs tests/fixtures
git commit -m "feat(agent): Claude harness with the measured invocation and stream contract"
```

---

## Task 9: Operation lifecycle — two-phase spawn for a Planner turn

**Files:**
- Create: `src/operation/mod.rs`, `src/storage/sqlite/operation.rs`
- Modify: `src/lib.rs`, `src/storage/mod.rs`
- Test: `tests/operation_lifecycle.rs`

**Interfaces:**
- Consumes: `Storage::write_txn`, `append_event` (Task 3); `RuntimeInstanceId` (Task 4); `AgentInvocation` (Task 8).
- Produces: `operation::{Operation, OperationStatus, FailureStage}`; `Storage::create_pending_operation(thread_id, runtime_instance_id) -> Result<String, StorageError>`; `Storage::mark_operation_started(op_id, expected_runtime) -> Result<(), StorageError>`; `Storage::mark_operation_failed(op_id, stage: FailureStage, reason: &str)`; `Storage::mark_operation_completed(op_id, outcome: serde_json::Value)`; `Storage::get_operation(op_id) -> Result<Operation, StorageError>`.

- [ ] **Step 1: Write the failing tests**

`tests/operation_lifecycle.rs`:

```rust
use shadows::operation::FailureStage;
use shadows::storage::{Storage, StorageError};

async fn fixture() -> (tempfile::TempDir, Storage, String, String) {
    let tmp = tempfile::tempdir().unwrap();
    let storage = Storage::open(&tmp.path().join("s.sqlite3")).await.unwrap();
    let runtime = storage.register_runtime_instance("test").await.unwrap();
    let params = serde_json::json!({ "slug": "demo" });
    let ctx = shadows::command::CommandContext {
        principal_kind: "User".into(),
        principal_id: "local".into(),
        command_id: "c1".into(),
        command_kind: "project.create".into(),
        command_schema_ver: 1,
        request_fingerprint: shadows::command::fingerprint("project.create", &params),
    };
    let project = storage.create_project(&ctx, "demo", "Demo").await.unwrap();
    let tctx = shadows::command::CommandContext {
        command_id: "c2".into(),
        command_kind: "thread.create".into(),
        request_fingerprint: shadows::command::fingerprint("thread.create", &params),
        ..ctx
    };
    let thread = storage.create_planning_thread(&tctx, &project.id, "T").await.unwrap();
    (tmp, storage, runtime, thread.id)
}

/// Spec §2.7. TX #1 persists Pending before anything spawns. A Pending
/// Operation is a durable attempt that has not yet claimed a running process.
#[tokio::test]
async fn phase_one_persists_pending_before_anything_spawns() {
    let (_t, storage, runtime, thread) = fixture().await;
    let op = storage.create_pending_operation(&thread, &runtime).await.unwrap();

    let loaded = storage.get_operation(&op).await.unwrap();
    assert_eq!(loaded.status_kind, "Pending");
    assert!(loaded.started_at.is_none(), "Pending must not claim a start time");
    assert!(loaded.finished_at.is_none());
}

/// Spec §2.7 and §8.3. The Pending -> Running transition is an exact CAS. It
/// must not fire against an operation owned by a different runtime, and it must
/// not fire twice.
#[tokio::test]
async fn the_running_transition_is_an_exact_compare_and_swap() {
    let (_t, storage, runtime, thread) = fixture().await;
    let op = storage.create_pending_operation(&thread, &runtime).await.unwrap();

    storage.mark_operation_started(&op, &runtime).await.unwrap();
    assert_eq!(storage.get_operation(&op).await.unwrap().status_kind, "Running");

    let again = storage.mark_operation_started(&op, &runtime).await;
    assert!(
        matches!(again, Err(StorageError::TransitionConflict { .. })),
        "a second Running transition must be refused"
    );

    let other_runtime = storage.register_runtime_instance("other").await.unwrap();
    let op2 = storage.create_pending_operation(&thread, &runtime).await.unwrap();
    let wrong_owner = storage.mark_operation_started(&op2, &other_runtime).await;
    assert!(
        matches!(wrong_owner, Err(StorageError::TransitionConflict { .. })),
        "a runtime must not start an operation it does not own"
    );
}

/// Spec §8.3. Prepare failure is not spawn failure: the process never existed.
/// Keeping them distinct is what makes a diagnosis possible later.
#[tokio::test]
async fn prepare_failure_and_spawn_failure_are_distinguishable() {
    let (_t, storage, runtime, thread) = fixture().await;

    let prepared = storage.create_pending_operation(&thread, &runtime).await.unwrap();
    storage
        .mark_operation_failed(&prepared, FailureStage::Prepare, "harness executable not found")
        .await
        .unwrap();
    let a = storage.get_operation(&prepared).await.unwrap();
    assert_eq!(a.status_kind, "Failed");
    assert_eq!(a.failure_stage.as_deref(), Some("Prepare"));
    assert!(a.started_at.is_none(), "nothing ever started");

    let spawned = storage.create_pending_operation(&thread, &runtime).await.unwrap();
    storage
        .mark_operation_failed(&spawned, FailureStage::Spawn, "os refused to start the process")
        .await
        .unwrap();
    let b = storage.get_operation(&spawned).await.unwrap();
    assert_eq!(b.failure_stage.as_deref(), Some("Spawn"));
}

/// Spec §8.6: terminal states never transition again. A retry creates a new
/// Operation; it does not revive an old one.
#[tokio::test]
async fn a_terminal_operation_never_transitions_again() {
    let (_t, storage, runtime, thread) = fixture().await;
    let op = storage.create_pending_operation(&thread, &runtime).await.unwrap();
    storage.mark_operation_started(&op, &runtime).await.unwrap();
    storage
        .mark_operation_completed(&op, serde_json::json!({ "ok": true }))
        .await
        .unwrap();

    let err = storage
        .mark_operation_failed(&op, FailureStage::Spawn, "too late")
        .await;
    assert!(matches!(err, Err(StorageError::TransitionConflict { .. })));
    assert_eq!(storage.get_operation(&op).await.unwrap().status_kind, "Completed");
}

/// Every transition appends its durable event in the same transaction.
/// Cross-cutting rule 5.
#[tokio::test]
async fn every_transition_appends_its_event_atomically() {
    let (_t, storage, runtime, thread) = fixture().await;
    let before: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM durable_event")
        .fetch_one(storage.reader()).await.unwrap();

    let op = storage.create_pending_operation(&thread, &runtime).await.unwrap();
    storage.mark_operation_started(&op, &runtime).await.unwrap();
    storage.mark_operation_completed(&op, serde_json::json!({})).await.unwrap();

    let kinds: Vec<String> = sqlx::query_scalar(
        "SELECT kind FROM durable_event WHERE operation_id = ? ORDER BY seq",
    )
    .bind(&op)
    .fetch_all(storage.reader()).await.unwrap();
    assert_eq!(kinds, vec!["OperationCreated", "OperationStarted", "OperationCompleted"]);
    let _ = before;
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --test operation_lifecycle`
Expected: FAIL — `shadows::operation` does not exist.

- [ ] **Step 3: Write `src/operation/mod.rs`**

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailureStage {
    /// Resolving the harness, building the environment, readying the workspace.
    /// The process never existed. Spec §8.3.
    Prepare,
    /// The OS refused to start the process. Spec §2.7.
    Spawn,
    /// The process ran and failed.
    Run,
}

impl FailureStage {
    pub fn as_str(self) -> &'static str {
        match self {
            FailureStage::Prepare => "Prepare",
            FailureStage::Spawn => "Spawn",
            FailureStage::Run => "Run",
        }
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct Operation {
    pub id: String,
    pub kind: String,
    pub status_kind: String,
    pub thread_id: Option<String>,
    pub runtime_instance_id: String,
    pub outcome_json: Option<String>,
    pub failure_stage: Option<String>,
    pub failure_reason: Option<String>,
    pub interrupt_reason: Option<String>,
    pub cancel_requested_at: Option<String>,
    pub created_at: String,
    pub started_at: Option<String>,
    pub finished_at: Option<String>,
}
```

- [ ] **Step 4: Write `src/storage/sqlite/operation.rs`**

```rust
use crate::events::{Actor, DurableEvent};
use crate::operation::{FailureStage, Operation};
use super::{events::append_event, now, Storage, StorageError};

impl Storage {
    /// TX #1 of the two-phase spawn. Spec §2.7: Pending is persisted before
    /// anything is spawned, so a crash between here and spawn is recoverable.
    pub async fn create_pending_operation(
        &self,
        thread_id: &str,
        runtime_instance_id: &str,
    ) -> Result<String, StorageError> {
        let id = uuid::Uuid::new_v4().to_string();
        let (id2, thread_id, runtime_instance_id, ts) =
            (id.clone(), thread_id.to_string(), runtime_instance_id.to_string(), now());
        self.write_txn(move |conn| {
            Box::pin(async move {
                sqlx::query(
                    "INSERT INTO operation
                       (id, kind, status_kind, thread_id, runtime_instance_id, created_at)
                     VALUES (?, 'PlannerTurn', 'Pending', ?, ?, ?)",
                )
                .bind(&id2).bind(&thread_id).bind(&runtime_instance_id).bind(&ts)
                .execute(&mut *conn).await?;

                append_event(
                    conn,
                    &DurableEvent::new("OperationCreated", Actor::system())
                        .with_thread(&thread_id)
                        .with_operation(&id2)
                        .with_payload(serde_json::json!({ "kind": "PlannerTurn" })),
                    &ts,
                ).await?;
                Ok(())
            })
        })
        .await?;
        Ok(id)
    }

    /// TX #2 of the two-phase spawn. Spec §8.3: no Operation becomes Running
    /// before its handle is registered and this compare-and-swap commits.
    /// The CAS pins id, expected status, AND owning runtime.
    pub async fn mark_operation_started(
        &self,
        op_id: &str,
        expected_runtime: &str,
    ) -> Result<(), StorageError> {
        let (op_id, expected_runtime, ts) =
            (op_id.to_string(), expected_runtime.to_string(), now());
        self.write_txn(move |conn| {
            Box::pin(async move {
                let affected = sqlx::query(
                    "UPDATE operation SET status_kind = 'Running', started_at = ?
                      WHERE id = ? AND status_kind = 'Pending' AND runtime_instance_id = ?",
                )
                .bind(&ts).bind(&op_id).bind(&expected_runtime)
                .execute(&mut *conn).await?
                .rows_affected();
                if affected == 0 {
                    return Err(StorageError::TransitionConflict {
                        expected: "Pending, owned by this runtime".into(),
                        found: "another status or another owner".into(),
                    });
                }
                append_event(
                    conn,
                    &DurableEvent::new("OperationStarted", Actor::system())
                        .with_operation(&op_id),
                    &ts,
                ).await?;
                Ok(())
            })
        })
        .await
    }

    pub async fn mark_operation_completed(
        &self,
        op_id: &str,
        outcome: serde_json::Value,
    ) -> Result<(), StorageError> {
        let (op_id, outcome, ts) = (op_id.to_string(), outcome.to_string(), now());
        self.write_txn(move |conn| {
            Box::pin(async move {
                let affected = sqlx::query(
                    "UPDATE operation
                        SET status_kind = 'Completed', outcome_json = ?, finished_at = ?
                      WHERE id = ? AND status_kind = 'Running'",
                )
                .bind(&outcome).bind(&ts).bind(&op_id)
                .execute(&mut *conn).await?
                .rows_affected();
                if affected == 0 {
                    return Err(StorageError::TransitionConflict {
                        expected: "Running".into(),
                        found: "another status".into(),
                    });
                }
                append_event(
                    conn,
                    &DurableEvent::new("OperationCompleted", Actor::system())
                        .with_operation(&op_id),
                    &ts,
                ).await?;
                Ok(())
            })
        })
        .await
    }

    pub async fn mark_operation_failed(
        &self,
        op_id: &str,
        stage: FailureStage,
        reason: &str,
    ) -> Result<(), StorageError> {
        let (op_id, reason, ts) = (op_id.to_string(), reason.to_string(), now());
        self.write_txn(move |conn| {
            Box::pin(async move {
                let affected = sqlx::query(
                    "UPDATE operation
                        SET status_kind = 'Failed', failure_stage = ?, failure_reason = ?,
                            finished_at = ?
                      WHERE id = ? AND status_kind IN ('Pending','Running')",
                )
                .bind(stage.as_str()).bind(&reason).bind(&ts).bind(&op_id)
                .execute(&mut *conn).await?
                .rows_affected();
                if affected == 0 {
                    return Err(StorageError::TransitionConflict {
                        expected: "Pending or Running".into(),
                        found: "already terminal".into(),
                    });
                }
                append_event(
                    conn,
                    &DurableEvent::new("OperationFailed", Actor::system())
                        .with_operation(&op_id)
                        .with_payload(serde_json::json!({ "stage": stage.as_str() })),
                    &ts,
                ).await?;
                Ok(())
            })
        })
        .await
    }

    pub async fn get_operation(&self, op_id: &str) -> Result<Operation, StorageError> {
        let r: (String, String, String, Option<String>, String, Option<String>,
                Option<String>, Option<String>, Option<String>, Option<String>,
                String, Option<String>, Option<String>) = sqlx::query_as(
            "SELECT id, kind, status_kind, thread_id, runtime_instance_id, outcome_json,
                    failure_stage, failure_reason, interrupt_reason, cancel_requested_at,
                    created_at, started_at, finished_at
               FROM operation WHERE id = ?",
        )
        .bind(op_id)
        .fetch_optional(self.reader())
        .await?
        .ok_or(StorageError::NotFound("operation"))?;
        Ok(Operation {
            id: r.0, kind: r.1, status_kind: r.2, thread_id: r.3,
            runtime_instance_id: r.4, outcome_json: r.5, failure_stage: r.6,
            failure_reason: r.7, interrupt_reason: r.8, cancel_requested_at: r.9,
            created_at: r.10, started_at: r.11, finished_at: r.12,
        })
    }
}
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test --test operation_lifecycle`
Expected: PASS, all five tests.

- [ ] **Step 6: Commit**

```bash
git add src/operation src/storage src/lib.rs tests/operation_lifecycle.rs
git commit -m "feat(operation): two-phase spawn with exact compare-and-swap transitions"
```

---

## Task 10: Cancellation — request, confirmed termination, terminal Cancelled

**Files:**
- Modify: `src/storage/sqlite/operation.rs`
- Create: `src/planner/mod.rs`
- Modify: `src/lib.rs`
- Test: `tests/operation_lifecycle.rs`, `tests/planner_turn.rs`

**Interfaces:**
- Consumes: everything from Tasks 3–9.
- Produces:
  - `Storage::request_cancellation(&self, op_id: &str, by_kind: &str, by_id: &str) -> Result<(), StorageError>`
  - `Storage::mark_operation_cancelled(&self, op_id: &str) -> Result<(), StorageError>`
  - `planner::LiveHandles` — `Default`, with a `pub(crate)` inner `Mutex<HashMap<String, ProcessHandle>>` so `cli::serve` can enumerate this runtime's live operations at shutdown.
  - `planner::PlannerTurn::start(runtime: Arc<Runtime>, handles: Arc<LiveHandles>, harness: Arc<ClaudeHarness>, thread_id: String, prompt: String, cwd: PathBuf, resume_session_id: Option<String>, bus: broadcast::Sender<(String, StreamItem)>) -> Result<String, StorageError>`
  - `planner::PlannerTurn::stop(runtime: Arc<Runtime>, handles: Arc<LiveHandles>, op_id: &str) -> Result<(), StorageError>`

**There is no `mark_operation_interrupted` capability.** Interruption is not a transition anyone requests — it is what startup recovery concludes about work a previous runtime left behind, so it belongs to `reconcile_orphans` in Task 4 and to nothing else. A public method for it would let live code mark its own work interrupted, which is a claim only a later runtime is entitled to make.

**The rule this task exists to enforce.** Spec §2.3: `Cancelled` means Shadows confirmed the managed execution is no longer running and persisted the terminal transition. It never means only that a user requested cancellation.

- [ ] **Step 1: Write the failing tests**

Append to `tests/operation_lifecycle.rs`:

```rust
/// Spec §2.3. The request and the terminal state are two separate facts in two
/// separate transactions. A request alone leaves the operation non-terminal.
#[tokio::test]
async fn a_cancellation_request_does_not_make_an_operation_terminal() {
    let (_t, storage, runtime, thread) = fixture().await;
    let op = storage.create_pending_operation(&thread, &runtime).await.unwrap();
    storage.mark_operation_started(&op, &runtime).await.unwrap();

    storage.request_cancellation(&op, "User", "local").await.unwrap();

    let loaded = storage.get_operation(&op).await.unwrap();
    assert_eq!(loaded.status_kind, "Running", "a request is not a terminal state");
    assert!(loaded.cancel_requested_at.is_some());
    assert!(loaded.finished_at.is_none());
}

/// Spec §2.3. Terminal Cancelled is written only after termination is
/// confirmed, and it is what closes the operation.
#[tokio::test]
async fn cancelled_is_written_after_confirmation_and_closes_the_operation() {
    let (_t, storage, runtime, thread) = fixture().await;
    let op = storage.create_pending_operation(&thread, &runtime).await.unwrap();
    storage.mark_operation_started(&op, &runtime).await.unwrap();
    storage.request_cancellation(&op, "User", "local").await.unwrap();

    storage.mark_operation_cancelled(&op).await.unwrap();

    let loaded = storage.get_operation(&op).await.unwrap();
    assert_eq!(loaded.status_kind, "Cancelled");
    assert!(loaded.finished_at.is_some());
    assert!(loaded.cancel_requested_at.is_some(), "the request is retained as history");

    let kinds: Vec<String> = sqlx::query_scalar(
        "SELECT kind FROM durable_event WHERE operation_id = ? ORDER BY seq",
    )
    .bind(&op)
    .fetch_all(storage.reader()).await.unwrap();
    assert_eq!(
        kinds,
        vec!["OperationCreated", "OperationStarted",
             "OperationCancellationRequested", "OperationCancelled"]
    );
}

/// Spec §8.4 case 4. A process that exited on its own before cancellation took
/// termination ownership is Completed by its own exit, not Cancelled. Shadows
/// does not claim to have stopped something that had already stopped.
#[tokio::test]
async fn a_natural_exit_wins_over_an_in_flight_cancellation() {
    let (_t, storage, runtime, thread) = fixture().await;
    let op = storage.create_pending_operation(&thread, &runtime).await.unwrap();
    storage.mark_operation_started(&op, &runtime).await.unwrap();
    storage.request_cancellation(&op, "User", "local").await.unwrap();

    // The process exits before containment takes ownership.
    storage.mark_operation_completed(&op, serde_json::json!({ "ok": true })).await.unwrap();

    let loaded = storage.get_operation(&op).await.unwrap();
    assert_eq!(loaded.status_kind, "Completed");
    assert!(loaded.cancel_requested_at.is_some(),
            "a terminal non-cancelled status may retain request metadata as history");

    // Spec §8.4 case 5 in reverse: the late Cancelled must not overwrite it.
    let late = storage.mark_operation_cancelled(&op).await;
    assert!(matches!(late, Err(StorageError::TransitionConflict { .. })));
}

/// Spec §8.4 case 6. If termination cannot be confirmed, the request is
/// preserved and Cancelled is NOT written. The operation stays non-terminal
/// until recovery can make an honest Interrupted transition.
#[tokio::test]
async fn unconfirmed_termination_leaves_the_operation_non_terminal() {
    let (_t, storage, runtime, thread) = fixture().await;
    let op = storage.create_pending_operation(&thread, &runtime).await.unwrap();
    storage.mark_operation_started(&op, &runtime).await.unwrap();
    storage.request_cancellation(&op, "User", "local").await.unwrap();

    // Termination failed: the daemon simply does not call mark_operation_cancelled.
    let loaded = storage.get_operation(&op).await.unwrap();
    assert_eq!(loaded.status_kind, "Running");

    // A later runtime resolves it honestly, and to Interrupted, not Cancelled,
    // because the final process outcome is unknown after a crash.
    let next = storage.register_runtime_instance("next").await.unwrap();
    let report = storage.reconcile_orphans(&next).await.unwrap();
    assert_eq!(report.interrupted, vec![op.clone()]);
    let after = storage.get_operation(&op).await.unwrap();
    assert_eq!(after.status_kind, "Interrupted");
    assert!(after.cancel_requested_at.is_some(),
            "the request stays visible on the interrupted record");
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --test operation_lifecycle`
Expected: FAIL — `request_cancellation` and `mark_operation_cancelled` do not exist.

- [ ] **Step 3: Extend `src/storage/sqlite/operation.rs`**

```rust
impl Storage {
    /// TX #1 of cancellation. Spec §2.3: this records that a stop was asked
    /// for. It does not stop anything and it does not make the operation
    /// terminal.
    pub async fn request_cancellation(
        &self,
        op_id: &str,
        by_kind: &str,
        by_id: &str,
    ) -> Result<(), StorageError> {
        let (op_id, by_kind, by_id, ts) =
            (op_id.to_string(), by_kind.to_string(), by_id.to_string(), now());
        self.write_txn(move |conn| {
            Box::pin(async move {
                let affected = sqlx::query(
                    "UPDATE operation
                        SET cancel_requested_at = ?, cancel_requested_by_kind = ?,
                            cancel_requested_by_id = ?
                      WHERE id = ? AND status_kind IN ('Pending','Running')
                        AND cancel_requested_at IS NULL",
                )
                .bind(&ts).bind(&by_kind).bind(&by_id).bind(&op_id)
                .execute(&mut *conn).await?
                .rows_affected();
                if affected == 0 {
                    // Already requested, or already terminal. Spec §2.3: a
                    // repeat records its own idempotent AlreadyTerminal result
                    // and runs no process effects. Treated as success here.
                    return Ok(());
                }
                append_event(
                    conn,
                    &DurableEvent::new("OperationCancellationRequested",
                                       Actor { kind: by_kind.clone(), id: by_id.clone() })
                        .with_operation(&op_id),
                    &ts,
                ).await?;
                Ok(())
            })
        })
        .await
    }

    /// TX #2 of cancellation, written ONLY after the process tree is confirmed
    /// gone. Spec §2.3. The CAS refuses to overwrite a terminal state, which is
    /// what makes a natural exit win the race (§8.4 case 4 and 5).
    pub async fn mark_operation_cancelled(&self, op_id: &str) -> Result<(), StorageError> {
        let (op_id, ts) = (op_id.to_string(), now());
        self.write_txn(move |conn| {
            Box::pin(async move {
                let affected = sqlx::query(
                    "UPDATE operation SET status_kind = 'Cancelled', finished_at = ?
                      WHERE id = ? AND status_kind IN ('Pending','Running')",
                )
                .bind(&ts).bind(&op_id)
                .execute(&mut *conn).await?
                .rows_affected();
                if affected == 0 {
                    return Err(StorageError::TransitionConflict {
                        expected: "Pending or Running".into(),
                        found: "already terminal".into(),
                    });
                }
                append_event(
                    conn,
                    &DurableEvent::new("OperationCancelled", Actor::system())
                        .with_operation(&op_id),
                    &ts,
                ).await?;
                Ok(())
            })
        })
        .await
    }
}
```

- [ ] **Step 4: Write `src/planner/mod.rs`**

This is where the two-phase spawn and the interlock actually run.

```rust
use std::sync::Arc;

use tokio::io::AsyncBufReadExt;
use tokio::sync::Mutex;

use crate::agent::{claude::ClaudeHarness, AgentHarness, AgentInvocation, StreamItem};
use crate::operation::FailureStage;
use crate::process::{spawn, ProcessHandle};
use crate::runtime::Runtime;
use crate::storage::StorageError;

/// Live handles for operations this runtime owns. Spec §8.3: termination goes
/// through the containment handle that owns the tree, never through a PID read
/// from the database, because the OS reuses PIDs.
#[derive(Default)]
pub struct LiveHandles(Mutex<std::collections::HashMap<String, ProcessHandle>>);

pub struct PlannerTurn;

impl PlannerTurn {
    /// Returns the operation id as soon as Pending is committed. The turn keeps
    /// running; the caller subscribes to the stream separately.
    pub async fn start(
        runtime: Arc<Runtime>,
        handles: Arc<LiveHandles>,
        harness: Arc<ClaudeHarness>,
        thread_id: String,
        prompt: String,
        cwd: std::path::PathBuf,
        resume_session_id: Option<String>,
        bus: tokio::sync::broadcast::Sender<(String, StreamItem)>,
    ) -> Result<String, StorageError> {
        // TX #1: the durable attempt exists before anything spawns.
        let op_id = runtime
            .storage
            .create_pending_operation(&thread_id, &runtime.instance_id)
            .await?;

        let invocation = AgentInvocation {
            operation_id: op_id.clone(),
            role: "Planner".into(),
            model: "sonnet".into(),
            prompt,
            cwd,
            resume_session_id,
            session_id: uuid::Uuid::new_v4().to_string(),
        };

        // Prepare: resolve, build the environment, ready the workspace. A
        // failure here is not a spawn failure — no process ever existed.
        let spec = harness.to_process_spec(&invocation);
        if !spec.executable.exists() && spec.executable.components().count() > 1 {
            runtime
                .storage
                .mark_operation_failed(
                    &op_id,
                    FailureStage::Prepare,
                    &format!("harness executable not found: {}", spec.executable.display()),
                )
                .await?;
            return Ok(op_id);
        }

        let mut handle = match spawn(spec) {
            Ok(h) => h,
            Err(e) => {
                runtime
                    .storage
                    .mark_operation_failed(&op_id, FailureStage::Spawn, &e.to_string())
                    .await?;
                return Ok(op_id);
            }
        };

        // Spec §8.3: register the handle, THEN commit Running. A committed
        // transition without a registered handle is a state we must not produce.
        let lines = handle.stdout_lines().take();
        {
            let mut map = handles.0.lock().await;
            map.insert(op_id.clone(), handle);
        }

        // TX #2.
        runtime
            .storage
            .mark_operation_started(&op_id, &runtime.instance_id)
            .await?;

        let reader_op = op_id.clone();
        let reader_runtime = runtime.clone();
        let reader_handles = handles.clone();
        let reader_harness = harness.clone();
        tokio::spawn(async move {
            let mut outcome = serde_json::json!({ "stop_reason": null });
            if let Some(mut lines) = lines {
                while let Ok(Some(line)) = lines.next_line().await {
                    let item = reader_harness.classify(&line);
                    match &item {
                        // Durable: write before forwarding. The UI may drop a
                        // frame; the record may not.
                        StreamItem::Entry { uuid, role, text } => {
                            let _ = reader_runtime
                                .storage
                                .append_thread_entry(
                                    &thread_id, "AgentMessage", role, uuid, text,
                                )
                                .await;
                        }
                        StreamItem::TurnEnd { subtype, stop_reason } => {
                            outcome = serde_json::json!({
                                "subtype": subtype, "stop_reason": stop_reason
                            });
                        }
                        _ => {}
                    }
                    let _ = bus.send((reader_op.clone(), item));
                }
            }

            // The process ended. Take the handle back and let its containment
            // drop, then record the terminal state. If a cancellation already
            // won, this CAS is refused and that is correct (§8.4 case 5).
            let _ = reader_handles.0.lock().await.remove(&reader_op);
            let _ = reader_runtime
                .storage
                .mark_operation_completed(&reader_op, outcome)
                .await;
        });

        Ok(op_id)
    }

    /// Spec §2.3: request, terminate, confirm, then write Cancelled. If
    /// termination cannot be confirmed, Cancelled is not written and the
    /// operation is left for recovery (§8.4 case 6).
    pub async fn stop(
        runtime: Arc<Runtime>,
        handles: Arc<LiveHandles>,
        op_id: &str,
    ) -> Result<(), StorageError> {
        runtime.storage.request_cancellation(op_id, "User", "local").await?;

        let mut map = handles.0.lock().await;
        let Some(mut handle) = map.remove(op_id) else {
            // No live handle: either it already exited, or this runtime does
            // not own it. Either way we cannot confirm termination, so we do
            // not write Cancelled.
            return Ok(());
        };
        drop(map);

        if handle.terminate_tree().is_err() {
            return Ok(()); // unconfirmed; left non-terminal on purpose
        }
        // `wait` returning is the confirmation that the tree is reaped.
        let _ = handle.wait().await;
        runtime.storage.mark_operation_cancelled(op_id).await
    }
}
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test --test operation_lifecycle`
Expected: PASS, all nine tests.

- [ ] **Step 6: Commit**

```bash
git add src/planner src/storage src/lib.rs tests/operation_lifecycle.rs
git commit -m "feat(planner): cancellation requires confirmed termination before Cancelled"
```

---

## Task 11: Durable replay with a no-gap handoff to live

**Files:**
- Create: `src/storage/sqlite/events_read.rs`, `src/protocol/mod.rs`, `src/protocol/sse.rs`
- Modify: `src/lib.rs`, `src/storage/mod.rs`, `src/cli/mod.rs`
- Test: `tests/resync.rs`

**Interfaces:**
- Consumes: `EventCursor` (Task 3); `Storage` (Tasks 2–10); `LiveHandles`, `PlannerTurn` (Task 10).
- Produces: `Storage::current_cursor() -> Result<EventCursor, StorageError>`; `Storage::read_events_after(cursor: EventCursor, thread_id: &str, limit: i64) -> Result<Vec<StoredEvent>, StorageError>`; `StoredEvent { seq, kind, operation_id, payload_json, created_at }`; `protocol::router(AppState) -> axum::Router`; `AppState { runtime, storage, handles, harness, bus }`.

**The guarantee.** Spec §2.10: durable replay, then a no-gap handoff to live, with de-duplication by durable sequence. The subscription is opened *before* the replay is read, so an event committed during the replay is buffered rather than lost.

- [ ] **Step 1: Write the failing tests**

`tests/resync.rs`:

```rust
use shadows::events::EventCursor;
use shadows::storage::Storage;

/// Spec §2.10. Reading after a cursor returns exactly the events the client has
/// not seen, in sequence order, with no gap and no repeat.
#[tokio::test]
async fn reading_after_a_cursor_returns_the_unseen_tail_in_order() {
    let tmp = tempfile::tempdir().unwrap();
    let storage = Storage::open(&tmp.path().join("s.sqlite3")).await.unwrap();
    let params = serde_json::json!({ "slug": "demo" });
    let ctx = shadows::command::CommandContext {
        principal_kind: "User".into(), principal_id: "local".into(),
        command_id: "c1".into(), command_kind: "project.create".into(),
        command_schema_ver: 1,
        request_fingerprint: shadows::command::fingerprint("project.create", &params),
    };
    let project = storage.create_project(&ctx, "demo", "Demo").await.unwrap();
    let tctx = shadows::command::CommandContext {
        command_id: "c2".into(), command_kind: "thread.create".into(),
        request_fingerprint: shadows::command::fingerprint("thread.create", &params),
        ..ctx
    };
    let thread = storage.create_planning_thread(&tctx, &project.id, "T").await.unwrap();

    let mid = storage.current_cursor().await.unwrap();

    for body in ["one", "two", "three"] {
        storage.append_thread_entry(&thread.id, "UserMessage", "User", "local", body)
            .await.unwrap();
    }

    let tail = storage.read_events_after(mid, &thread.id, 100).await.unwrap();
    assert_eq!(tail.len(), 3, "exactly the events after the cursor");
    let seqs: Vec<i64> = tail.iter().map(|e| e.seq).collect();
    let mut sorted = seqs.clone();
    sorted.sort();
    assert_eq!(seqs, sorted, "events arrive in sequence order");
    assert!(tail.iter().all(|e| e.kind == "ThreadEntryAppended"));

    // Re-reading from the same cursor is idempotent.
    let again = storage.read_events_after(mid, &thread.id, 100).await.unwrap();
    assert_eq!(seqs, again.iter().map(|e| e.seq).collect::<Vec<_>>());

    // Reading after the last delivered seq returns nothing.
    let after_all = EventCursor(*seqs.last().unwrap());
    assert!(storage.read_events_after(after_all, &thread.id, 100).await.unwrap().is_empty());
}

/// A cursor from before any event returns the whole thread history.
#[tokio::test]
async fn a_zero_cursor_replays_the_whole_thread() {
    let tmp = tempfile::tempdir().unwrap();
    let storage = Storage::open(&tmp.path().join("s.sqlite3")).await.unwrap();
    let params = serde_json::json!({ "slug": "demo" });
    let ctx = shadows::command::CommandContext {
        principal_kind: "User".into(), principal_id: "local".into(),
        command_id: "c1".into(), command_kind: "project.create".into(),
        command_schema_ver: 1,
        request_fingerprint: shadows::command::fingerprint("project.create", &params),
    };
    let project = storage.create_project(&ctx, "demo", "Demo").await.unwrap();
    let tctx = shadows::command::CommandContext {
        command_id: "c2".into(), command_kind: "thread.create".into(),
        request_fingerprint: shadows::command::fingerprint("thread.create", &params),
        ..ctx
    };
    let thread = storage.create_planning_thread(&tctx, &project.id, "T").await.unwrap();
    storage.append_thread_entry(&thread.id, "UserMessage", "User", "local", "hi")
        .await.unwrap();

    let all = storage.read_events_after(EventCursor(0), &thread.id, 100).await.unwrap();
    assert_eq!(all.len(), 2, "thread creation and the entry");
    assert_eq!(all[0].kind, "PlanningThreadCreated");
    assert_eq!(all[1].kind, "ThreadEntryAppended");
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --test resync`
Expected: FAIL — `current_cursor` and `read_events_after` do not exist.

- [ ] **Step 3: Write `src/storage/sqlite/events_read.rs`**

```rust
use crate::events::EventCursor;
use super::{Storage, StorageError};

#[derive(Debug, Clone, serde::Serialize)]
pub struct StoredEvent {
    pub seq: i64,
    pub kind: String,
    pub operation_id: Option<String>,
    pub payload_json: String,
    pub created_at: String,
}

impl Storage {
    /// The highest sequence committed so far. Spec §2.10: the snapshot and the
    /// cursor must come from the same read, so a caller building a snapshot
    /// takes this inside that same read transaction.
    pub async fn current_cursor(&self) -> Result<EventCursor, StorageError> {
        let seq: Option<i64> = sqlx::query_scalar("SELECT MAX(seq) FROM durable_event")
            .fetch_one(self.reader())
            .await?;
        Ok(EventCursor(seq.unwrap_or(0)))
    }

    /// Thread-scoped replay. Ordering is by `seq` explicitly — never by
    /// insertion order. On SQLite, sequence order is commit order; see the
    /// OPEN block in §6.18 before assuming that on another backend.
    pub async fn read_events_after(
        &self,
        cursor: EventCursor,
        thread_id: &str,
        limit: i64,
    ) -> Result<Vec<StoredEvent>, StorageError> {
        let rows: Vec<(i64, String, Option<String>, String, String)> = sqlx::query_as(
            "SELECT seq, kind, operation_id, payload_json, created_at
               FROM durable_event
              WHERE thread_id = ? AND seq > ?
              ORDER BY seq
              LIMIT ?",
        )
        .bind(thread_id)
        .bind(cursor.0)
        .bind(limit)
        .fetch_all(self.reader())
        .await?;
        Ok(rows.into_iter().map(|r| StoredEvent {
            seq: r.0, kind: r.1, operation_id: r.2, payload_json: r.3, created_at: r.4,
        }).collect())
    }
}
```

- [ ] **Step 4: Write `src/protocol/sse.rs`**

```rust
use std::convert::Infallible;

use axum::extract::{Query, State};
use axum::response::sse::{Event, Sse};
use tokio_stream::wrappers::ReceiverStream;

use crate::agent::StreamItem;
use crate::events::EventCursor;
use super::AppState;

#[derive(serde::Deserialize)]
pub struct SubscribeQuery {
    pub thread_id: String,
    /// The last durable sequence this client has already applied.
    #[serde(default)]
    pub after: i64,
}

/// Spec §2.10: durable replay, then a no-gap handoff to live, then
/// de-duplication by durable sequence.
///
/// The order below is the whole guarantee. The live subscription is taken
/// FIRST, so anything committed while the replay is being read lands in the
/// broadcast buffer instead of falling into the gap between them. Replayed
/// events carry their `seq`; the client discards any live event whose `seq` it
/// has already applied.
pub async fn subscribe(
    State(state): State<AppState>,
    Query(q): Query<SubscribeQuery>,
) -> Sse<ReceiverStream<Result<Event, Infallible>>> {
    let (tx, rx) = tokio::sync::mpsc::channel::<Result<Event, Infallible>>(1024);
    let mut live = state.bus.subscribe(); // taken before the replay is read

    tokio::spawn(async move {
        // 1. Durable replay.
        let mut last_seq = q.after;
        loop {
            let batch = match state
                .storage
                .read_events_after(EventCursor(last_seq), &q.thread_id, 500)
                .await
            {
                Ok(b) => b,
                Err(e) => {
                    let _ = tx.send(Ok(Event::default().event("fatal").data(e.to_string()))).await;
                    return;
                }
            };
            if batch.is_empty() {
                break;
            }
            for ev in batch {
                last_seq = ev.seq;
                let payload = serde_json::json!({
                    "seq": ev.seq, "kind": ev.kind, "payload": ev.payload_json,
                });
                if tx.send(Ok(Event::default().event("durable").data(payload.to_string())))
                    .await.is_err()
                {
                    return;
                }
            }
        }

        // 2. Handoff. Tell the client where the durable replay ended so it can
        // de-duplicate anything the live stream repeats.
        let _ = tx
            .send(Ok(Event::default().event("caught-up").data(last_seq.to_string())))
            .await;

        // 3. Live. Transient deltas are forwarded and never stored.
        loop {
            match live.recv().await {
                Ok((op_id, item)) => {
                    let ev = match item {
                        StreamItem::Delta { text } => Event::default()
                            .event("delta")
                            .data(serde_json::json!({ "op": op_id, "text": text }).to_string()),
                        StreamItem::Entry { uuid, role, text } => Event::default()
                            .event("entry")
                            .data(serde_json::json!({
                                "op": op_id, "uuid": uuid, "role": role, "text": text
                            }).to_string()),
                        StreamItem::TurnEnd { subtype, stop_reason } => Event::default()
                            .event("turn-end")
                            .data(serde_json::json!({
                                "op": op_id, "subtype": subtype, "stop_reason": stop_reason
                            }).to_string()),
                        StreamItem::Operational { label, .. } => Event::default()
                            .event("meta")
                            .data(serde_json::json!({ "op": op_id, "label": label }).to_string()),
                        StreamItem::Unparsed(_) => continue,
                    };
                    if tx.send(Ok(ev)).await.is_err() {
                        return;
                    }
                }
                // Spec §8.4 case 7: a client falling behind or disconnecting
                // never cancels work. It resubscribes with its last seq.
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                    let _ = tx.send(Ok(Event::default().event("lagged").data(""))).await;
                }
                Err(_) => return,
            }
        }
    });

    Sse::new(ReceiverStream::new(rx))
}
```

- [ ] **Step 5: Write `src/protocol/mod.rs`**

```rust
use std::sync::Arc;

use axum::extract::{Path, State};
use axum::response::Html;
use axum::routing::{get, post};
use axum::{Json, Router};

use crate::agent::{claude::ClaudeHarness, StreamItem};
use crate::command::{fingerprint, CommandContext};
use crate::planner::{LiveHandles, PlannerTurn};
use crate::runtime::Runtime;
use crate::storage::Storage;

pub mod sse;

#[derive(Clone)]
pub struct AppState {
    pub runtime: Arc<Runtime>,
    pub storage: Arc<Storage>,
    pub handles: Arc<LiveHandles>,
    pub harness: Arc<ClaudeHarness>,
    pub bus: tokio::sync::broadcast::Sender<(String, StreamItem)>,
    pub project_root: std::path::PathBuf,
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/", get(index))
        .route("/api/projects", get(list_projects).post(create_project))
        .route("/api/projects/{id}/threads", get(list_threads).post(create_thread))
        .route("/api/threads/{id}/entries", get(list_entries))
        .route("/api/threads/{id}/turns", post(start_turn))
        .route("/api/operations/{id}/stop", post(stop_turn))
        .route("/api/subscribe", get(sse::subscribe))
        .with_state(state)
}

/// The whole web client. Spec §1.0: the daemon serves it and never opens it.
async fn index() -> Html<&'static str> {
    Html(include_str!("index.html"))
}

#[derive(serde::Deserialize)]
struct CreateProject {
    command_id: String,
    slug: String,
    name: String,
}

fn ctx(command_id: String, kind: &str, params: serde_json::Value) -> CommandContext {
    CommandContext {
        principal_kind: "User".into(),
        principal_id: "local".into(),
        command_id,
        command_kind: kind.into(),
        command_schema_ver: 1,
        request_fingerprint: fingerprint(kind, &params),
    }
}

async fn list_projects(State(s): State<AppState>) -> Result<Json<serde_json::Value>, Failure> {
    Ok(Json(serde_json::json!(s.storage.list_projects().await?)))
}

async fn create_project(
    State(s): State<AppState>,
    Json(body): Json<CreateProject>,
) -> Result<Json<serde_json::Value>, Failure> {
    let params = serde_json::json!({ "slug": body.slug, "name": body.name });
    let c = ctx(body.command_id, "project.create", params);
    Ok(Json(serde_json::json!(
        s.storage.create_project(&c, &body.slug, &body.name).await?
    )))
}

async fn list_threads(
    State(s): State<AppState>,
    Path(project_id): Path<String>,
) -> Result<Json<serde_json::Value>, Failure> {
    Ok(Json(serde_json::json!(
        s.storage.list_threads_for_project(&project_id).await?
    )))
}

#[derive(serde::Deserialize)]
struct CreateThread {
    command_id: String,
    title: String,
}

async fn create_thread(
    State(s): State<AppState>,
    Path(project_id): Path<String>,
    Json(body): Json<CreateThread>,
) -> Result<Json<serde_json::Value>, Failure> {
    let params = serde_json::json!({ "project": project_id, "title": body.title });
    let c = ctx(body.command_id, "thread.create", params);
    Ok(Json(serde_json::json!(
        s.storage.create_planning_thread(&c, &project_id, &body.title).await?
    )))
}

async fn list_entries(
    State(s): State<AppState>,
    Path(thread_id): Path<String>,
) -> Result<Json<serde_json::Value>, Failure> {
    Ok(Json(serde_json::json!(
        s.storage.list_thread_entries(&thread_id).await?
    )))
}

#[derive(serde::Deserialize)]
struct StartTurn {
    prompt: String,
    #[serde(default)]
    resume_session_id: Option<String>,
}

/// Spec §3.3: a long-running command returns 202 and an operation id. The
/// operation reaches its terminal outcome later.
async fn start_turn(
    State(s): State<AppState>,
    Path(thread_id): Path<String>,
    Json(body): Json<StartTurn>,
) -> Result<(axum::http::StatusCode, Json<serde_json::Value>), Failure> {
    // Record the user's message as a durable entry before the turn starts, so
    // a restart mid-turn still shows what was asked.
    s.storage
        .append_thread_entry(&thread_id, "UserMessage", "User", "local", &body.prompt)
        .await?;

    let op = PlannerTurn::start(
        s.runtime.clone(),
        s.handles.clone(),
        s.harness.clone(),
        thread_id,
        body.prompt,
        s.project_root.clone(),
        body.resume_session_id,
        s.bus.clone(),
    )
    .await?;

    Ok((
        axum::http::StatusCode::ACCEPTED,
        Json(serde_json::json!({ "operation_id": op })),
    ))
}

async fn stop_turn(
    State(s): State<AppState>,
    Path(op_id): Path<String>,
) -> Result<Json<serde_json::Value>, Failure> {
    PlannerTurn::stop(s.runtime.clone(), s.handles.clone(), &op_id).await?;
    Ok(Json(serde_json::json!(s.storage.get_operation(&op_id).await?)))
}

/// Transport mapping lives here and nowhere else. Spec §3.3: `Blocked` and
/// `Rejected` are domain outcomes, not HTTP failures, and would be returned as
/// 200 with the outcome — they are not reachable in Milestone 0.
pub struct Failure(crate::storage::StorageError);

impl From<crate::storage::StorageError> for Failure {
    fn from(e: crate::storage::StorageError) -> Self {
        Failure(e)
    }
}

impl axum::response::IntoResponse for Failure {
    fn into_response(self) -> axum::response::Response {
        use crate::error::ErrorCode;
        use crate::storage::StorageError as E;
        let (status, code) = match &self.0 {
            E::CommandConflict => (axum::http::StatusCode::CONFLICT, ErrorCode::CommandConflict),
            E::NotFound(_) => (axum::http::StatusCode::NOT_FOUND, ErrorCode::InvalidCommand),
            E::TransitionConflict { .. } => {
                (axum::http::StatusCode::CONFLICT, ErrorCode::StorageConstraintViolation)
            }
            _ => (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                ErrorCode::StorageUnavailable,
            ),
        };
        (status, Json(serde_json::json!({ "code": code, "message": self.0.to_string() })))
            .into_response()
    }
}
```

- [ ] **Step 6: Wire it into `src/cli/mod.rs`**

Replace the placeholder router from Task 1:

```rust
use std::sync::Arc;

use crate::agent::claude::ClaudeHarness;
use crate::config::Config;
use crate::planner::LiveHandles;
use crate::protocol::{router, AppState};
use crate::runtime::Runtime;
use crate::storage::{StopKind, Storage};

pub async fn serve(config: Config) -> anyhow::Result<()> {
    let storage = Arc::new(Storage::open(&config.db_path).await?);
    let (runtime, report) = Runtime::start(storage.clone()).await?;
    let runtime = Arc::new(runtime);
    tracing::info!(
        interrupted = report.interrupted.len(),
        anomalies = report.anomalies.len(),
        "startup recovery complete"
    );

    let version = harness_version(&config.harness_path).await;
    let (bus, _) = tokio::sync::broadcast::channel(4096);
    let state = AppState {
        runtime: runtime.clone(),
        storage,
        handles: Arc::new(LiveHandles::default()),
        harness: Arc::new(ClaudeHarness::new(config.harness_path.clone(), version)),
        bus,
        project_root: std::env::current_dir()?,
    };

    let listener = tokio::net::TcpListener::bind(config.bind).await?;
    let addr = listener.local_addr()?;
    println!("shadows serve listening on http://{addr}");

    // Spec §8.5: shutdown reuses the cancellation path. There is no drain mode.
    let shutdown_runtime = runtime.clone();
    let shutdown_state = state.clone();
    axum::serve(listener, router(state))
        .with_graceful_shutdown(async move {
            let _ = tokio::signal::ctrl_c().await;
            tracing::info!("stop signal received; cancelling this runtime's operations");
            let ids: Vec<String> = {
                let map = shutdown_state.handles.0.lock().await;
                map.keys().cloned().collect()
            };
            let mut all_confirmed = true;
            for op in ids {
                if crate::planner::PlannerTurn::stop(
                    shutdown_runtime.clone(),
                    shutdown_state.handles.clone(),
                    &op,
                )
                .await
                .is_err()
                {
                    all_confirmed = false;
                }
            }
            let kind = if all_confirmed { StopKind::Graceful } else { StopKind::Escalated };
            let _ = shutdown_runtime.stop(kind).await;
        })
        .await?;
    Ok(())
}

/// Spec §1.4: read the harness's self-reported version and record it. The
/// measured stream contract belongs to one installation at one version.
async fn harness_version(path: &std::path::Path) -> String {
    match tokio::process::Command::new(path).arg("--version").output().await {
        Ok(out) => String::from_utf8_lossy(&out.stdout).trim().to_string(),
        Err(_) => "unknown".to_string(),
    }
}
```

`LiveHandles`'s inner map must be `pub(crate)` for `serve` to enumerate it.

- [ ] **Step 7: Run tests to verify they pass**

Run: `cargo test`
Expected: PASS, the whole suite.

- [ ] **Step 8: Commit**

```bash
git add src/protocol src/storage src/cli src/lib.rs tests/resync.rs
git commit -m "feat(protocol): durable replay with a no-gap handoff to live SSE"
```

---

## Task 12: The web client

**Files:**
- Create: `src/protocol/index.html`
- Test: `tests/web_client.rs`

**Interfaces:**
- Consumes: every route from Task 11.
- Produces: nothing other tasks consume.

**No framework, no bundler.** The serve/stream spike established that one embedded page covers this entire milestone; choosing a framework now would decide a question the milestone does not ask.

- [ ] **Step 1: Write the failing test**

`tests/web_client.rs`:

```rust
/// The page is embedded in the binary, so there is no build step and no asset
/// path to get wrong. This test is what catches an accidental external
/// dependency being introduced later.
#[test]
fn the_client_is_self_contained() {
    let html = include_str!("../src/protocol/index.html");
    assert!(html.contains("<title>Shadows</title>"));
    for id in ["projects", "threads", "entries", "live", "prompt", "start", "stop"] {
        assert!(html.contains(&format!("id=\"{id}\"")), "missing #{id}");
    }
    assert!(
        !html.contains("http://") && !html.contains("https://"),
        "the client must not fetch anything external"
    );
    assert!(html.contains("EventSource"), "the client must subscribe over SSE");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test web_client`
Expected: FAIL — `src/protocol/index.html` does not exist.

- [ ] **Step 3: Write `src/protocol/index.html`**

```html
<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<title>Shadows</title>
<style>
  body { font: 14px/1.6 ui-monospace, Consolas, monospace; margin: 0; padding: 16px;
         background: #101215; color: #d8dee9; }
  h1 { font-size: 15px; margin: 0 0 12px; }
  select, input, button, textarea {
    font: inherit; background: #181b20; color: #d8dee9;
    border: 1px solid #2b313a; border-radius: 4px; padding: 6px 8px; }
  button { background: #2b6cb0; border: 0; cursor: pointer; }
  button.secondary { background: #2b313a; }
  button:disabled { opacity: .5; cursor: default; }
  .row { display: flex; gap: 8px; align-items: center; margin-bottom: 10px; flex-wrap: wrap; }
  .cols { display: grid; grid-template-columns: 1fr 1fr; gap: 12px; }
  section { border: 1px solid #2b313a; border-radius: 4px; }
  section > h2 { font-size: 12px; margin: 0; padding: 6px 10px; background: #181b20;
                 border-bottom: 1px solid #2b313a; color: #8b97a8; }
  .pane { padding: 10px; white-space: pre-wrap; word-break: break-word;
          max-height: 52vh; overflow: auto; }
  .entry { border-left: 2px solid #48a868; padding-left: 8px; margin-bottom: 8px; }
  #status { margin-top: 10px; color: #8b97a8; }
  @media (max-width: 760px) { .cols { grid-template-columns: 1fr; } }
</style>
</head>
<body>
<h1>Shadows — Planner</h1>

<div class="row">
  <select id="projects"></select>
  <input id="newProject" placeholder="new project slug">
  <button id="addProject" class="secondary">Add project</button>
</div>

<div class="row">
  <select id="threads"></select>
  <input id="newThread" placeholder="new thread title">
  <button id="addThread" class="secondary">Add thread</button>
</div>

<div class="row">
  <textarea id="prompt" rows="3" style="flex:1 1 420px"
            placeholder="Ask the planner something"></textarea>
</div>
<div class="row">
  <button id="start">Start turn</button>
  <button id="stop" class="secondary" disabled>Stop</button>
</div>

<div class="cols">
  <section>
    <h2>Live — transient, never stored</h2>
    <div class="pane" id="live"></div>
  </section>
  <section>
    <h2>Durable — survives a restart</h2>
    <div class="pane" id="entries"></div>
  </section>
</div>

<div id="status">idle</div>

<script>
const $ = (id) => document.getElementById(id);
let es = null, currentOp = null, lastSeq = 0;
const uuid = () => crypto.randomUUID();

async function api(method, url, body) {
  const res = await fetch(url, {
    method,
    headers: body ? { "content-type": "application/json" } : undefined,
    body: body ? JSON.stringify(body) : undefined,
  });
  if (!res.ok) {
    const err = await res.json().catch(() => ({ message: res.statusText }));
    throw new Error(err.message || res.statusText);
  }
  return res.status === 204 ? null : res.json();
}

async function loadProjects() {
  const list = await api("GET", "/api/projects");
  $("projects").innerHTML = list
    .map((p) => `<option value="${p.id}">${p.slug}</option>`).join("");
  if (list.length) await loadThreads();
}

async function loadThreads() {
  const pid = $("projects").value;
  if (!pid) return;
  const list = await api("GET", `/api/projects/${pid}/threads`);
  $("threads").innerHTML = list
    .map((t) => `<option value="${t.id}">${t.title}</option>`).join("");
  if (list.length) await openThread();
}

// Resume: replay the durable history, then hand off to live without a gap.
async function openThread() {
  const tid = $("threads").value;
  if (!tid) return;
  $("live").textContent = "";
  $("entries").textContent = "";
  lastSeq = 0;

  const entries = await api("GET", `/api/threads/${tid}/entries`);
  for (const e of entries) addEntry(`#${e.ordinal} ${e.author_kind}: ${e.body}`);

  if (es) es.close();
  es = new EventSource(`/api/subscribe?thread_id=${tid}&after=0`);

  es.addEventListener("durable", (ev) => {
    const d = JSON.parse(ev.data);
    lastSeq = Math.max(lastSeq, d.seq);
  });
  es.addEventListener("caught-up", (ev) => {
    lastSeq = Math.max(lastSeq, Number(ev.data));
    $("status").textContent = `caught up at seq ${lastSeq}`;
  });
  es.addEventListener("delta", (ev) => {
    $("live").textContent += JSON.parse(ev.data).text;
  });
  es.addEventListener("entry", (ev) => {
    const d = JSON.parse(ev.data);
    addEntry(`${d.role}: ${d.text}`);
  });
  es.addEventListener("turn-end", (ev) => {
    const d = JSON.parse(ev.data);
    $("status").textContent = `turn ended: ${d.subtype} / ${d.stop_reason}`;
    setRunning(false);
  });
  es.addEventListener("meta", (ev) => {
    const d = JSON.parse(ev.data);
    if (d.label.startsWith("system/api_retry")) {
      $("status").textContent = "harness is retrying upstream...";
    }
  });
}

function addEntry(text) {
  const d = document.createElement("div");
  d.className = "entry";
  d.textContent = text;
  $("entries").appendChild(d);
  $("entries").scrollTop = $("entries").scrollHeight;
}

function setRunning(running) {
  $("start").disabled = running;
  $("stop").disabled = !running;
}

$("addProject").onclick = async () => {
  const slug = $("newProject").value.trim();
  if (!slug) return;
  await api("POST", "/api/projects", { command_id: uuid(), slug, name: slug });
  $("newProject").value = "";
  await loadProjects();
};

$("addThread").onclick = async () => {
  const title = $("newThread").value.trim();
  if (!title) return;
  await api("POST", `/api/projects/${$("projects").value}/threads`,
            { command_id: uuid(), title });
  $("newThread").value = "";
  await loadThreads();
};

$("projects").onchange = loadThreads;
$("threads").onchange = openThread;

$("start").onclick = async () => {
  const prompt = $("prompt").value.trim();
  if (!prompt) return;
  $("live").textContent = "";
  setRunning(true);
  $("status").textContent = "starting...";
  const res = await api("POST", `/api/threads/${$("threads").value}/turns`, { prompt });
  currentOp = res.operation_id;
  $("status").textContent = `operation ${currentOp} accepted`;
};

// Stop goes through the real backend. The button going grey is not the proof;
// the operation reaching Cancelled is.
$("stop").onclick = async () => {
  if (!currentOp) return;
  $("status").textContent = "stopping...";
  const op = await api("POST", `/api/operations/${currentOp}/stop`);
  $("status").textContent = `operation ${op.id} is ${op.status_kind}`;
  setRunning(false);
};

loadProjects().catch((e) => { $("status").textContent = e.message; });
</script>
</body>
</html>
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --test web_client`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/protocol/index.html tests/web_client.rs
git commit -m "feat(protocol): embed the whole web client as one page"
```

---

## Task 13: The Windows acceptance run, and an honest Linux gap report

**Files:**
- Create: `docs/evidence/milestone0/ACCEPTANCE.md`
- Modify: `docs/status.md`

**Interfaces:**
- Consumes: everything.
- Produces: the record that decides whether Milestone 0 is done.

**Spec §11.1: the milestone is incomplete until the user can operate this path from a browser.** A green test suite is not the deliverable.

- [ ] **Step 1: Run the full gate suite and record the exact output**

Run each bare, and read its exit code. Never pipe a gate through `tail` — the shell reports the pipeline's status, not the command's.

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```

Record all three exit codes verbatim in the acceptance document.

- [ ] **Step 2: Run the product path by hand, on Windows**

```bash
cargo run --release -- serve --db ./shadows.sqlite3 --harness "C:/Users/<you>/.local/bin/claude.exe"
```

Then, in a browser you open yourself, walk every line of the §11.1 checklist and write down what actually happened:

```text
[ ] `shadows serve` starts and prints one local address without opening a browser
[ ] the user can manually open the Web client in any browser
[ ] a local-directory project can be selected without exposing a path as project identity
[ ] a PlanningThread can be created and resumed
[ ] one real Claude Planner turn starts and streams output
[ ] Stop terminates and reaps the managed process tree before durable Cancelled
[ ] daemon restart restores the durable thread and terminal operation
[ ] structured logs correlate project, thread, and operation without sensitive payloads
[ ] the exact Windows acceptance run is recorded; Linux gaps are named honestly
```

- [ ] **Step 3: Prove Stop with an independent observation**

The UI reporting `Cancelled` is not proof. While a turn is streaming, note the child pid from the logs, press Stop, then check the tree independently:

```bash
powershell -NoProfile -Command "Get-CimInstance Win32_Process | Where-Object { $_.ProcessId -eq <pid> -or $_.ParentProcessId -eq <pid> } | Select-Object ProcessId,ParentProcessId,Name"
```

Expected: empty. Then confirm the durable state agrees:

```bash
sqlite3 shadows.sqlite3 "SELECT id, status_kind, cancel_requested_at, finished_at FROM operation ORDER BY created_at DESC LIMIT 1;"
```

Expected: `Cancelled`, with both timestamps set. A database flip without the empty process list is not acceptance — spec §2.3 requires confirmed termination, and checking only the database is how the previous project shipped guarantees it did not have.

- [ ] **Step 4: Prove recovery by killing the daemon mid-turn**

Start a turn, then terminate the daemon uncleanly (not Ctrl-C — that is the graceful path, which is a different test). Restart it and check:

```bash
sqlite3 shadows.sqlite3 "SELECT status_kind, interrupt_reason FROM operation ORDER BY created_at DESC LIMIT 1;"
```

Expected: `Interrupted`, with `PreviousRuntimeEndedDuringRun`. Then reopen the browser and confirm the thread's entries are all still there, in ordinal order.

- [ ] **Step 5: Write `docs/evidence/milestone0/ACCEPTANCE.md`**

Follow the shape of the existing evidence reports: environment table with exact versions, what was run, what was observed, then what was NOT established. Required content:

- the three gate exit codes, verbatim;
- each §11.1 checklist line marked pass or fail with what was observed, not with an adjective;
- the Stop proof: the process listing before and after, and the durable row;
- the recovery proof: the interrupted row and the surviving entries;
- **the Linux gap, named exactly.** Spec §1.5's containment contract has a Linux half that Milestone 0 does not implement: `containment_unix.rs` handles deliberate termination through a process group, and spec §1.5 explicitly refuses to accept a process group as proof that descendants die when the daemon crashes. Say that plainly. Do not report the Linux column as passing because the test file compiles.
- anything else observed and not explained.

- [ ] **Step 6: Update `docs/status.md`**

Move Milestone 0 out of "Next" and into "What has been measured", pointing at the acceptance record. Add any newly discovered gap to "Standing risks". Do not restate the spec.

- [ ] **Step 7: Commit**

```bash
git add docs/evidence/milestone0/ACCEPTANCE.md docs/status.md
git commit -m "docs: record the Milestone 0 Windows acceptance run and the Linux gap"
```

---

## Self-Review

Run against the spec after the plan is written, before execution starts.

**Spec coverage.** Every §11.1 acceptance line maps to a task: serve and no browser → Task 1 and Task 13; local project without path-as-identity → Task 5; thread create and resume → Task 6 and Task 12; a real Claude turn streaming → Tasks 7, 8, 9, 11; Stop with confirmed reaping → Tasks 7 and 10; restart recovery → Task 4; structured logs → Task 1 and §8.7 discipline in Tasks 9–11; the recorded acceptance run → Task 13. The seven implementation-order steps of §11.1 map onto Tasks 1–13 in order.

**Deliberate exclusions, restated so no one implements them by accident.** No `workflow/`, `scheduler/`, `execution/`, `verification/`, `mcp/`, or `secrets/`. No FTS5. No `decision`, `research_artifact`, `workflow`, `task`, `task_parent`, `gate`, `verification_check`, `agent_invocation`, `verification_run`, or `verdict` tables. No protocol versioning — the spec has none, and adding it before a second client exists repeats a specific failure of the previous project. No `Context Compiler`; Milestone 0 sends the prompt and lets the harness hold the conversation.

**Known incompleteness, stated rather than hidden.**

1. **Linux containment is not implemented.** `containment_unix.rs` covers deliberate termination only. The parent-death half of §1.5 is absent, and `tests/containment.rs` will expose that on Linux. Task 7 Step 7 says not to gate the test off to make the suite green. Milestone 0 claims Windows.
2. **`agent_invocation` is not persisted.** §8.2 requires the invocation be frozen durably at claim time, and its table is not in the milestone schema. In Milestone 0 the invocation is in-memory only, so a restart loses which model and flags produced a turn. This is a real gap against §8.2 and it is the first thing the next milestone should close.
3. **The harness session id is not persisted.** Task 10 generates one per turn and does not store it, so `--resume` continuity works within a running daemon but not across a restart. Spec §2.11 permits this — a missing native session must never make durable continuity impossible, and the `ThreadEntry` log is the durable record — but the user will notice the model losing its own context after a restart. Closing it needs a column that belongs with `agent_invocation`.

**Type consistency.** `Storage` methods return `Result<_, StorageError>` throughout; `protocol` maps that to HTTP in exactly one place. `StreamItem` has the same five variants in Task 8, Task 10, and Task 11. `FailureStage` is `Prepare | Spawn | Run` everywhere. `EventCursor(i64)` is used identically in Tasks 3 and 11. `StopKind` is `Graceful | Escalated` in Task 4 and Task 11. `now()` has one definition, in `storage/sqlite/mod.rs`.

---

*End of plan.*
