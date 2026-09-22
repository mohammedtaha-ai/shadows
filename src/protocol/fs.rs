//! One job: the disk routes a client uses to choose a project directory.
//!
//! **These routes expose the machine's disk** — every directory name the
//! daemon's user can read, and the power to create directories — to any client
//! allowed to call this daemon. Today that is a loopback bind plus the CORS
//! origin list, and nothing else: there is no authentication. That is sound
//! only while every client is on this machine; spec §1's OPEN block on remote
//! access names the trigger that ends it.
//!
//! What the disk says is `project::browse`'s business; these handlers only
//! move it onto a blocking thread and back, because listing a large or slow
//! directory must not stall the async runtime.

use axum::Json;
use axum::extract::Query;
use axum::http::StatusCode;

use super::Failure;
use crate::project::DirectoryError;
use crate::project::browse::{self, DirectoryEntry, DirectoryListing};

#[derive(serde::Deserialize)]
pub(super) struct DirsQuery {
    /// Absolute path to list. Absent or empty: the roots to start from.
    #[serde(default)]
    path: Option<String>,
}

pub(super) async fn list_dirs(
    Query(q): Query<DirsQuery>,
) -> Result<Json<DirectoryListing>, Failure> {
    let path = q.path.filter(|p| !p.is_empty());
    let listing = blocking(move || browse::list(path.as_deref().map(std::path::Path::new))).await?;
    Ok(Json(listing))
}

#[derive(serde::Deserialize)]
pub(super) struct CreateDir {
    /// Absolute path to an existing directory.
    parent: String,
    /// One new path component.
    name: String,
}

pub(super) async fn create_dir(
    Json(body): Json<CreateDir>,
) -> Result<(StatusCode, Json<DirectoryEntry>), Failure> {
    let entry = blocking(move || {
        browse::create_subdirectory(std::path::Path::new(&body.parent), &body.name)
    })
    .await?;
    Ok((StatusCode::CREATED, Json(entry)))
}

async fn blocking<T: Send + 'static>(
    work: impl FnOnce() -> Result<T, DirectoryError> + Send + 'static,
) -> Result<T, DirectoryError> {
    tokio::task::spawn_blocking(work)
        .await
        .map_err(|e| DirectoryError::Unavailable {
            path: String::new(),
            source: std::io::Error::other(e),
        })?
}
