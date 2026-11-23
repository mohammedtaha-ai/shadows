//! CodeRecord service HTTP translation.
use super::{AppState, Failure, failure::ErrorBody};
use axum::{
    Json,
    extract::{Path, Query, State},
};
use shadows_core::{CoreError, ProjectId, WhereAnswer};

#[derive(serde::Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub(super) struct WhereQuery {
    /// A topic, a path, a symbol, or an element id or name: 1 to 200 characters.
    topic: String,
}

#[utoipa::path(get, path = "/api/projects/{id}/code-where", tag = "design",
    params(("id" = ProjectId, Path, description = "The project"), WhereQuery),
    responses((status = 200, body = WhereAnswer),
        (status = 404, body = ErrorBody), (status = 422, body = ErrorBody),
        (status = 500, body = ErrorBody)))]
pub(super) async fn code_where(
    State(s): State<AppState>,
    Path(project): Path<ProjectId>,
    Query(q): Query<WhereQuery>,
) -> Result<Json<WhereAnswer>, Failure> {
    let answer = s.core.code_record().code_where(&project, &q.topic).await;
    Ok(Json(answer.map_err(CoreError::from)?))
}
