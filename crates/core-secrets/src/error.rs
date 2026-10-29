use core_db::StorageError;

#[derive(Debug, thiserror::Error)]
pub enum SecretsError {
    #[error("reserved environment name: {0}")]
    NameReserved(String),
    #[error("invalid environment variable name: {0}")]
    NameInvalid(String),
    #[error("invalid environment variable value for {0}")]
    ValueInvalid(String),
    #[error("cannot decrypt {0}; replace it in Settings")]
    Unreadable(String),
    #[error("encrypted environment storage is unavailable")]
    Unavailable,
    #[error("{0}")]
    MasterKeyUnavailable(&'static str),
    #[error(transparent)]
    Storage(#[from] StorageError),
}
