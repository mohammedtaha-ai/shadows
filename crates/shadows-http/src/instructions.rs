//! One job: the routes over a project's Planner instructions (spec §13.8,
//! §13.10) — reading the current version, saving the next.
//!
//! Every handler here is `pub(super)`, named only by the route table.

use axum::Json;
use axum::extract::{Path, State};

use super::failure::ErrorBody;
use super::{AppState, Failure};
use shadows_core::{InstructionsVersion, ProjectId};

/// The project's current instructions — its highest-numbered version — or
/// `null` before the first save.
#[utoipa::path(
    get,
    path = "/api/projects/{id}/planner-instructions",
    tag = "projects",
    params(("id" = ProjectId, Path, description = "The project")),
    responses(
        (status = 200, body = Option<InstructionsVersion>),
        (status = 500, description = "STORAGE_UNAVAILABLE", body = ErrorBody),
    )
)]
pub(super) async fn get_instructions(
    State(s): State<AppState>,
    Path(project): Path<ProjectId>,
) -> Result<Json<Option<InstructionsVersion>>, Failure> {
    Ok(Json(s.core.instructions().current(&project).await?))
}

#[derive(serde::Deserialize, utoipa::ToSchema)]
pub(super) struct SaveInstructions {
    /// The idempotency key (spec §13.5), scoped to the project.
    command_id: String,
    /// The instructions, as the person wrote them.
    body: String,
}

/// Saves the project's instructions as its next version; nothing earlier is
/// overwritten. A running Planner is told of the change before its next turn
/// (§13.8), never mid-turn.
#[utoipa::path(
    put,
    path = "/api/projects/{id}/planner-instructions",
    tag = "projects",
    params(("id" = ProjectId, Path, description = "The project")),
    request_body = SaveInstructions,
    responses(
        (
            status = 200,
            description = "Saved, or the replay of the same command",
            body = InstructionsVersion,
        ),
        (status = 404, description = "INVALID_COMMAND: no such project", body = ErrorBody),
        (status = 409, description = "COMMAND_CONFLICT", body = ErrorBody),
        (status = 500, description = "STORAGE_UNAVAILABLE", body = ErrorBody),
    )
)]
pub(super) async fn save_instructions(
    State(s): State<AppState>,
    Path(project): Path<ProjectId>,
    Json(body): Json<SaveInstructions>,
) -> Result<Json<InstructionsVersion>, Failure> {
    Ok(Json(
        s.core
            .instructions()
            .save(body.command_id, &project, &body.body)
            .await?,
    ))
}
