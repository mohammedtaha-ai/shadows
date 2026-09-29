//! One job: the directories on this machine a project may own.
//!
//! Spec §1: only the daemon can see the machine's disk, so the check that a
//! path names a real directory, and the form in which it is stored and shown,
//! is made here, once, before anything records it. Nothing here knows about
//! HTTP; `protocol/` maps [`DirectoryError`] to a status and a stable code.

use std::io;
use std::path::{Path, PathBuf};

/// Why a path cannot serve as, or lead to, a project directory. Each variant
/// is a distinct fact a client can act on, which is why they are not folded
/// into one "bad path".
#[derive(Debug, thiserror::Error)]
pub enum DirectoryError {
    #[error("the path must be absolute, got `{0}`")]
    NotAbsolute(String),
    #[error("the path is not valid UTF-8, so it cannot be shown to a client or stored")]
    NotUtf8,
    #[error("no such path: `{0}`")]
    NotFound(String),
    #[error("not a directory: `{0}`")]
    NotADirectory(String),
    #[error("access denied: `{0}`")]
    AccessDenied(String),
    #[error("`{name}` is not a usable directory name: {why}")]
    InvalidName { name: String, why: &'static str },
    #[error("already exists: `{0}`")]
    AlreadyExists(String),
    #[error("`{path}` could not be read: {source}")]
    Unavailable { path: String, source: io::Error },
}

impl DirectoryError {
    /// The operating system's answer, kept as the fact it states rather than
    /// flattened: "does not exist" and "may not look" ask different things of
    /// the person choosing a folder.
    pub(super) fn from_io(path: &Path, error: io::Error) -> Self {
        let shown = path.display().to_string();
        match error.kind() {
            io::ErrorKind::NotFound => Self::NotFound(shown),
            io::ErrorKind::PermissionDenied => Self::AccessDenied(shown),
            io::ErrorKind::NotADirectory => Self::NotADirectory(shown),
            io::ErrorKind::AlreadyExists => Self::AlreadyExists(shown),
            _ => Self::Unavailable {
                path: shown,
                source: error,
            },
        }
    }
}

/// A directory a project owns: absolute, canonical, UTF-8, and a directory at
/// the moment it was checked. The only way to build one is [`resolve`], so a
/// value of this type is proof the check ran.
///
/// Canonical because two spellings of one folder (`C:/Work/`, `c:\work`) must
/// be one directory, both for the person reading the project list and for the
/// idempotency fingerprint, which is taken over this form. On Windows the
/// `\\?\` prefix `std::fs::canonicalize` adds is removed wherever it is not
/// needed — see `dunce` in `Cargo.toml`.
///
/// It says nothing about *later*: a directory can be deleted after it was
/// chosen, which is why a turn checks again before it spawns (spec §8.3).
///
/// [`resolve`]: ProjectDirectory::resolve
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectDirectory(String);

impl ProjectDirectory {
    pub fn resolve(raw: &Path) -> Result<Self, DirectoryError> {
        Ok(Self(utf8(canonical_dir(raw)?)?))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// `raw`, made canonical, provided it is absolute and names a directory.
pub(super) fn canonical_dir(raw: &Path) -> Result<PathBuf, DirectoryError> {
    if !raw.is_absolute() {
        return Err(DirectoryError::NotAbsolute(raw.display().to_string()));
    }
    let canonical = dunce::canonicalize(raw).map_err(|e| DirectoryError::from_io(raw, e))?;
    let metadata =
        std::fs::metadata(&canonical).map_err(|e| DirectoryError::from_io(&canonical, e))?;
    if !metadata.is_dir() {
        return Err(DirectoryError::NotADirectory(
            canonical.display().to_string(),
        ));
    }
    Ok(canonical)
}

pub(super) fn utf8(path: PathBuf) -> Result<String, DirectoryError> {
    path.into_os_string()
        .into_string()
        .map_err(|_| DirectoryError::NotUtf8)
}
