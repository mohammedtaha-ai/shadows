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
