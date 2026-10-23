//! One job: the failures a store can return.

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
    /// Spec §13.5, §18.2: `expected_revision` is not the resource's current one.
    /// `summary` describes what changed since the expected revision.
    #[error("the resource is at revision {current}; changed since: {summary}")]
    RevisionConflict { current: i64, summary: String },
    #[error("stored JSON is invalid: {0}")]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Database(sqlx::Error),
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
