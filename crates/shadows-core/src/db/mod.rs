//! One job: what belongs to no service (spec §14.4) — the SQLite pool, its
//! write transactions and migrations, the command log, the journal's append,
//! the clock. Private to the crate: an adapter reaches none of it (§14.6).

use std::path::Path;
use std::str::FromStr;
use std::time::Duration;

use futures_core::future::BoxFuture;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};
use sqlx::{Connection, SqliteConnection, SqlitePool};
use tokio::sync::{Mutex, watch};

use crate::plans::{PlanId, Problem, WorkflowId};

mod command;
mod journal;

// Store helpers every service's write shares inside its one transaction (spec
// §14.6): the command log and the journal, which belong to no service.
pub(crate) use command::{classify, record_command};
pub(crate) use journal::append_event;
#[cfg(feature = "test-support")]
pub use journal::append_event_for_test;

pub(crate) const MAX_SEQ: &str = "SELECT MAX(seq) FROM durable_event";

pub(crate) fn now() -> String {
    time::OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .expect("RFC3339 formatting cannot fail")
}

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
    /// Spec §12.6: the thread already ran a turn on its harness, or is a fork.
    #[error("the thread's harness is fixed: it already ran a turn, or it is a fork")]
    HarnessLocked,
    /// Spec §12.7, §12.9: the thread has a turn that has not ended.
    #[error("the thread has a turn running")]
    ThreadBusy,
    /// Spec §12.9: only the last entry of a completed turn is a fork point.
    #[error("only the thread's last entry, written by a completed turn, can be forked from")]
    ForkPointNotSupported,
    /// Spec §13.5: `expected_revision` is not the version's current one.
    /// `summary` joins what every edit since the expected revision did.
    #[error("the plan is at revision {current}; changed since: {summary}")]
    RevisionConflict { current: i64, summary: String },
    /// Spec §13.2: a frozen version never changes.
    #[error("the plan version is frozen; start a new version to change it")]
    WorkflowFrozen,
    /// Spec §13.4: the edit's final state, or the approval, breaks a rule.
    #[error("the plan is not valid: {}", problems(.0))]
    PlanInvalid(Vec<Problem>),
    /// Spec §13.7: the writer's grant is unknown or revoked.
    #[error("the grant is unknown or revoked")]
    GrantInvalid,
    /// Spec §13.6: the plan, thread or draft ref is outside the writer's grant.
    #[error("outside what the grant allows")]
    GrantScope,
    /// Spec §13.9: a task named by a focus or by `plan_show` is not in that
    /// plan version. The text says which.
    #[error("{0}")]
    TaskNotInPlan(String),
    /// Spec §13.6: a Planner's `draft_start` named a source other than the
    /// version it starts from, its thread's latest, which this carries.
    #[error("the source is not the thread's latest version, {0}")]
    NotLatestVersion(WorkflowId),
    /// Spec §16.3: a version after v1 is started without a non-blank
    /// `change_reason`. A request missing a part, not an invalid plan.
    #[error("a new version needs its reason")]
    ReasonMissing,
    /// §16.2: the plan became read only before this write committed.
    #[error("plan {0} is archived; a person can unarchive it")]
    PlanArchived(PlanId),
    /// Spec §4.2: a project that holds a planning thread is not removed.
    #[error("the project has planning threads; it cannot be removed")]
    ProjectHasThreads,
    #[error("stored JSON is invalid: {0}")]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Database(sqlx::Error),
}

fn problems(list: &[Problem]) -> String {
    list.iter()
        .map(|p| p.message.as_str())
        .collect::<Vec<_>>()
        .join("; ")
}

/// A write the schema refused — a second project with a slug already in use —
/// is `Constraint`, which a client can act on; any other database failure is
/// `Database`. Classified by the driver's kind, never by message text.
impl From<sqlx::Error> for StorageError {
    fn from(error: sqlx::Error) -> Self {
        use sqlx::error::ErrorKind;
        match error
            .as_database_error()
            .map(|db| (db.kind(), db.message()))
        {
            Some((
                ErrorKind::UniqueViolation
                | ErrorKind::ForeignKeyViolation
                | ErrorKind::NotNullViolation
                | ErrorKind::CheckViolation,
                message,
            )) => Self::Constraint(message.to_string()),
            _ => Self::Database(error),
        }
    }
}

/// The write connection, plus whether it currently sits inside an open
/// transaction. `txn_open` exists for recovery: if a `write_txn` call panics,
/// or its future is dropped (cancellation — an aborted request, a `select!`,
/// a timeout) while `BEGIN IMMEDIATE` has run but `COMMIT`/`ROLLBACK` has
/// not, `Drop` cannot run the async `ROLLBACK` needed to close it out. The
/// flag survives that failure (it lives behind the same mutex as the
/// connection, so it is never observed half-written), and the next
/// `write_txn` call recovers by rolling back before doing anything else —
/// see `Storage::write_txn`.
struct WriteConn {
    conn: SqliteConnection,
    txn_open: bool,
}

/// Task 3 adds a serialized write connection alongside `read` (spec §6.23,
/// evidence `docs/evidence/persistence/WAL_VALIDATION.md`). `write` is one
/// connection, not a pool, guarded by an async mutex: the evidence found that
/// a deferred `BEGIN` fails a read-then-write transaction with
/// `SQLITE_BUSY_SNAPSHOT` on a lock upgrade, that `busy_timeout` cannot
/// rescue that upgrade, and that a single serialized writer using
/// `BEGIN IMMEDIATE` is the only scenario with zero failures under
/// concurrency (scenarios K and L).
pub struct Storage {
    read: SqlitePool,
    write: Mutex<WriteConn>,
    /// The highest journal sequence known committed. Spec §2.4: live
    /// publication happens after commit, so this is raised only after a
    /// `COMMIT` returns, never inside a transaction. A `watch` rather than a
    /// broadcast because a reader re-reads the journal on every change: two
    /// commits coalesced into one wake-up lose nothing.
    committed: watch::Sender<i64>,
}

impl Storage {
    pub async fn open(db_path: &Path) -> Result<Self, StorageError> {
        let url = format!("sqlite://{}", db_path.to_string_lossy().replace('\\', "/"));
        let opts = SqliteConnectOptions::from_str(&url)
            .map_err(|e| StorageError::Unavailable(e.to_string()))?
            .create_if_missing(true)
            .foreign_keys(true)
            .journal_mode(SqliteJournalMode::Wal)
            .busy_timeout(Duration::from_millis(5000));

        let read = SqlitePoolOptions::new()
            .max_connections(8)
            .connect_with(opts.clone())
            .await?;

        sqlx::migrate!("./migrations").run(&read).await?;

        let write = SqliteConnection::connect_with(&opts).await?;
        let highest: Option<i64> = sqlx::query_scalar(MAX_SEQ).fetch_one(&read).await?;

        Ok(Self {
            read,
            write: Mutex::new(WriteConn {
                conn: write,
                txn_open: false,
            }),
            committed: watch::channel(highest.unwrap_or(0)).0,
        })
    }

    pub fn reader(&self) -> &SqlitePool {
        &self.read
    }

    /// A receiver that changes whenever a write transaction that appended to
    /// the journal has committed. Its value is the highest committed `seq`.
    /// Take it BEFORE reading the journal: a commit that lands during the read
    /// then shows as a change instead of falling between the two.
    pub fn watch_committed(&self) -> watch::Receiver<i64> {
        self.committed.subscribe()
    }

    /// Raises the committed-sequence signal. Called only once `COMMIT` has
    /// returned, with the write lock still held so no writer of this process
    /// commits in between. Spec §2.4: publication is best-effort — a failure
    /// here is logged and never reaches the write's caller, whose transaction
    /// is already durable. Only an increase notifies, so a transaction that
    /// appended nothing wakes nobody.
    async fn publish_committed(&self, conn: &mut SqliteConnection) {
        match sqlx::query_scalar::<_, Option<i64>>(MAX_SEQ)
            .fetch_one(&mut *conn)
            .await
        {
            Ok(Some(seq)) => {
                self.committed.send_if_modified(|current| {
                    let raised = seq > *current;
                    if raised {
                        *current = seq;
                    }
                    raised
                });
            }
            Ok(None) => {}
            Err(error) => tracing::warn!(
                %error,
                "storage.publish_committed_failed: a committed write was not signalled live"
            ),
        }
    }

    /// Every write goes through here. Two invariants, each pinned down by its
    /// own test in `tests/storage_contract.rs`, and by nothing else:
    ///
    /// - The write connection is single, not pooled: this is required so the
    ///   mutex below can serialize every writer this process owns.
    ///   `concurrent_read_then_write_transactions_all_succeed` is the test for
    ///   this — it would fail against a connection pool.
    /// - `BEGIN IMMEDIATE` (plus `busy_timeout`) is required against a writer
    ///   this process does *not* own — a second daemon, a CLI client, or any
    ///   other connection to the same file, which the mutex cannot see.
    ///   `write_txn_waits_out_an_external_writer_holding_begin_immediate` is
    ///   the test for this: its closure reads before it writes, so that,
    ///   against a *deferred* `BEGIN`, an external writer's commit landing in
    ///   between would produce `SQLITE_BUSY_SNAPSHOT` — the lock-upgrade
    ///   failure `busy_timeout` cannot rescue (spec §6.23,
    ///   `docs/evidence/persistence/WAL_VALIDATION.md`). `BEGIN IMMEDIATE`
    ///   avoids that failure entirely by taking the write lock, and waiting
    ///   out `busy_timeout` for it, before the closure's read ever runs.
    ///
    /// Recovery at entry: if the *previous* call left `txn_open` set — it
    /// panicked or was cancelled after `BEGIN IMMEDIATE` but before
    /// `COMMIT`/`ROLLBACK` — this call rolls back before proceeding.
    /// `write_txn_recovers_after_a_panicking_transaction` covers this.
    pub async fn write_txn<F, T>(&self, f: F) -> Result<T, StorageError>
    where
        F: for<'a> FnOnce(&'a mut SqliteConnection) -> BoxFuture<'a, Result<T, StorageError>>,
    {
        use sqlx::Executor;
        let mut guard = self.write.lock().await;

        if guard.txn_open {
            tracing::warn!(
                "write connection had an open transaction on entry (a previous \
                 write_txn panicked or was cancelled); rolling back before proceeding"
            );
            match guard.conn.execute("ROLLBACK").await {
                Ok(_) => guard.txn_open = false,
                Err(e) => {
                    tracing::error!(
                        error = %e,
                        "failed to roll back a recovered transaction; the write \
                         connection may be poisoned"
                    );
                    return Err(e.into());
                }
            }
        }

        // Set before `BEGIN` runs, not after: if this call itself panics or is
        // cancelled while awaiting `BEGIN IMMEDIATE`, the flag must still be
        // there to trigger recovery on the next call, in case the statement
        // took effect on the connection before we lost the chance to observe
        // its result.
        guard.txn_open = true;
        guard.conn.execute("BEGIN IMMEDIATE").await?;

        match f(&mut guard.conn).await {
            Ok(v) => {
                guard.conn.execute("COMMIT").await?;
                guard.txn_open = false;
                self.publish_committed(&mut guard.conn).await;
                Ok(v)
            }
            Err(e) => {
                if let Err(rollback_err) = guard.conn.execute("ROLLBACK").await {
                    tracing::error!(
                        error = %rollback_err,
                        "rollback failed after a write_txn error; the write \
                         connection may be poisoned"
                    );
                } else {
                    guard.txn_open = false;
                }
                Err(e)
            }
        }
    }
}
