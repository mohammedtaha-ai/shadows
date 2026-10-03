//! HTTP translation for the project design workspace.
use super::{AppState, Failure, failure::ErrorBody};
use axum::{
    Json,
    extract::{Path, Query, State},
};
use shadows_core::{DesignChange, DesignOp, PartId, PartPage, PartView, ProjectId, VisionView};

#[derive(serde::Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub(super) struct PartsQuery {
    parent: Option<PartId>,
    after: Option<PartId>,
}

#[utoipa::path(get, path = "/api/projects/{id}/design/parts", tag = "design",
    params(("id" = ProjectId, Path, description = "The project"), PartsQuery),
    responses((status = 200, body = PartPage), (status = 404, body = ErrorBody), (status = 422, body = ErrorBody), (status = 500, body = ErrorBody)))]
pub(super) async fn parts(
    State(s): State<AppState>,
    Path(project): Path<ProjectId>,
    Query(q): Query<PartsQuery>,
) -> Result<Json<PartPage>, Failure> {
    Ok(Json(
        s.core
            .design()
            .parts(&project, q.parent.as_ref(), q.after.as_ref())
            .await?,
    ))
}

#[utoipa::path(get, path = "/api/projects/{id}/design/parts/{part}", tag = "design",
    params(("id" = ProjectId, Path, description = "The project"), ("part" = PartId, Path, description = "The part")),
    responses((status = 200, body = PartView), (status = 404, body = ErrorBody), (status = 500, body = ErrorBody)))]
pub(super) async fn part(
    State(s): State<AppState>,
    Path((project, id)): Path<(ProjectId, PartId)>,
) -> Result<Json<PartView>, Failure> {
    Ok(Json(s.core.design().part(&project, &id).await?))
}

#[utoipa::path(get, path = "/api/projects/{id}/design/vision", tag = "design",
    params(("id" = ProjectId, Path, description = "The project")),
    responses((status = 200, body = VisionView), (status = 404, body = ErrorBody), (status = 500, body = ErrorBody)))]
pub(super) async fn vision(
    State(s): State<AppState>,
    Path(project): Path<ProjectId>,
) -> Result<Json<VisionView>, Failure> {
    Ok(Json(s.core.design().vision(&project).await?))
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
            .design()
            .edit(body.command_id, &project, body.expected_revision, body.ops)
            .await?,
    ))
}
