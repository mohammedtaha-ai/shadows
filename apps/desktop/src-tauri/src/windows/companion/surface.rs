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
}

impl Default for SurfacePresence {
    fn default() -> Self {
        Self {
            expression: "idle".into(),
            status: "Choose an agent".into(),
            preview: None,
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
