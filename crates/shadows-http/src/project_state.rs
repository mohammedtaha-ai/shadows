//! ProjectState service HTTP translation.
use super::{AppState, Failure, failure::ErrorBody};
use axum::{
    Json,
    extract::{Path, Query, State},
};
use shadows_core::{CoreError, ElementView, OutcomeId, PartId, ProjectId, ProjectStateView};

#[derive(serde::Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub(super) struct StateQuery {
    /// One part of the map, answered in full under `part`.
    part_id: Option<PartId>,
    /// One outcome, answered in full under `outcome`; never with `part_id`.
    outcome_id: Option<OutcomeId>,
}

#[utoipa::path(get, path = "/api/projects/{id}/state", tag = "design",
    params(("id" = ProjectId, Path, description = "The project"), StateQuery),
    responses((status = 200, body = ProjectStateView),
        (status = 404, body = ErrorBody), (status = 422, body = ErrorBody),
        (status = 500, body = ErrorBody)))]
pub(super) async fn state(
    State(s): State<AppState>,
    Path(project): Path<ProjectId>,
    Query(q): Query<StateQuery>,
) -> Result<Json<ProjectStateView>, Failure> {
    let view = s
        .core
        .project_state()
        .state_for(&project, q.part_id.as_ref(), q.outcome_id.as_ref())
        .await;
    Ok(Json(view.map_err(CoreError::from)?))
}

#[utoipa::path(get, path = "/api/projects/{id}/elements", tag = "design",
    params(("id" = ProjectId, Path, description = "The project")),
    responses((status = 200, body = Vec<ElementView>),
        (status = 404, body = ErrorBody), (status = 500, body = ErrorBody)))]
pub(super) async fn elements(
    State(s): State<AppState>,
    Path(project): Path<ProjectId>,
) -> Result<Json<Vec<ElementView>>, Failure> {
    let elements = s.core.project_state().elements(&project).await;
    Ok(Json(elements.map_err(CoreError::from)?))
}
