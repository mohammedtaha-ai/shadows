//! Refusals owned by project registration and directory selection.
use core_db::StorageError;

#[derive(Debug, thiserror::Error)]
pub enum ProjectsError {
    #[error(transparent)]
    Directory(#[from] crate::DirectoryError),
    #[error("{what} {id} is not offered")]
    SettingNotOffered {
        what: String,
        id: String,
        detail: Option<String>,
    },
    #[error(transparent)]
    Storage(#[from] StorageError),
}
