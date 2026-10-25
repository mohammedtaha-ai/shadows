//! The project design workspace service.
use core_db::Storage;

mod agreement;
mod agreement_scope;
pub use agreement_scope::AgreementOrigin;
mod agreement_validation;
mod agreements;
mod map_proposals;
mod map_rules;
pub use map_proposals::{MapAction, MapProposal, MapPropose, ProposedPart, ProposedWaiver};
mod model;
mod ops;
mod outcomes;
mod parts;
mod proposals;
mod roadmap_proposals;
pub use proposals::{ProposalStatus, VisionAction, VisionProposal, VisionPropose};
pub use roadmap_proposals::{ProposedOutcome, RoadmapAction, RoadmapProposal, RoadmapPropose};
pub mod stage;
mod store;
mod structure;
mod structure_create;
mod structure_write;
pub use structure_create::StructureWritten;
mod vision_items;
mod workspace;

use std::sync::Arc;

use core_projects::ProjectId;
pub use model::WorkspaceOp;
pub use model::{
    ContractHead, DesignAnchor, DesignChange, DesignRevision, InitialFile, Outcome, OutcomeContent,
    OutcomeId, OutcomePage, Part, PartContent, PartId, PartPage, PartWaiver, VisionContent,
    VisionField, VisionItem, VisionView, WorkspaceView,
};

#[derive(Clone)]
pub struct Design {
    store: Arc<store::DesignStore>,
    operations: core_operations::Operations,
    structure_lock: Arc<tokio::sync::Mutex<()>>,
}
pub use agreement::{
    AgreementContent, AgreementId, AgreementIssue, AgreementParty, AgreementRole, AgreementState,
    AgreementVersion, AgreementWriter,
};
pub use stage::{Stage, StageView};

impl Design {
    #[cfg(feature = "test-support")]
    pub async fn seed_map_for_tests(&self, project: &ProjectId) -> Result<(), DesignError> {
        Ok(self.store.seed_map_for_tests(project).await?)
    }
    #[cfg(feature = "test-support")]
    pub async fn seed_roadmap_for_tests(&self, project: &ProjectId) -> Result<PartId, DesignError> {
        Ok(self.store.seed_roadmap_for_tests(project).await?)
    }
    pub async fn stage(&self, project: &ProjectId) -> Result<StageView, DesignError> {
        Ok(self.store.project_stage(project).await?)
    }
    /// The stage fact appended to a Planner prompt.
    pub async fn stage_line_in(
        conn: &mut sqlx::SqliteConnection,
        project: &ProjectId,
    ) -> Result<String, core_db::StorageError> {
        Ok(stage::StageRules::line(
            &Self::stage_view_in(conn, project).await?,
        ))
    }

    pub fn new(storage: Arc<Storage>, operations: core_operations::Operations) -> Self {
        Self {
            store: Arc::new(store::DesignStore { storage }),
            operations,
            structure_lock: Arc::new(tokio::sync::Mutex::new(())),
        }
    }

    pub async fn vision(&self, project: &ProjectId) -> Result<VisionView, DesignError> {
        Ok(self.store.design_vision(project).await?)
    }
}

pub use model::PartDetails;

pub use model::OutcomeDetails;

#[cfg(feature = "test-support")]
impl Design {
    pub async fn approve_vision_proposal_for_test(
        &self,
        ctx: &core_db::CommandContext,
        project: &ProjectId,
        id: &str,
        approver: &core_grants::Approver,
    ) -> Result<DesignChange, DesignError> {
        self.store
            .approve_vision_proposal_in(ctx, project, id, approver)
            .await
    }
    pub async fn propose_vision_for_test(
        &self,
        ctx: &core_db::CommandContext,
        writer: &core_grants::Writer,
        project: &ProjectId,
        turn: &crate::OperationId,
        args: VisionPropose,
    ) -> Result<VisionProposal, DesignError> {
        self.store
            .propose_vision_in(ctx, writer, project, turn, args)
            .await
    }
}

mod error;
pub use error::DesignError;

mod proposal_context;

use core_db::StorageError;
use core_grants::Grant;
use core_operations::OperationId;
use core_threads::ThreadId;

#[cfg(feature = "test-support")]
pub use map_rules::{MapPart as TestMapPart, MapRules as TestMapRules};
#[cfg(feature = "test-support")]
pub use stage::StageRules as TestStageRules;
#[cfg(feature = "test-support")]
pub use structure::{RenderInput as TestRenderInput, Structure as TestStructure};

#[cfg(feature = "test-support")]
pub use structure::{Planned as TestPlanned, PlannedFile as TestPlannedFile};
