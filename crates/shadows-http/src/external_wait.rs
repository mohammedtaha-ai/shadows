//! One job: an external connection's hook waiting for its reports over HTTP.
//!
//! The hook carries the connection's bearer, as `/mcp` does, and no browser
//! origin. A missing, unknown or revoked bearer is 401 `GRANT_INVALID`, also when the
//! grant is revoked while the call waits; a project not on the connection is 404.

use std::time::Duration;

use axum::Json;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{IntoResponse, Response};
use shadows_core::{AuthorWait, ErrorCode, Grants, GrantsError, ProjectId, RunsError};

use super::failure::{ErrorBody, RunFailure};
use super::{AppState, Failure};

/// Without `seconds` a call waits five minutes.
const WAIT_DEFAULT_SECS: u32 = 300;
/// One call ends inside the hook's own 600-second timeout.
const WAIT_LIMIT_SECS: u32 = 540;

#[derive(serde::Deserialize, utoipa::ToSchema)]
pub(super) struct TaskWait {
    project_id: ProjectId,
    /// How long to wait; 300 when absent, at most 540.
    seconds: Option<u32>,
    /// The report ids an earlier call handed to its session.
    #[serde(default)]
    ack: Vec<String>,
}

#[utoipa::path(
    post,
    path = "/api/external/task-wait",
    tag = "grants",
    request_body = TaskWait,
    responses(
        (status = 200, body = AuthorWait),
        (status = 401, body = ErrorBody),
        (status = 404, body = ErrorBody),
        (status = 500, body = ErrorBody),
    )
)]
pub(super) async fn task_wait(
    State(s): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<TaskWait>,
) -> Response {
    let authorization = headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok());
    let Some(token) = Grants::bearer(authorization) else {
        return unauthorized();
    };
    let grants = s.core.grants();
    let connection = match grants.authorize_external(token).await {
        Ok(Some(connection)) => connection,
        Ok(None) => return unauthorized(),
        Err(error) => return Failure::from(error).into_response(),
    };
    let grant = match grants
        .external_project(&connection.id, &body.project_id)
        .await
    {
        Ok(grant) => grant,
        Err(GrantsError::GrantInvalid) => return unauthorized(),
        Err(GrantsError::GrantScope) => {
            return Failure::from(shadows_core::StorageError::NotFound("project")).into_response();
        }
        Err(error) => return Failure::from(error).into_response(),
    };
    let seconds = body
        .seconds
        .unwrap_or(WAIT_DEFAULT_SECS)
        .min(WAIT_LIMIT_SECS);
    let bound = Duration::from_secs(seconds.into());
    match s.core.runs().author_wait(&grant, &body.ack, bound).await {
        Ok(wait) => Json(wait).into_response(),
        Err(RunsError::Grants(GrantsError::GrantInvalid)) => unauthorized(),
        Err(error) => RunFailure::from(error).into_response(),
    }
}

fn unauthorized() -> Response {
    tracing::info!("external_wait.unauthorized");
    let body = ErrorBody {
        code: ErrorCode::GrantInvalid,
        message: "the connection is unknown or revoked".into(),
        field: None,
        current_revision: None,
        problems: None,
    };
    (StatusCode::UNAUTHORIZED, Json(body)).into_response()
}
