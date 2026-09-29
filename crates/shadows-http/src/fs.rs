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
//! What the disk says is `Projects`' business (`core.projects()`), which
//! reads it on a blocking thread; these handlers only carry it over HTTP.

use axum::Json;
use axum::extract::{Query, State};
use axum::http::StatusCode;

use super::failure::ErrorBody;
use super::{AppState, Failure};
use shadows_core::{DirectoryEntry, DirectoryListing};

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
    State(s): State<AppState>,
    Query(q): Query<DirsQuery>,
) -> Result<Json<DirectoryListing>, Failure> {
    Ok(Json(s.core.projects().list_dirs(q.path).await?))
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
    State(s): State<AppState>,
    Json(body): Json<CreateDir>,
) -> Result<(StatusCode, Json<DirectoryEntry>), Failure> {
    let entry = s.core.projects().create_dir(body.parent, body.name).await?;
    Ok((StatusCode::CREATED, Json(entry)))
}
