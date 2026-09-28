//! One job: the disk routes a client uses to choose a project directory.
//!
//! **These routes expose the machine's disk** — every directory name the
//! daemon's user can read, and the power to create directories — to any client
//! allowed to call this daemon. Today that is a loopback bind, the CORS origin
//! list, and `guard.rs` refusing requests other pages make a browser send, and
//! nothing else: there is no authentication. That is sound
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
use super::failure::ErrorBody;
use shadows_core::project::DirectoryError;
use shadows_core::project::browse::{self, DirectoryEntry, DirectoryListing};

#[derive(serde::Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub(super) struct DirsQuery {
    /// Absolute path to list. Absent or empty: the roots to start from.
    #[serde(default)]
    path: Option<String>,
}

/// A directory's immediate subdirectories, or the roots to start from.
///
/// Exposes the machine's directory names to any client the CORS list admits;
/// see spec §1's OPEN block on remote access.
#[utoipa::path(
    get,
    path = "/api/fs/dirs",
    tag = "filesystem",
    params(DirsQuery),
    responses(
        (status = 200, body = DirectoryListing),
        (status = 400, description = "PATH_INVALID, PATH_NOT_A_DIRECTORY", body = ErrorBody),
        (status = 403, description = "PATH_ACCESS_DENIED", body = ErrorBody),
        (status = 404, description = "PATH_NOT_FOUND", body = ErrorBody),
        (status = 500, description = "PATH_UNAVAILABLE", body = ErrorBody),
    )
)]
pub(super) async fn list_dirs(
    Query(q): Query<DirsQuery>,
) -> Result<Json<DirectoryListing>, Failure> {
    let path = q.path.filter(|p| !p.is_empty());
    let listing = blocking(move || browse::list(path.as_deref().map(std::path::Path::new))).await?;
    Ok(Json(listing))
}

#[derive(serde::Deserialize, utoipa::ToSchema)]
pub(super) struct CreateDir {
    /// Absolute path to an existing directory.
    parent: String,
    /// One new path component that Windows would accept.
    name: String,
}

/// Creates one new directory to choose as a project's.
#[utoipa::path(
    post,
    path = "/api/fs/dirs",
    tag = "filesystem",
    request_body = CreateDir,
    responses(
        (status = 201, body = DirectoryEntry),
        (status = 400, description = "PATH_INVALID, PATH_NOT_A_DIRECTORY", body = ErrorBody),
        (status = 403, description = "PATH_ACCESS_DENIED", body = ErrorBody),
        (status = 404, description = "PATH_NOT_FOUND: no such parent", body = ErrorBody),
        (status = 409, description = "PATH_ALREADY_EXISTS", body = ErrorBody),
        (status = 500, description = "PATH_UNAVAILABLE", body = ErrorBody),
    )
)]
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
