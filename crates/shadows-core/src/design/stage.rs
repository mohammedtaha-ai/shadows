//! Project stage computed from persisted workspace inputs (§23.4).
//! Structure writing, waivers and drift arrive in subsequent PRs.

use super::VisionContent;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum Stage {
    Idea,
    Vision,
    Map,
    Structure,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct StageView {
    pub stage: Stage,
    /// Missing vision field names at vision; mandatory part names at map.
    pub missing: Vec<String>,
}

pub(crate) fn compute(vision: &VisionContent, kinds: &[String], required: &[&str]) -> StageView {
    let fields = [
        ("purpose", &vision.purpose),
        ("users", &vision.users),
        ("goals", &vision.goals),
        ("boundaries", &vision.boundaries),
        ("technical_direction", &vision.technical_direction),
    ];
    let empty: Vec<_> = fields
        .iter()
        .filter(|(_, text)| text.trim().is_empty())
        .map(|(name, _)| name.to_string())
        .collect();
    if empty.len() == fields.len() {
        return StageView {
            stage: Stage::Idea,
            missing: vec![],
        };
    }
    if !empty.is_empty() {
        return StageView {
            stage: Stage::Vision,
            missing: empty,
        };
    }
    let missing: Vec<_> = required
        .iter()
        .filter(|name| !kinds.iter().any(|kind| kind.trim() == **name))
        .map(|name| name.to_string())
        .collect();
    StageView {
        stage: if missing.is_empty() {
            Stage::Structure
        } else {
            Stage::Map
        },
        missing,
    }
}

pub(crate) fn line(view: &StageView) -> String {
    let missing = view.missing.join(", ");
    match view.stage {
        Stage::Idea => "[Shadows] Stage: idea. Nothing is written yet: discuss the idea; \
                        when the person asks how to start, walk the vision fields one at a time."
            .into(),
        Stage::Vision => format!("[Shadows] Stage: vision. Missing vision fields: {missing}."),
        Stage::Map => format!("[Shadows] Stage: map. Missing mandatory parts: {missing}."),
        Stage::Structure => "[Shadows] Stage: structure. The vision and map are complete; \
             the structure is not written yet."
            .into(),
    }
}
