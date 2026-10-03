//! The project vision's public value shapes (§18.2).

pub type DesignRevision = i64;

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
    VisionPut { content: VisionContent },
}
