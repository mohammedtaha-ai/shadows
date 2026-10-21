//! The project workspace's public value shapes (§18.2).

pub type DesignRevision = i64;

crate::db::id::newtype_id!(PartId);
crate::db::id::newtype_id!(OutcomeId);

#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    serde::Serialize,
    serde::Deserialize,
    utoipa::ToSchema,
    schemars::JsonSchema,
)]
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

#[derive(
    Debug,
    Clone,
    Default,
    PartialEq,
    Eq,
    serde::Serialize,
    serde::Deserialize,
    utoipa::ToSchema,
    schemars::JsonSchema,
)]
pub struct PartContent {
    pub title: String,
    pub responsibility: String,
    pub design: String,
    pub kind: Option<String>,
    #[serde(default)]
    pub serves: Vec<String>,
    #[serde(default)]
    pub folders: Vec<String>,
    #[serde(default)]
    pub files: Vec<InitialFile>,
    #[serde(default)]
    pub contract: ContractHead,
}

#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    serde::Serialize,
    serde::Deserialize,
    utoipa::ToSchema,
    schemars::JsonSchema,
)]
pub struct InitialFile {
    pub path: String,
    pub content: String,
}

#[derive(
    Debug,
    Clone,
    Default,
    PartialEq,
    Eq,
    serde::Serialize,
    serde::Deserialize,
    utoipa::ToSchema,
    schemars::JsonSchema,
)]
pub struct ContractHead {
    pub header: String,
    pub rules: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct PartWaiver {
    pub part: String,
    pub reason: String,
    pub approved_by: String,
    pub created_at: String,
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
    pub purpose: Vec<VisionItem>,
    pub users: Vec<VisionItem>,
    pub goals: Vec<VisionItem>,
    pub boundaries: Vec<VisionItem>,
    pub technical_direction: Vec<VisionItem>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct VisionItem {
    pub id: String,
    pub text: String,
    pub approved_from: Option<crate::threads::ThreadId>,
    pub revision: i64,
}

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    serde::Serialize,
    serde::Deserialize,
    utoipa::ToSchema,
    schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum VisionField {
    Purpose,
    Users,
    Goals,
    Boundaries,
    TechnicalDirection,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(crate) struct VisionCounters {
    pub purpose: u32,
    pub users: u32,
    pub goals: u32,
    pub boundaries: u32,
    pub technical_direction: u32,
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
        plan: crate::plans::PlanId,
    },
    PlanLinkRemove {
        anchor: DesignAnchor,
        plan: crate::plans::PlanId,
    },
}

/// What a Planner reads before it plans: the vision, and the top-level parts
/// and outcomes, each a first page with its cursor.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct WorkspaceView {
    pub vision: VisionView,
    pub parts: PartPage,
    pub outcomes: OutcomePage,
}
