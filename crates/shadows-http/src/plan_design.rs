//! Plan association HTTP translation for design resources.
use super::{AppState, Failure, failure::ErrorBody};
use axum::{
    Json,
    extract::{Path, State},
};
use shadows_core::{DesignChange, DesignOp, OutcomeId, OutcomeView, PartId, PartView, ProjectId};

#[utoipa::path(
    get,
    path = "/api/projects/{id}/design/outcomes/{outcome}",
    tag = "design",
    params(
        ("id" = ProjectId, Path, description = "The project"),
        ("outcome" = OutcomeId, Path, description = "The outcome"),
    ),
    responses(
        (status = 200, body = OutcomeView),
        (status = 404, body = ErrorBody),
        (status = 500, body = ErrorBody),
    )
)]
pub(super) async fn outcome(
    State(s): State<AppState>,
    Path((project, id)): Path<(ProjectId, OutcomeId)>,
) -> Result<Json<OutcomeView>, Failure> {
    Ok(Json(s.core.plans().design_outcome(&project, &id).await?))
}

#[utoipa::path(
    get,
    path = "/api/projects/{id}/design/parts/{part}",
    tag = "design",
    params(
        ("id" = ProjectId, Path, description = "The project"),
        ("part" = PartId, Path, description = "The part"),
    ),
    responses(
        (status = 200, body = PartView),
        (status = 404, body = ErrorBody),
        (status = 500, body = ErrorBody),
    )
)]
pub(super) async fn part(
    State(s): State<AppState>,
    Path((project, id)): Path<(ProjectId, PartId)>,
) -> Result<Json<PartView>, Failure> {
    Ok(Json(s.core.plans().design_part(&project, &id).await?))
}

#[derive(serde::Deserialize, utoipa::ToSchema)]
pub(super) struct DesignEdit {
    command_id: String,
    expected_revision: i64,
    ops: Vec<DesignOp>,
}

#[utoipa::path(post, path = "/api/projects/{id}/design/edits", tag = "design",
    params(("id" = ProjectId, Path, description = "The project")), request_body = DesignEdit,
    responses((status = 200, body = DesignChange), (status = 404, body = ErrorBody),
    (status = 409, description = "REVISION_CONFLICT or COMMAND_CONFLICT", body = ErrorBody),
    (status = 422, body = ErrorBody), (status = 500, body = ErrorBody)))]
pub(super) async fn edit(
    State(s): State<AppState>,
    Path(project): Path<ProjectId>,
    Json(body): Json<DesignEdit>,
) -> Result<Json<DesignChange>, Failure> {
    Ok(Json(
        s.core
            .plans()
            .edit_design(body.command_id, &project, body.expected_revision, body.ops)
            .await?,
    ))
}
