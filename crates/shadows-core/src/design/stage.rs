//! Project stage computed from persisted workspace inputs.
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
    Roadmap,
    Plans,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct StageView {
    #[serde(default)]
    pub drift: Vec<String>,
    pub stage: Stage,
    /// Missing vision field names at vision; mandatory part names at map.
    pub missing: Vec<String>,
}

impl StageRules {
    pub(crate) fn compute(
        vision: &VisionContent,
        kinds: &[String],
        required: &[&str],
    ) -> StageView {
        let fields = [
            ("purpose", &vision.purpose),
            ("users", &vision.users),
            ("goals", &vision.goals),
            ("boundaries", &vision.boundaries),
            ("technical_direction", &vision.technical_direction),
        ];
        let empty: Vec<_> = fields
            .iter()
            .filter(|(_, items)| items.is_empty())
            .map(|(name, _)| name.to_string())
            .collect();
        if empty.len() == fields.len() {
            return StageView {
                drift: vec![],
                stage: Stage::Idea,
                missing: vec![],
            };
        }
        if !empty.is_empty() {
            return StageView {
                drift: vec![],
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
            drift: vec![],
            stage: if missing.is_empty() {
                Stage::Structure
            } else {
                Stage::Map
            },
            missing,
        }
    }
}

impl StageRules {
    pub(crate) fn line(view: &StageView) -> String {
        let missing = view.missing.join(", ");
        let mut line = match view.stage {
            Stage::Roadmap => "[Shadows] Stage: roadmap. The structure is written; \
            propose the roadmap with roadmap_propose."
                .into(),
            Stage::Plans => "[Shadows] Stage: plans. The roadmap is approved; plan work for \
            its outcomes, each plan tied to the parts it changes."
                .into(),
            Stage::Idea => "[Shadows] Stage: idea. Nothing is written yet: discuss the idea; \
                        when the person asks how to start, walk the vision fields one at a time."
                .into(),
            Stage::Vision => format!("[Shadows] Stage: vision. Missing vision fields: {missing}."),
            Stage::Map => format!("[Shadows] Stage: map. Missing mandatory parts: {missing}."),
            Stage::Structure if !view.missing.is_empty() => format!(
                "[Shadows] Stage: structure. Missing mandatory parts: {missing}; \
             update the approved map before creating structure."
            ),
            Stage::Structure => "[Shadows] Stage: structure. The vision and map are complete; \
             the structure is not written yet."
                .into(),
        };
        if !view.drift.is_empty() {
            line.push_str(&format!(" Drift: {}.", view.drift.join("; ")));
        }
        line
    }
}

pub(in crate::design) struct StageRules;

impl StageRules {
    pub(super) fn with_workspace(
        mut view: StageView,
        map: i64,
        written: bool,
        roadmap: i64,
    ) -> StageView {
        if matches!(view.stage, Stage::Map | Stage::Structure) {
            view.stage = if map == 0 {
                Stage::Map
            } else {
                Stage::Structure
            };
        }
        if written {
            view.stage = if roadmap > 0 {
                Stage::Plans
            } else {
                Stage::Roadmap
            };
        }
        view
    }
}

#[cfg(test)]
mod tests {
    use super::{Stage, StageRules, StageView};

    #[test]
    fn approved_map_reports_new_missing_mandatory_kinds() {
        let rendered = StageRules::line(&StageView {
            stage: Stage::Structure,
            missing: vec!["billing".into()],
            drift: vec![],
        });
        assert!(
            rendered.contains("Missing mandatory parts: billing"),
            "{rendered}"
        );
        assert!(!rendered.contains("map are complete"), "{rendered}");
    }
}
