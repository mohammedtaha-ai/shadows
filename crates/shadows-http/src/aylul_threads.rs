//! Agent conversation listing HTTP translation.
use super::{AppState, Failure, failure::ErrorBody};
use axum::{
    Json,
    extract::{Path, State},
};
use shadows_core::PlanningThread;

#[utoipa::path(get, path="/api/aylul/agents/{slug}/threads", tag="aylul",
    params(("slug"=String,Path)),
    responses((status=200,body=[PlanningThread]),(status=500,body=ErrorBody)))]
pub(super) async fn threads(
    State(s): State<AppState>,
    Path(slug): Path<String>,
) -> Result<Json<Vec<PlanningThread>>, Failure> {
    Ok(Json(s.core.threads().list_for_profile(&slug).await?))
}
