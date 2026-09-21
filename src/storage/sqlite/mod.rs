use std::path::Path;
use std::str::FromStr;
use std::time::Duration;

use futures_core::future::BoxFuture;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};
use sqlx::{Connection, SqliteConnection, SqlitePool};
use tokio::sync::Mutex;

pub(super) mod events;

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

        Ok(Self {
            read,
            write: Mutex::new(WriteConn {
                conn: write,
                txn_open: false,
            }),
        })
    }

    pub fn reader(&self) -> &SqlitePool {
        &self.read
    }

    /// Every write goes through here. Two invariants, each pinned down by its
    /// own test in `tests/storage_contract.rs`:
    ///
    /// - The write connection is single and mutex-guarded, not pooled. That is
    ///   what `concurrent_read_then_write_transactions_all_succeed` regresses:
    ///   it would fail against a connection pool. It would *not* fail against
    ///   a plain deferred `BEGIN`, because by the time any closure runs, the
    ///   mutex has already excluded every other writer this process owns —
    ///   the mutex alone is enough to pass that test.
    /// - `BEGIN IMMEDIATE` takes the write lock up front so no transaction has
    ///   to upgrade mid-flight. That matters against a writer this process
    ///   does *not* own — a second daemon, a CLI client, or any other
    ///   connection to the same file, which the mutex cannot see.
    ///   `write_txn_waits_out_an_external_writer_holding_begin_immediate` is
    ///   the test for that: it holds the write lock from an independent
    ///   connection and checks `write_txn` waits out `busy_timeout` and then
    ///   succeeds rather than failing immediately.
    ///
    /// See spec §6.23 and `docs/evidence/persistence/WAL_VALIDATION.md`.
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
