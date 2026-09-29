//! One job: a project's Planner instructions (spec §13.8, §14.4) — the
//! `Instructions` service. Each save is a new numbered version; the current
//! one is the highest number.
//!
//! `model` holds the type, `store` the queries.

mod model;
mod store;

use std::sync::Arc;

pub use model::InstructionsVersion;

use crate::app::user_command;
use crate::db::Storage;
use crate::error::CoreError;
use crate::projects::ProjectId;

/// Instructions: what storage holds.
pub struct Instructions {
    storage: Arc<Storage>,
}

impl Instructions {
    pub(crate) fn new(storage: Arc<Storage>) -> Self {
        Self { storage }
    }

    /// The project's highest-numbered version, or `None` before its first save.
    pub async fn current(
        &self,
        project: &ProjectId,
    ) -> Result<Option<InstructionsVersion>, CoreError> {
        Ok(self.storage.current_planner_instructions(project).await?)
    }

    /// Saves `body` as the project's next version: "PlannerInstructionsSave",
    /// params { "project", "body" }. A replay answers the version it saved.
    pub async fn save(
        &self,
        command_id: String,
        project: &ProjectId,
        body: &str,
    ) -> Result<InstructionsVersion, CoreError> {
        let params = serde_json::json!({ "project": project, "body": body });
        let c = user_command(command_id, "PlannerInstructionsSave", params);
        Ok(self
            .storage
            .save_planner_instructions(&c, project, body)
            .await?)
    }
}
