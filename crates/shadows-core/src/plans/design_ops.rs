//! Wire operations composed across design and plan associations.
use super::PlanId;
use crate::design::{DesignAnchor, OutcomeContent, OutcomeId, PartContent, PartId, VisionField};
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
#[serde(tag = "kind")]
pub enum DesignOp {
    OutcomeCreate {
        id: OutcomeId,
        parent: Option<OutcomeId>,
        before: Option<OutcomeId>,
        content: OutcomeContent,
    },
    OutcomePut {
        id: OutcomeId,
        content: OutcomeContent,
    },
    OutcomeMove {
        id: OutcomeId,
        parent: Option<OutcomeId>,
        before: Option<OutcomeId>,
    },
    OutcomePartPut {
        outcome: OutcomeId,
        part: PartId,
    },
    OutcomePartRemove {
        outcome: OutcomeId,
        part: PartId,
    },
    VisionItemAdd {
        field: VisionField,
        text: String,
    },
    VisionItemPut {
        id: String,
        text: String,
    },
    VisionItemRemove {
        id: String,
    },
    PartCreate {
        id: PartId,
        parent: Option<PartId>,
        before: Option<PartId>,
        content: PartContent,
    },
    PartPut {
        id: PartId,
        content: PartContent,
    },
    PartMove {
        id: PartId,
        parent: Option<PartId>,
        before: Option<PartId>,
    },
    PlanLinkPut {
        anchor: DesignAnchor,
        plan: PlanId,
    },
    PlanLinkRemove {
        anchor: DesignAnchor,
        plan: PlanId,
    },
}

impl From<DesignOp> for crate::design::WorkspaceOp {
    fn from(op: DesignOp) -> Self {
        match op {
            DesignOp::OutcomeCreate {
                id,
                parent,
                before,
                content,
            } => Self::OutcomeCreate {
                id,
                parent,
                before,
                content,
            },
            DesignOp::OutcomePut { id, content } => Self::OutcomePut { id, content },
            DesignOp::OutcomeMove { id, parent, before } => {
                Self::OutcomeMove { id, parent, before }
            }
            DesignOp::OutcomePartPut { outcome, part } => Self::OutcomePartPut { outcome, part },
            DesignOp::OutcomePartRemove { outcome, part } => {
                Self::OutcomePartRemove { outcome, part }
            }
            DesignOp::VisionItemAdd { field, text } => Self::VisionItemAdd { field, text },
            DesignOp::VisionItemPut { id, text } => Self::VisionItemPut { id, text },
            DesignOp::VisionItemRemove { id } => Self::VisionItemRemove { id },
            DesignOp::PartCreate {
                id,
                parent,
                before,
                content,
            } => Self::PartCreate {
                id,
                parent,
                before,
                content,
            },
            DesignOp::PartPut { id, content } => Self::PartPut { id, content },
            DesignOp::PartMove { id, parent, before } => Self::PartMove { id, parent, before },
            DesignOp::PlanLinkPut { anchor, plan } => Self::PlanLinkPut {
                anchor,
                plan: plan.to_string(),
            },
            DesignOp::PlanLinkRemove { anchor, plan } => Self::PlanLinkRemove {
                anchor,
                plan: plan.to_string(),
            },
        }
    }
}
