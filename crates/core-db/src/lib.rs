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

/// One serialized BEGIN IMMEDIATE writer plus a WAL reader pool.
pub struct Storage {
    read: SqlitePool,
    write: Mutex<Option<SqliteConnection>>,
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
            write: Mutex::new(Some(write)),
            committed: watch::channel(highest.unwrap_or(0)).0,
        })
    }

    pub fn reader(&self) -> &SqlitePool {
        &self.read
    }

    /// Releases every database connection before its directory can be removed.
    pub async fn close(&self) -> Result<(), StorageError> {
        let mut writer = self.write.lock().await;
        let close_writer = async {
            if let Some(connection) = writer.take() {
                connection.close().await?;
            }
            Ok::<_, StorageError>(())
        };
        let (_, result) = tokio::join!(self.read.close(), close_writer);
        result
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
        let mut guard = self.write.lock().await;
        let guard = guard
            .as_mut()
            .ok_or_else(|| StorageError::Unavailable("storage is closed".into()))
            .map_err(E::from)?;
        // SQLx queues rollback when the guard is dropped, including cancellation.
        // Its worker also cleans up BEGIN if the waiting future disappears.
        let mut transaction = guard
            .begin_with("BEGIN IMMEDIATE")
            .await
            .map_err(StorageError::from)?;

        match f(&mut transaction).await {
            Ok(v) => {
                transaction.commit().await.map_err(StorageError::from)?;
                self.publish_committed(guard).await;
                Ok(v)
            }
            Err(e) => {
                if let Err(rollback_err) = transaction.rollback().await {
                    tracing::error!(
                        error = %rollback_err,
                        "rollback failed after a write_txn error; the write \
                         connection may be poisoned"
                    );
                }
                Err(e)
            }
        }
    }
}
