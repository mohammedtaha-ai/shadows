//! Standards service HTTP translation.
use super::{AppState, Failure, failure::ErrorBody};
use axum::{
    Json,
    extract::{Path, State},
};
use shadows_core::{EffectiveStandards, ProjectId, StandardsAdditions, StandardsAdditionsVersion};

#[utoipa::path(get, path = "/api/projects/{id}/standards", tag = "projects",
    params(("id" = ProjectId, Path, description = "The project")),
    responses((status = 200, body = EffectiveStandards),
    (status = 404, body = ErrorBody), (status = 500, body = ErrorBody)))]
pub(super) async fn get_standards(
    State(s): State<AppState>,
    Path(project): Path<ProjectId>,
) -> Result<Json<EffectiveStandards>, Failure> {
    Ok(Json(s.core.standards().standards(&project).await?))
}

#[derive(serde::Deserialize, utoipa::ToSchema)]
pub(super) struct StandardsAdditionsSave {
    command_id: String,
    content: StandardsAdditions,
}

#[utoipa::path(put, path = "/api/projects/{id}/standards/additions", tag = "projects",
    params(("id" = ProjectId, Path, description = "The project")),
    request_body = StandardsAdditionsSave,
    responses((status = 200, body = StandardsAdditionsVersion),
    (status = 404, body = ErrorBody), (status = 409, body = ErrorBody),
    (status = 422, body = ErrorBody), (status = 500, body = ErrorBody)))]
pub(super) async fn save_standards_additions(
    State(s): State<AppState>,
    Path(project): Path<ProjectId>,
    Json(body): Json<StandardsAdditionsSave>,
) -> Result<Json<StandardsAdditionsVersion>, Failure> {
    Ok(Json(
        s.core
            .standards()
            .save_standards_additions(body.command_id, &project, body.content)
            .await?,
    ))
}
