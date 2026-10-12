//! One job: the routes over a project's roadmap proposals (§23.5), shaped as
//! the map proposal routes in `design.rs` are.
use super::{AppState, Failure, design::VisionApprove, failure::ErrorBody};
use axum::{
    Json,
    extract::{Path, State},
};
use shadows_core::{DesignChange, ProjectId, RoadmapProposal};

#[utoipa::path(get,path="/api/projects/{id}/roadmap-proposals",tag="design",
    params(("id" = ProjectId, Path)),responses((status=200,body=Vec<RoadmapProposal>),
        (status=404,body=ErrorBody),(status=500,body=ErrorBody)))]
pub(super) async fn roadmap_proposals(
    State(s): State<AppState>,
    Path(p): Path<ProjectId>,
) -> Result<Json<Vec<RoadmapProposal>>, Failure> {
    Ok(Json(s.core.design().roadmap_proposals(&p).await?))
}

#[utoipa::path(post,path="/api/projects/{id}/roadmap-proposals/{proposal_id}/approve",
    tag="design",request_body=VisionApprove,
    params(("id" = ProjectId,Path),("proposal_id" = String,Path)),
    responses((status=200,body=DesignChange),(status=404,body=ErrorBody),
        (status=409,body=ErrorBody),(status=422,body=ErrorBody),(status=500,body=ErrorBody)))]
pub(super) async fn approve_roadmap_proposal(
    State(s): State<AppState>,
    Path((p, id)): Path<(ProjectId, String)>,
    Json(body): Json<VisionApprove>,
) -> Result<Json<DesignChange>, Failure> {
    Ok(Json(
        s.core
            .design()
            .approve_roadmap_proposal(body.command_id, &p, &id)
            .await?,
    ))
}
