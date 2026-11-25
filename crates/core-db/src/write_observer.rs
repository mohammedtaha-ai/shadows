//! Test-only committed-write observation on one retained SQLite connection.
use super::{Storage, StorageError};
use sqlx::{Sqlite, pool::PoolConnection};

pub struct DatabaseWrites {
    conn: PoolConnection<Sqlite>,
}

impl DatabaseWrites {
    pub async fn observe(storage: &Storage) -> Result<DatabaseWrites, StorageError> {
        Ok(DatabaseWrites {
            conn: storage.reader().acquire().await?,
        })
    }

    pub async fn version(&mut self) -> Result<i64, StorageError> {
        Ok(sqlx::query_scalar("PRAGMA data_version")
            .fetch_one(&mut *self.conn)
            .await?)
    }
}

/// Test-only: the next serialized write stops for good right after its commit, before it
/// is announced, so a test can drop its caller in exactly that gap.
pub struct CommitPause {
    committed: std::sync::Arc<tokio::sync::Notify>,
}

impl CommitPause {
    /// Waits until the paused write has committed.
    pub async fn committed(&self) {
        self.committed.notified().await;
    }
}

impl Storage {
    pub fn pause_after_commit(&self) -> CommitPause {
        let committed = std::sync::Arc::new(tokio::sync::Notify::new());
        *self.pause.lock().expect("commit pause") = Some(committed.clone());
        CommitPause { committed }
    }

    /// Holds a paused write after its commit until its caller is dropped.
    pub(crate) async fn hold_if_paused(&self) {
        let paused = self.pause.lock().expect("commit pause").take();
        if let Some(committed) = paused {
            committed.notify_one();
            std::future::pending::<()>().await;
        }
    }
}
