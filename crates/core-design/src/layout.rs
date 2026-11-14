//! Accepted layout values and project-relative path ownership.
use crate::PartId;
use core_standards::LayerRef;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectLayout {
    pub map_revision: i64,
    pub standards_base: i64,
    pub standards_additions: Option<i64>,
    pub parts: Vec<LayoutPart>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct LayoutPart {
    pub id: PartId,
    pub parent: Option<PartId>,
    pub title: String,
    pub kind: Option<String>,
    pub folder: String,
    pub grouping: bool,
    pub layers: Vec<LayoutLayer>,
    pub rules: Vec<RuleText>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct LayoutLayer {
    pub id: String,
    pub role: String,
    pub place: Option<String>,
    pub not_used: Option<String>,
    pub may_use: Vec<LayerRef>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct RuleText {
    pub id: String,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub enum OwnedVia {
    Folder,
    Layer(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PathOwner {
    pub part: PartId,
    pub via: OwnedVia,
}

impl ProjectLayout {
    pub fn owners(&self, path: &str) -> Vec<PathOwner> {
        let Some(path) = segments(path) else {
            return Vec::new();
        };
        let mut folder = None;
        for part in &self.parts {
            if let Some(prefix) = segments(&part.folder)
                && path.starts_with(&prefix)
                && folder
                    .as_ref()
                    .is_none_or(|(depth, _)| prefix.len() > *depth)
            {
                folder = Some((prefix.len(), part.id.clone()));
            }
        }
        let mut owners = folder
            .map(|(_, part)| PathOwner {
                part,
                via: OwnedVia::Folder,
            })
            .into_iter()
            .collect::<Vec<_>>();
        for part in &self.parts {
            for layer in &part.layers {
                if layer.not_used.is_some() {
                    continue;
                }
                if let Some(place) = layer.place.as_deref().and_then(segments)
                    && place.len() <= path.len()
                    && place.iter().zip(&path).all(|(a, b)| matches_segment(a, b))
                {
                    owners.push(PathOwner {
                        part: part.id.clone(),
                        via: OwnedVia::Layer(layer.id.clone()),
                    });
                }
            }
        }
        owners
    }
}

fn segments(path: &str) -> Option<Vec<String>> {
    if path.starts_with('/') || path.contains(['\\', ':']) || path.chars().any(char::is_control) {
        return None;
    }
    let mut parts = Vec::new();
    for part in path.split('/').filter(|s| !s.is_empty() && *s != ".") {
        if part == ".." {
            return None;
        }
        parts.push(part.to_lowercase());
    }
    (!parts.is_empty()).then_some(parts)
}

fn matches_segment(pattern: &str, value: &str) -> bool {
    let pieces = pattern
        .split(crate::map_rules::MODULE_MARKER)
        .collect::<Vec<_>>();
    if pieces.len() == 1 {
        return pattern == value;
    }
    if !value.starts_with(pieces[0]) {
        return false;
    }
    let mut positions = vec![pieces[0].len()];
    for piece in &pieces[1..] {
        let mut next = Vec::new();
        for (end, _) in value
            .char_indices()
            .chain(std::iter::once((value.len(), '\0')))
        {
            if positions.iter().any(|start| end > *start) && value[end..].starts_with(piece) {
                next.push(end + piece.len());
            }
        }
        positions = next;
    }
    positions.contains(&value.len())
}
