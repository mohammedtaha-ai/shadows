//! One job: the routes over plan versions (spec §13.10) — a project's list, one
//! version's read, and the person's approval.
//!
//! Approval is a person's action in the web client (§13.2); no MCP tool
//! approves. Every handler here is `pub(super)`, named only by the route table.

use axum::Json;
use axum::extract::{Path, State};

use super::failure::ErrorBody;
use super::{AppState, Failure};
use shadows_core::ProjectId;
use shadows_core::{Approved, Plan, PlanListing, WorkflowId};

/// The project's Active plans, each by its latest version, in the order
/// they were created. An unknown project has none.
#[utoipa::path(
    get,
    path = "/api/projects/{id}/workflows",
    tag = "workflows",
    params(("id" = ProjectId, Path, description = "The project")),
    responses(
        (status = 200, body = Vec<PlanListing>),
        (status = 500, description = "STORAGE_UNAVAILABLE", body = ErrorBody),
    )
)]
pub(super) async fn list_plans(
    State(s): State<AppState>,
    Path(project_id): Path<ProjectId>,
) -> Result<Json<Vec<PlanListing>>, Failure> {
    Ok(Json(s.core.plans().list(&project_id, false).await?))
}

/// One plan version: its tasks, links and revision, the versions before and
/// after it, what blocks its approval, and what the last edit changed.
#[utoipa::path(
    get,
    path = "/api/workflows/{id}",
    tag = "workflows",
    params(("id" = WorkflowId, Path, description = "The plan version")),
    responses(
        (status = 200, body = Plan),
        (status = 404, description = "INVALID_COMMAND: no such plan version", body = ErrorBody),
        (status = 500, description = "STORAGE_UNAVAILABLE", body = ErrorBody),
    )
)]
pub(super) async fn get_plan(
    State(s): State<AppState>,
    Path(workflow): Path<WorkflowId>,
) -> Result<Json<Plan>, Failure> {
    Ok(Json(s.core.plans().get(&workflow).await?))
}

#[derive(serde::Deserialize, utoipa::ToSchema)]
pub(super) struct ApprovePlan {
    /// The idempotency key (spec §13.5), scoped to the plan version.
    command_id: String,
    /// The revision the person saw; approval of any other is refused.
    expected_revision: i64,
}

/// Approves a `Draft` version as the person saw it: it becomes `Frozen` and
/// never changes again (§13.2). The client reads the plan again afterwards. A
/// replay of the same command answers what it answered first.
#[utoipa::path(
    post,
    path = "/api/workflows/{id}/approve",
    tag = "workflows",
    params(("id" = WorkflowId, Path, description = "The plan version")),
    request_body = ApprovePlan,
    responses(
        (status = 200, description = "Approved, or the replay of the same command", body = Approved),
        (status = 404, description = "INVALID_COMMAND: no such plan version", body = ErrorBody),
        (status = 409, description = "REVISION_CONFLICT, carrying `current_revision`; \
                                      WORKFLOW_FROZEN_IMMUTABLE; COMMAND_CONFLICT", body = ErrorBody),
        (status = 422, description = "WORKFLOW_VALIDATION_FAILED, carrying `problems`", body = ErrorBody),
        (status = 500, description = "STORAGE_UNAVAILABLE", body = ErrorBody),
    )
)]
pub(super) async fn approve_plan(
    State(s): State<AppState>,
    Path(workflow): Path<WorkflowId>,
    Json(body): Json<ApprovePlan>,
) -> Result<Json<Approved>, Failure> {
    Ok(Json(
        s.core
            .plans()
            .approve(body.command_id, &workflow, body.expected_revision)
            .await?,
    ))
}
