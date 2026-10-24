//! One job: a project's Planner instructions, the `Instructions` service.
//! Each save is a new numbered version; the current one is the highest number.
//!
//! `model` holds the type, `store` the queries.

mod model;
mod store;

use std::sync::Arc;

pub use model::InstructionsVersion;
use store::InstructionsStore;

use crate::error::CoreError;
use crate::projects::ProjectId;
use core_db::{CommandContext, Storage};

/// Instructions: its private store over the shared storage handle.
#[derive(Clone)]
pub struct Instructions {
    store: InstructionsStore,
}

impl Instructions {
    pub(crate) fn new(storage: Arc<Storage>) -> Self {
        Self {
            store: InstructionsStore::new(storage),
        }
    }

    /// The project's highest-numbered version, or `None` before its first save.
    /// Turns records it and Harness opens sessions with it.
    pub async fn current(
        &self,
        project: &ProjectId,
    ) -> Result<Option<InstructionsVersion>, CoreError> {
        Ok(self.store.current(project).await?)
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
        let c = CommandContext::user(command_id, "PlannerInstructionsSave", params);
        Ok(self.store.save(&c, project, body).await?)
    }
}
