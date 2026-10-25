//! Workspace views composed from Design and Plans in one snapshot.
use super::{BindingParticipant, PlanId, Plans};
use crate::{PlansError, ProjectId, StorageError};
use core_design::{Design, DesignRevision, Outcome, OutcomeId, Part, PartId};

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct PartView {
    pub revision: DesignRevision,
    pub part: Part,
    pub ancestors: Vec<Part>,
    pub plans: Vec<PlanId>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[schema(required = false)]
    pub binding_plans: Vec<BindingParticipant>,
}

impl Plans {
    pub async fn design_part(
        &self,
        project: &ProjectId,
        id: &PartId,
    ) -> Result<PartView, PlansError> {
        let mut tx = self
            .store
            .storage
            .reader()
            .begin()
            .await
            .map_err(StorageError::from)?;
        let details = Design::design_part_in(&mut tx, project, id).await?;
        let plans =
            Self::anchor_plans_in(&mut tx, project, &crate::DesignAnchor::Part(id.clone())).await?;
        Ok(PartView {
            revision: details.revision,
            part: details.part,
            ancestors: details.ancestors,
            plans,
            binding_plans: Self::part_bindings_in(&mut tx, project, id).await?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct OutcomeView {
    pub revision: DesignRevision,
    pub outcome: Outcome,
    pub ancestors: Vec<Outcome>,
    pub parts: Vec<PartId>,
    pub plans: Vec<PlanId>,
}

impl Plans {
    pub async fn design_outcome(
        &self,
        project: &ProjectId,
        id: &OutcomeId,
    ) -> Result<OutcomeView, PlansError> {
        let mut tx = self
            .store
            .storage
            .reader()
            .begin()
            .await
            .map_err(StorageError::from)?;
        let details = Design::design_outcome_in(&mut tx, project, id).await?;
        let plans =
            Self::anchor_plans_in(&mut tx, project, &crate::DesignAnchor::Outcome(id.clone()))
                .await?;
        Ok(OutcomeView {
            revision: details.revision,
            outcome: details.outcome,
            ancestors: details.ancestors,
            plans,
            parts: details.parts,
        })
    }
}
