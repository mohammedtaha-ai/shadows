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
    // Consumed by `write_txn` in Task 3. Held here now so the writer
    // connection's lifecycle matches the read pool's from the start.
    #[allow(dead_code)]
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
        Ok(Self {
            read,
            write: Mutex::new(write),
        })
    }

    pub fn reader(&self) -> &SqlitePool {
        &self.read
    }
}
