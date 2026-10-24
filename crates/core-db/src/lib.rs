//! Shared SQLite lifetime and serialized persistence.

use std::path::Path;
use std::str::FromStr;
use std::time::Duration;

use futures_core::future::BoxFuture;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};
use sqlx::{Connection, SqliteConnection, SqlitePool};
use tokio::sync::{Mutex, watch};

mod command;
mod derive;
mod embedded_migrations;
mod error;
mod id;
mod journal;
#[cfg(feature = "test-support")]
mod write_observer;

pub use error::StorageError;
#[cfg(feature = "test-support")]
pub use write_observer::DatabaseWrites;

/// The SQLx connection borrowed by one serialized write.
pub type Tx = SqliteConnection;

pub use command::CommandContext;
pub use derive::Anchor;
pub use journal::{Actor, Causation, JournalEntry};

const MAX_SEQ: &str = "SELECT MAX(seq) FROM durable_event";

// The flag survives a dropped write future so the next lock holder can roll back
// before starting another transaction on this same SQLx connection.
struct WriteConn {
    conn: SqliteConnection,
    txn_open: bool,
}

/// One serialized BEGIN IMMEDIATE writer plus a WAL reader pool.
pub struct Storage {
    read: SqlitePool,
    write: Mutex<WriteConn>,
    committed: watch::Sender<i64>,
}

impl Storage {
    pub fn now() -> String {
        time::OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Rfc3339)
            .expect("RFC3339 formatting cannot fail")
    }

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

        embedded_migrations::EmbeddedMigrations::for_database(&read)
            .await?
            .run(&read)
            .await?;

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

    pub fn watch_committed(&self) -> watch::Receiver<i64> {
        self.committed.subscribe()
    }

    // Publish while retaining the write lock: the announced sequence belongs to
    // a completed commit, and concurrent writers cannot overtake its notification.
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

    pub async fn write_txn<F, T>(&self, f: F) -> Result<T, StorageError>
    where
        F: for<'a> FnOnce(&'a mut Tx) -> BoxFuture<'a, Result<T, StorageError>>,
    {
        self.write_txn_as(f).await
    }

    pub async fn write_txn_as<F, T, E>(&self, f: F) -> Result<T, E>
    where
        F: for<'a> FnOnce(&'a mut Tx) -> BoxFuture<'a, Result<T, E>>,
        E: From<StorageError>,
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
                    return Err(StorageError::from(e).into());
                }
            }
        }

        // Set before awaiting BEGIN: cancellation may occur after SQLite accepts
        // the statement but before its result reaches this future.
        guard.txn_open = true;
        guard
            .conn
            .execute("BEGIN IMMEDIATE")
            .await
            .map_err(StorageError::from)?;

        match f(&mut guard.conn).await {
            Ok(v) => {
                guard
                    .conn
                    .execute("COMMIT")
                    .await
                    .map_err(StorageError::from)?;
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
