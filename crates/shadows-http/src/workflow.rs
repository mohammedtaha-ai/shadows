//! One job: the routes over plan versions (spec §13.10) — a project's list, one
//! version's read, and the person's approval.
//!
//! Approval is a person's action in the web client (§13.2); no MCP tool
//! approves. Every handler here is `pub(super)`, named only by the route table.

use axum::Json;
use axum::extract::{Path, Query, State};

use super::failure::ErrorBody;
use super::{AppState, Failure};
use shadows_core::{Approved, Plan, PlanId, PlanListing, PlanVersions, ProjectId, WorkflowId};

#[derive(serde::Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub(super) struct ListPlansQuery {
    /// When true, includes archived plans in the listing (§16.2).
    #[serde(default)]
    archived: bool,
}

/// The project's plans, each by its latest version, in the order
/// they were created. Active plans only, or archived ones too when `archived=true`.
/// An unknown project has none.
#[utoipa::path(
    get,
    path = "/api/projects/{id}/workflows",
    tag = "workflows",
    params(("id" = ProjectId, Path, description = "The project"), ListPlansQuery),
    responses(
        (status = 200, body = Vec<PlanListing>),
        (status = 500, description = "STORAGE_UNAVAILABLE", body = ErrorBody),
    )
)]
pub(super) async fn list_plans(
    State(s): State<AppState>,
    Path(project_id): Path<ProjectId>,
    Query(q): Query<ListPlansQuery>,
) -> Result<Json<Vec<PlanListing>>, Failure> {
    Ok(Json(s.core.plans().list(&project_id, q.archived).await?))
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
        (
            status = 200,
            description = "Approved, or the replay of the same command",
            body = Approved,
        ),
        (status = 404, description = "INVALID_COMMAND: no such plan version", body = ErrorBody),
        (
            status = 409,
            description = "REVISION_CONFLICT, carrying `current_revision`; \
                           WORKFLOW_FROZEN_IMMUTABLE; COMMAND_CONFLICT",
            body = ErrorBody,
        ),
        (
            status = 422,
            description = "WORKFLOW_VALIDATION_FAILED, carrying `problems`",
            body = ErrorBody,
        ),
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

/// One plan with every version, oldest first (§16.10).
#[utoipa::path(
    get,
    path = "/api/plans/{id}",
    tag = "workflows",
    params(("id" = PlanId, Path, description = "The plan")),
    responses(
        (status = 200, body = PlanVersions),
        (status = 404, description = "INVALID_COMMAND: no such plan", body = ErrorBody),
        (status = 500, description = "STORAGE_UNAVAILABLE", body = ErrorBody),
    )
)]
pub(super) async fn get_plan_versions(
    State(s): State<AppState>,
    Path(plan_id): Path<PlanId>,
) -> Result<Json<PlanVersions>, Failure> {
    Ok(Json(s.core.plans().plan(&plan_id).await?))
}

#[derive(serde::Deserialize, utoipa::ToSchema)]
pub(super) struct PlanCommand {
    /// The idempotency key (spec §16.2), scoped to the plan.
    command_id: String,
}

/// Archives a plan (§16.2). An archived plan is read, never written.
/// Archiving an already archived plan answers the plan unchanged.
#[utoipa::path(
    post,
    path = "/api/plans/{id}/archive",
    tag = "workflows",
    params(("id" = PlanId, Path, description = "The plan")),
    request_body = PlanCommand,
    responses(
        (
            status = 200,
            description = "Archived, or the replay of the same command",
            body = PlanVersions,
        ),
        (status = 404, description = "INVALID_COMMAND: no such plan", body = ErrorBody),
        (status = 409, description = "COMMAND_CONFLICT", body = ErrorBody),
        (status = 500, description = "STORAGE_UNAVAILABLE", body = ErrorBody),
    )
)]
pub(super) async fn archive_plan(
    State(s): State<AppState>,
    Path(plan_id): Path<PlanId>,
    Json(body): Json<PlanCommand>,
) -> Result<Json<PlanVersions>, Failure> {
    Ok(Json(
        s.core.plans().archive(body.command_id, &plan_id).await?,
    ))
}

/// Unarchives a plan (§16.2), returning it to `Active`.
/// Unarchiving an active plan answers the plan unchanged.
#[utoipa::path(
    post,
    path = "/api/plans/{id}/unarchive",
    tag = "workflows",
    params(("id" = PlanId, Path, description = "The plan")),
    request_body = PlanCommand,
    responses(
        (
            status = 200,
            description = "Unarchived, or the replay of the same command",
            body = PlanVersions,
        ),
        (status = 404, description = "INVALID_COMMAND: no such plan", body = ErrorBody),
        (status = 409, description = "COMMAND_CONFLICT", body = ErrorBody),
        (status = 500, description = "STORAGE_UNAVAILABLE", body = ErrorBody),
    )
)]
pub(super) async fn unarchive_plan(
    State(s): State<AppState>,
    Path(plan_id): Path<PlanId>,
    Json(body): Json<PlanCommand>,
) -> Result<Json<PlanVersions>, Failure> {
    Ok(Json(
        s.core.plans().unarchive(body.command_id, &plan_id).await?,
    ))
}
