//! Planning anchor and prepared context HTTP translation.
use super::failure::ErrorBody;
use super::{AppState, Failure};
use axum::Json;
use axum::extract::{Path, State};
use shadows_core::{
    CarriedContext, DesignAnchor, ErrorCode, OperationId, OutcomeId, PartId, ThreadId,
};

#[derive(serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
#[serde(tag = "kind", content = "id", rename_all = "snake_case")]
pub(super) enum ThreadPlanningAnchor {
    Part(PartId),
    Outcome(OutcomeId),
}

impl From<ThreadPlanningAnchor> for DesignAnchor {
    fn from(anchor: ThreadPlanningAnchor) -> Self {
        match anchor {
            ThreadPlanningAnchor::Part(id) => Self::Part(id),
            ThreadPlanningAnchor::Outcome(id) => Self::Outcome(id),
        }
    }
}

impl From<DesignAnchor> for ThreadPlanningAnchor {
    fn from(anchor: DesignAnchor) -> Self {
        match anchor {
            DesignAnchor::Part(id) => Self::Part(id),
            DesignAnchor::Outcome(id) => Self::Outcome(id),
        }
    }
}

#[derive(serde::Deserialize, utoipa::ToSchema)]
pub(super) struct SetAnchor {
    command_id: String,
    #[schema(required)]
    anchor: Option<ThreadPlanningAnchor>,
}

#[derive(serde::Serialize, utoipa::ToSchema)]
pub(super) struct AnchorResponse {
    #[schema(required)]
    anchor: Option<ThreadPlanningAnchor>,
}

#[derive(serde::Serialize, utoipa::ToSchema)]
pub(super) struct ContextResponse {
    carried: CarriedContext,
    heard: bool,
}

#[utoipa::path(get, path = "/api/threads/{id}/anchor", tag = "threads",
    params(("id" = ThreadId, Path)),
    responses((status = 200, body = AnchorResponse), (status = 404, body = ErrorBody)))]
pub(super) async fn get_anchor(
    State(state): State<AppState>,
    Path(thread): Path<ThreadId>,
) -> Result<Json<AnchorResponse>, Failure> {
    let anchor = state.core.design().thread_anchor(&thread).await?;
    Ok(Json(AnchorResponse {
        anchor: anchor.map(Into::into),
    }))
}

#[utoipa::path(put, path = "/api/threads/{id}/anchor", tag = "threads",
    params(("id" = ThreadId, Path)), request_body = SetAnchor,
    responses((status = 200, body = AnchorResponse), (status = 404, body = ErrorBody),
        (status = 409, body = ErrorBody), (status = 422, body = ErrorBody)))]
pub(super) async fn set_anchor(
    State(state): State<AppState>,
    Path(thread): Path<ThreadId>,
    Json(body): Json<SetAnchor>,
) -> Result<Json<AnchorResponse>, Failure> {
    let anchor = super::transport::Transport::detached(async move {
        state
            .core
            .design()
            .set_thread_anchor(body.command_id, &thread, body.anchor.map(Into::into))
            .await
            .map_err(Failure::from)
    })
    .await?;
    Ok(Json(AnchorResponse {
        anchor: anchor.map(Into::into),
    }))
}

#[utoipa::path(get, path = "/api/operations/{id}/context", tag = "threads",
    params(("id" = OperationId, Path)),
    responses((status = 200, body = ContextResponse),
        (status = 404, description = "CONTEXT_NOT_CARRIED", body = ErrorBody)))]
pub(super) async fn context(
    State(state): State<AppState>,
    Path(operation): Path<OperationId>,
) -> Result<Json<ContextResponse>, Failure> {
    let (carried, heard) = state
        .core
        .harness()
        .context_record(&operation)
        .await?
        .ok_or_else(|| {
            Failure::refused(
                ErrorCode::ContextNotCarried,
                "No planning context record is available for this turn.".into(),
            )
        })?;
    Ok(Json(ContextResponse { carried, heard }))
}
