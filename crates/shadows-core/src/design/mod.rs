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
pub(crate) mod stage;
pub(crate) mod standards;
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
pub use stage::{Stage, StageView};
pub use standards::{
    AdditionalPart, BaseStandards, ContractTemplate, EffectiveStandards, MandatoryPart,
    StandardRule, StandardsAdditions, StandardsAdditionsVersion, base as base_standards,
};

impl Design {
    pub async fn standards(&self, project: &ProjectId) -> Result<EffectiveStandards, CoreError> {
        Ok(EffectiveStandards {
            base: standards::base().clone(),
            additions: self.storage.current_standards_additions(project).await?,
        })
    }

    pub async fn save_standards_additions(
        &self,
        command_id: String,
        project: &ProjectId,
        content: StandardsAdditions,
    ) -> Result<StandardsAdditionsVersion, CoreError> {
        let params = serde_json::json!({"project": project, "content": content});
        let ctx = user_command(command_id, "StandardsAdditionsSave", params);
        self.storage
            .save_standards_additions(&ctx, project, &content)
            .await
            .map_err(|error| match error {
                crate::StorageError::Constraint(message) => CoreError::Refused {
                    code: crate::ErrorCode::InvalidCommand,
                    message,
                },
                other => other.into(),
            })
    }

    pub async fn stage(&self, project: &ProjectId) -> Result<StageView, CoreError> {
        Ok(self.storage.project_stage(project).await?)
    }

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
