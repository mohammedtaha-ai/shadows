//! The project design workspace service (§18.10).

mod agreement;
mod agreement_review;
mod agreement_scope;
pub(crate) use agreement_scope::AgreementOrigin;
mod agreement_validation;
mod agreements;
mod model;
mod ops;
mod outcomes;
mod parts;
mod store;
mod workspace;
pub(crate) use store::check_agreement_binding_in;

use std::sync::Arc;

use crate::{app::user_command, db::Storage, error::CoreError, projects::ProjectId};
pub use model::{
    DesignAnchor, DesignChange, DesignOp, DesignRevision, Outcome, OutcomeContent, OutcomeId,
    OutcomePage, OutcomeView, Part, PartContent, PartId, PartPage, PartView, VisionContent,
    VisionView, WorkspaceView,
};

pub struct Design {
    storage: Arc<Storage>,
    handles: Arc<crate::turns::LiveHandles>,
}
pub use agreement::{
    AgreementContent, AgreementId, AgreementIssue, AgreementParty, AgreementRole, AgreementState,
    AgreementVersion, AgreementWriter,
};
pub use agreement_review::{AgreementParticipantImpact, AgreementPartyReview, AgreementReview};

impl Design {
    pub(crate) fn new(storage: Arc<Storage>, handles: Arc<crate::turns::LiveHandles>) -> Self {
        Self { storage, handles }
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
