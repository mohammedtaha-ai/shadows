//! Design service HTTP translation.
use super::{AppState, Failure, failure::ErrorBody};
use axum::{
    Json,
    extract::{Path, Query, State},
};
use shadows_core::{
    DesignChange, OutcomeId, OutcomePage, PartId, PartPage, ProjectId, StageView, StructureWritten,
    VisionView,
};

#[derive(serde::Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub(super) struct OutcomesQuery {
    parent: Option<OutcomeId>,
    after: Option<OutcomeId>,
}

#[utoipa::path(
    get,
    path = "/api/projects/{id}/design/outcomes",
    tag = "design",
    params(("id" = ProjectId, Path, description = "The project"), OutcomesQuery),
    responses(
        (status = 200, body = OutcomePage),
        (status = 404, body = ErrorBody),
        (status = 422, body = ErrorBody),
        (status = 500, body = ErrorBody),
    )
)]
pub(super) async fn outcomes(
    State(s): State<AppState>,
    Path(project): Path<ProjectId>,
    Query(q): Query<OutcomesQuery>,
) -> Result<Json<OutcomePage>, Failure> {
    Ok(Json(
        s.core
            .design()
            .outcomes(&project, q.parent.as_ref(), q.after.as_ref())
            .await?,
    ))
}

#[derive(serde::Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub(super) struct PartsQuery {
    parent: Option<PartId>,
    after: Option<PartId>,
}

#[utoipa::path(
    get,
    path = "/api/projects/{id}/design/parts",
    tag = "design",
    params(("id" = ProjectId, Path, description = "The project"), PartsQuery),
    responses(
        (status = 200, body = PartPage),
        (status = 404, body = ErrorBody),
        (status = 422, body = ErrorBody),
        (status = 500, body = ErrorBody),
    )
)]
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

#[utoipa::path(
    get,
    path = "/api/projects/{id}/design/vision",
    tag = "design",
    params(("id" = ProjectId, Path, description = "The project")),
    responses(
        (status = 200, body = VisionView),
        (status = 404, body = ErrorBody),
        (status = 500, body = ErrorBody),
    )
)]
pub(super) async fn vision(
    State(s): State<AppState>,
    Path(project): Path<ProjectId>,
) -> Result<Json<VisionView>, Failure> {
    Ok(Json(s.core.design().vision(&project).await?))
}

#[utoipa::path(get, path = "/api/projects/{id}/vision-proposals", tag = "design",
    params(("id" = ProjectId, Path, description = "The project")),
    responses((status = 200, body = Vec<shadows_core::VisionProposal>),
        (status = 404, body = ErrorBody), (status = 500, body = ErrorBody)))]
pub(super) async fn vision_proposals(
    State(s): State<AppState>,
    Path(project): Path<ProjectId>,
) -> Result<Json<Vec<shadows_core::VisionProposal>>, Failure> {
    Ok(Json(s.core.design().vision_proposals(&project).await?))
}

#[derive(serde::Deserialize, utoipa::ToSchema)]
pub(super) struct VisionApprove {
    pub(super) command_id: String,
}

#[utoipa::path(get,path="/api/projects/{id}/map-proposals",tag="design",
    params(("id" = ProjectId, Path)),responses((status=200,body=Vec<shadows_core::MapProposal>),
        (status=404,body=ErrorBody),(status=500,body=ErrorBody)))]
pub(super) async fn map_proposals(
    State(s): State<AppState>,
    Path(p): Path<ProjectId>,
) -> Result<Json<Vec<shadows_core::MapProposal>>, Failure> {
    Ok(Json(s.core.design().map_proposals(&p).await?))
}

#[utoipa::path(post,path="/api/projects/{id}/map-proposals/{proposal_id}/approve",tag="design",
    request_body=VisionApprove,params(("id" = ProjectId,Path),("proposal_id" = String,Path)),
    responses((status=200,body=DesignChange),(status=404,body=ErrorBody),
        (status=409,body=ErrorBody),(status=422,body=ErrorBody),(status=500,body=ErrorBody)))]
pub(super) async fn approve_map_proposal(
    State(s): State<AppState>,
    Path((p, id)): Path<(ProjectId, String)>,
    Json(body): Json<VisionApprove>,
) -> Result<Json<DesignChange>, Failure> {
    Ok(Json(
        s.core
            .design()
            .approve_map_proposal(body.command_id, &p, &id)
            .await?,
    ))
}

#[utoipa::path(post, path = "/api/projects/{id}/vision-proposals/{proposal_id}/approve",
    tag = "design", request_body = VisionApprove,
    params(("id" = ProjectId, Path, description = "The project"),
        ("proposal_id" = String, Path, description = "The exact immutable proposal")),
    responses((status = 200, body = DesignChange), (status = 404, body = ErrorBody),
        (status = 409, body = ErrorBody), (status = 422, body = ErrorBody),
        (status = 500, body = ErrorBody)))]
pub(super) async fn approve_vision_proposal(
    State(s): State<AppState>,
    Path((project, proposal)): Path<(ProjectId, String)>,
    Json(body): Json<VisionApprove>,
) -> Result<Json<DesignChange>, Failure> {
    Ok(Json(
        s.core
            .design()
            .approve_vision_proposal(body.command_id, &project, &proposal)
            .await?,
    ))
}

#[utoipa::path(get, path = "/api/projects/{id}/stage", tag = "projects",
    params(("id" = ProjectId, Path, description = "The project")),
    responses((status = 200, body = StageView),
    (status = 404, body = ErrorBody), (status = 500, body = ErrorBody)))]
pub(super) async fn get_stage(
    State(s): State<AppState>,
    Path(project): Path<ProjectId>,
) -> Result<Json<StageView>, Failure> {
    Ok(Json(s.core.design().stage(&project).await?))
}

#[derive(serde::Deserialize, utoipa::ToSchema)]
pub(super) struct CreateStructure {
    command_id: String,
    /// The map revision the person confirmed, from the stage read.
    map_revision: i64,
}

#[utoipa::path(post, path = "/api/projects/{id}/structure", tag = "design",
    params(("id" = ProjectId, Path, description = "The project")),
    request_body = CreateStructure,
    responses((status = 200, body = StructureWritten), (status = 404, body = ErrorBody),
        (status = 409, body = ErrorBody,
            description = "STAGE_BLOCKED, COMMAND_CONFLICT, or REVISION_CONFLICT with \
            `current_revision` when the map moved after `map_revision`"),
        (status = 422, body = ErrorBody), (status = 500, body = ErrorBody)))]
pub(super) async fn create_structure(
    State(s): State<AppState>,
    Path(project): Path<ProjectId>,
    Json(body): Json<CreateStructure>,
) -> Result<Json<StructureWritten>, Failure> {
    Ok(Json(
        s.core
            .design()
            .create_structure(body.command_id, &project, body.map_revision)
            .await?,
    ))
}
