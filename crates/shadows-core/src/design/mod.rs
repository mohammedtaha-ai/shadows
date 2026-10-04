//! The project design workspace service (§18.10).

mod model;
mod ops;
mod outcomes;
mod parts;
mod store;

use std::sync::Arc;

use crate::{app::user_command, db::Storage, error::CoreError, projects::ProjectId};
pub use model::{
    DesignAnchor, DesignChange, DesignOp, DesignRevision, Outcome, OutcomeContent, OutcomeId,
    OutcomePage, OutcomeView, Part, PartContent, PartId, PartPage, PartView, VisionContent,
    VisionView,
};

pub struct Design {
    storage: Arc<Storage>,
}

impl Design {
    pub(crate) fn new(storage: Arc<Storage>) -> Self {
        Self { storage }
    }

    pub async fn vision(&self, project: &ProjectId) -> Result<VisionView, CoreError> {
        Ok(self.storage.design_vision(project).await?)
    }

    pub async fn edit(
        &self,
        command_id: String,
        project: &ProjectId,
        expected_revision: i64,
        mut ops: Vec<DesignOp>,
    ) -> Result<DesignChange, CoreError> {
        ops::normalize(&mut ops)?;
        let params = serde_json::json!({
            "project": project,
            "expected_revision": expected_revision,
            "ops": ops
        });
        let ctx = user_command(command_id, "DesignEdit", params);
        self.storage
            .edit_design(&ctx, project, expected_revision, ops)
            .await
            .map_err(|error| match error {
                crate::db::StorageError::Constraint(message) => CoreError::Refused {
                    code: crate::ErrorCode::InvalidCommand,
                    message,
                },
                other => other.into(),
            })
    }
}
