//! The project workspace's public value shapes (§18.2).

pub type DesignRevision = i64;

crate::id::newtype_id!(PartId);

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
