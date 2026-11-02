//! Values accepted by the native surface, independent of their producer.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SurfacePreview {
    pub id: String,
    pub text: String,
    pub expires_at_ms: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SurfacePresence {
    pub expression: String,
    pub status: String,
    pub preview: Option<SurfacePreview>,
    /// Carried to the state's own `waiting` field, never serialized under `presence`.
    #[serde(skip)]
    pub waiting: SurfaceWaiting,
}

impl Default for SurfacePresence {
    fn default() -> Self {
        Self {
            expression: "idle".into(),
            status: "Choose an agent".into(),
            preview: None,
            waiting: Default::default(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct SurfaceAppearance {
    pub color: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outfit: Option<String>,
}

pub struct EditorContext {
    pub scope: Option<String>,
    pub context: Option<String>,
    pub agent: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct SurfaceAgent {
    pub slug: String,
    pub name: String,
    pub description: String,
    pub color: String,
    pub outfit: String,
    pub bound: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize)]
pub struct SurfacePanel {
    pub open: bool,
    pub agents: Vec<SurfaceAgent>,
}
impl SurfacePanel {
    pub fn is_empty(&self) -> bool {
        !self.open && self.agents.is_empty()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SurfaceWaitingOption {
    pub option_id: String,
    pub name: String,
    pub kind: String,
}
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SurfaceWaitingRequest {
    pub thread_id: String,
    pub request_id: String,
    pub title: String,
    pub options: Vec<SurfaceWaitingOption>,
}
/// Requests waiting for the person: the count across AYLUL, the bound conversation's own.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize)]
pub struct SurfaceWaiting {
    pub count: u32,
    pub bound: Vec<SurfaceWaitingRequest>,
}
impl SurfaceWaiting {
    pub fn is_empty(&self) -> bool {
        self.count == 0 && self.bound.is_empty()
    }
}
