//! One job: the refusals of the code record.

/// Storage fails a verification: a file that cannot be asked about is an
/// unchecked file in the verdict, never an error. A question also fails on
/// its topic, or on Design's refusal (an unknown project).
#[derive(Debug, thiserror::Error)]
pub enum CodeRecordError {
    #[error(transparent)]
    Storage(#[from] core_db::StorageError),
    #[error("{0}")]
    InvalidTopic(String),
    #[error(transparent)]
    Design(#[from] core_design::DesignError),
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
