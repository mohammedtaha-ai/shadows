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
            .busy_timeout(Duration::from_millis(5000));

        let read = SqlitePoolOptions::new()
            .max_connections(8)
            .connect_with(opts.clone())
            .await?;

        sqlx::migrate!("./migrations").run(&read).await?;

        let write = SqliteConnection::connect_with(&opts).await?;

        Ok(Self {
            read,
            write: Mutex::new(write),
        })
    }

    pub fn reader(&self) -> &SqlitePool {
        &self.read
    }

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
