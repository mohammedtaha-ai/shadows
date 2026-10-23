//! Persistence of the project design workspace.
mod agreement_binding;
mod agreement_write;
mod agreements;
mod hierarchy;
mod map_approval;
mod map_proposal_read;
mod map_proposal_shape;
mod map_proposals;
mod map_versions;
mod outcome_edit;
mod outcomes;
mod part_edit;
mod parts;
mod proposal_approval;
mod proposal_read;
mod proposals;
mod roadmap_approval;
pub(super) mod roadmap_gate;
mod roadmap_proposal_read;
mod roadmap_proposals;
#[cfg(feature = "test-support")]
mod roadmap_seed;
mod roadmap_shape;
mod stage;
pub(super) mod structure;
mod vision;
mod vision_versions;

use crate::db::Storage;
use std::sync::Arc;

pub(super) struct DesignStore {
    pub(super) storage: Arc<Storage>,
}
