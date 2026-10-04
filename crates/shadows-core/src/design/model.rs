//! The project workspace's public value shapes (§18.2).

pub type DesignRevision = i64;

crate::id::newtype_id!(PartId);
crate::id::newtype_id!(OutcomeId);

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct OutcomeContent {
    pub title: String,
    pub intended_result: String,
    pub acceptance: Vec<String>,
}
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct Outcome {
    pub id: OutcomeId,
    pub revision: i64,
    pub parent: Option<OutcomeId>,
    pub ordinal: i64,
    pub content: OutcomeContent,
}
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct OutcomeView {
    pub revision: DesignRevision,
    pub outcome: Outcome,
    pub ancestors: Vec<Outcome>,
    pub parts: Vec<PartId>,
    pub plans: Vec<crate::plans::PlanId>,
}
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct OutcomePage {
    pub revision: DesignRevision,
    pub items: Vec<Outcome>,
    pub next: Option<OutcomeId>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct PartContent {
    pub title: String,
    pub responsibility: String,
    pub design: String,
    pub kind: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct Part {
    pub id: PartId,
    pub revision: i64,
    pub parent: Option<PartId>,
    pub ordinal: i64,
    pub content: PartContent,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct PartView {
    pub revision: DesignRevision,
    pub part: Part,
    pub ancestors: Vec<Part>,
    pub plans: Vec<crate::plans::PlanId>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[schema(required = false)]
    pub binding_plans: Vec<crate::plans::BindingParticipant>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct PartPage {
    pub revision: DesignRevision,
    pub items: Vec<Part>,
    pub next: Option<PartId>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
#[serde(tag = "kind", content = "id")]
pub enum DesignAnchor {
    Part(PartId),
    Outcome(OutcomeId),
}

#[derive(
    Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize, utoipa::ToSchema,
)]
pub struct VisionContent {
    pub purpose: String,
    pub users: String,
    pub goals: String,
    pub boundaries: String,
    pub technical_direction: String,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct VisionView {
    #[schema(value_type = i64, minimum = 0)]
    pub revision: DesignRevision,
    pub content: VisionContent,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct DesignChange {
    #[schema(value_type = i64, minimum = 0)]
    pub revision: DesignRevision,
}

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
    VisionPut {
        content: VisionContent,
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
        plan: crate::plans::PlanId,
    },
    PlanLinkRemove {
        anchor: DesignAnchor,
        plan: crate::plans::PlanId,
    },
}
