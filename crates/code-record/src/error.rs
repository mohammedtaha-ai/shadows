//! One job: the refusals of the code record.

/// Only storage fails a verification: a file that cannot be asked about is
/// an unchecked file in the verdict, never an error.
#[derive(Debug, thiserror::Error)]
pub enum CodeRecordError {
    #[error(transparent)]
    Storage(#[from] core_db::StorageError),
}

impl From<sqlx::Error> for CodeRecordError {
    fn from(error: sqlx::Error) -> Self {
        core_db::StorageError::from(error).into()
    }
}

impl From<serde_json::Error> for CodeRecordError {
    fn from(error: serde_json::Error) -> Self {
        core_db::StorageError::from(error).into()
    }
}
