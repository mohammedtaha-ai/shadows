//! Pure map ownership, dependency and coverage rules.
use super::{PartContent, VisionContent};
use core_standards::EffectiveStandards;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{rule}: {message}")]
pub struct MapRefusal {
    pub rule: &'static str,
    pub message: String,
}

impl MapRefusal {
    fn new(rule: &'static str, message: String) -> Self {
        Self { rule, message }
    }

    fn path(title: &str, message: String) -> Self {
        Self::new(
            "invalid_path",
            format!(
                "part {title:?}: {message}. \
                 Use a relative path without parent traversal or reserved names."
            ),
        )
    }
}

pub struct MapPart<'a> {
    pub key: &'a str,
    pub parent: Option<&'a str>,
    pub content: &'a PartContent,
}

impl MapRules {
    pub fn normalise_folder(raw: &str) -> Result<String, String> {
        let path = raw.replace('\\', "/");
        let path = path
            .strip_prefix("./")
            .unwrap_or(&path)
            .trim_end_matches('/');
        if path.is_empty()
            || path.starts_with('/')
            || path.contains(':')
            || path.split('/').any(|s| {
                s.is_empty()
                    || s == "."
                    || s == ".."
                    || s.ends_with(['.', ' '])
                    || device_name(s)
                    || s.chars().any(|c| c.is_control() || "<>\"|?*".contains(c))
            })
        {
            return Err(format!("invalid relative path: {raw}"));
        }
        Ok(path.into())
    }
}

fn device_name(segment: &str) -> bool {
    let stem = segment
        .split('.')
        .next()
        .unwrap_or(segment)
        .to_ascii_uppercase();
    matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || ["COM", "LPT"].iter().any(|prefix| {
            stem.strip_prefix(prefix)
                .is_some_and(|n| n.len() == 1 && ("1"..="9").contains(&n))
        })
}

/// The marker a place holds when the stack gives every module its own folder.
pub(crate) const MODULE_MARKER: &str = "{module}";

impl MapRules {
    /// A layer's place follows the stack; markers are descriptive text.
    fn normalise_place(raw: &str) -> Result<String, String> {
        let path = MapRules::normalise_folder(raw)?;
        Ok(path)
    }
}

impl MapRules {
    pub fn normalise(content: &mut PartContent) -> Result<(), MapRefusal> {
        content.title = content.title.trim().into();
        if content.title.is_empty() {
            return Err(MapRefusal::new(
                "invalid_part",
                "a part needs a title. Add a title.".into(),
            ));
        }
        if !content.folder.is_empty() {
            content.folder = MapRules::normalise_folder(&content.folder)
                .map_err(|message| MapRefusal::path(&content.title, message))?;
        }
        for layer in &mut content.layers {
            if layer.place.is_empty() {
                continue;
            }
            layer.place = MapRules::normalise_place(&layer.place).map_err(|message| {
                MapRefusal::new(
                    "invalid_path",
                    format!(
                        "part {:?} layer {}: {message}. Use a relative path \
                         without parent traversal or reserved names.",
                        content.title, layer.layer
                    ),
                )
            })?;
        }
        Ok(())
    }
}

fn inside(inner: &str, outer: &str) -> bool {
    inner.starts_with(&format!("{outer}/"))
}

fn descendant<'a>(parts: &[MapPart<'a>], mut child: &'a str, ancestor: &str) -> bool {
    for _ in 0..parts.len() {
        let Some(parent) = parts.iter().find(|p| p.key == child).and_then(|p| p.parent) else {
            return false;
        };
        if parent == ancestor {
            return true;
        }
        child = parent;
    }
    false
}

impl MapRules {
    pub fn check_shape(
        parts: &[MapPart<'_>],
        written: &[&str],
        vision: &VisionContent,
        standards: &EffectiveStandards,
    ) -> Result<(), MapRefusal> {
        let mut keys = BTreeSet::new();
        let mut identities = BTreeSet::new();
        let rules: BTreeSet<_> = standards
            .base
            .rules
            .iter()
            .chain(standards.additions.iter().flat_map(|a| &a.content.rules))
            .map(|r| r.id.as_str())
            .collect();
        for part in parts {
            if !keys.insert(part.key) {
                return Err(MapRefusal::new(
                    "duplicate_part",
                    format!(
                        "part {:?} repeats key {:?}. Give every part a unique key.",
                        part.content.title, part.key
                    ),
                ));
            }
            if !identities.insert((part.parent, part.content.title.trim().to_lowercase())) {
                return Err(MapRefusal::new(
                    "duplicate_sibling",
                    format!(
                        "part {:?} repeats a sibling title. Rename it or choose another parent.",
                        part.content.title
                    ),
                ));
            }
            if let Some(parent) = part.parent
                && (!parts.iter().any(|p| p.key == parent)
                    || parent == part.key
                    || descendant(parts, parent, part.key))
            {
                return Err(MapRefusal::new(
                    "missing_parent",
                    format!(
                        "part {:?} has missing parent {parent:?} or a containment cycle. \
                    Choose an existing parent outside its descendants.",
                        part.content.title
                    ),
                ));
            }
            if written.contains(&part.key) {
                for id in &part.content.serves {
                    if vision.item(id).is_none() {
                        return Err(MapRefusal::new(
                            "unknown_vision_item",
                            format!(
                                "part {:?} serves unknown vision item {id}. \
                            Read the vision and use an existing item id.",
                                part.content.title
                            ),
                        ));
                    }
                }
                for rule in &part.content.contract.rules {
                    if !rules.contains(rule.as_str()) {
                        return Err(MapRefusal::new(
                            "unknown_rule",
                            format!(
                                "part {:?} names unknown rule {rule}. \
                            Read the standards and use an existing rule id.",
                                part.content.title
                            ),
                        ));
                    }
                }
            }
        }
        MapRules::check_places(parts, standards)
    }
}

impl MapRules {
    /// The one policy for a part's folder and its layer places: the folder
    /// rules of the map, and the layers the standards' catalog asks for.
    pub fn check_places(
        parts: &[MapPart<'_>],
        standards: &EffectiveStandards,
    ) -> Result<(), MapRefusal> {
        let mut folders: Vec<(&str, String)> = Vec::new();
        for part in parts {
            if part.content.folder.is_empty() {
                return Err(MapRefusal::new(
                    "one_folder",
                    format!(
                        "part {:?} has no folder. Give the part one folder.",
                        part.content.title
                    ),
                ));
            }
            let folder = MapRules::normalise_folder(&part.content.folder)
                .map_err(|message| MapRefusal::path(&part.content.title, message))?
                .to_lowercase();
            // Part ownership needs a concrete folder; layer places can describe patterns.
            if folder.contains(MODULE_MARKER) {
                return Err(MapRefusal::new(
                    "invalid_path",
                    format!(
                        "part {:?} has the folder {:?}, which holds {MODULE_MARKER}. \
                         Give the part one literal folder and write the marker in a layer place.",
                        part.content.title, part.content.folder
                    ),
                ));
            }
            folders.push((part.key, folder.clone()));
            let Some(parent) = part.parent.and_then(|k| parts.iter().find(|p| p.key == k)) else {
                continue;
            };
            if parent.content.folder.is_empty() {
                continue;
            }
            let outer = parent.content.folder.to_lowercase();
            let (child, outer) = (folder.trim_end_matches('/'), outer.trim_end_matches('/'));
            if child != outer && !inside(child, outer) {
                return Err(MapRefusal::new(
                    "folder_outside_parent",
                    format!(
                        "part {:?} has folder {:?}, outside its parent {:?} folder {:?}. \
                         Put the folder inside the parent's, or choose another parent.",
                        part.content.title,
                        part.content.folder,
                        parent.content.title,
                        parent.content.folder
                    ),
                ));
            }
        }
        MapRules::check_folder_claims(parts, &folders)?;
        MapRules::check_layers(parts, standards)
    }
}

impl MapRules {
    /// Two parts cannot claim the same folder or nest outside their containment.
    fn check_folder_claims(
        parts: &[MapPart<'_>],
        folders: &[(&str, String)],
    ) -> Result<(), MapRefusal> {
        for (i, (owner, folder)) in folders.iter().enumerate() {
            for (other, path) in &folders[i + 1..] {
                if folder == path
                    || (inside(path, folder) && !descendant(parts, other, owner))
                    || (inside(folder, path) && !descendant(parts, owner, other))
                {
                    let name = |key: &str| {
                        parts
                            .iter()
                            .find(|p| p.key == key)
                            .expect("listed folder has an owner")
                            .content
                            .title
                            .as_str()
                    };
                    let message = if folder == path {
                        format!(
                            "parts {:?} and {:?} both claim {folder}.",
                            name(owner),
                            name(other)
                        )
                    } else {
                        format!(
                            "parts {:?} ({folder}) and {:?} ({path}) overlap.",
                            name(owner),
                            name(other)
                        )
                    };
                    return Err(MapRefusal::new(
                        "folder_claimed_twice",
                        if folder == path {
                            format!(
                                "{message} Give the folder one owner, \
                                 or change one part's folder claim to a distinct path."
                            )
                        } else {
                            format!(
                                "{message} Give the folder to one part, \
                                 or make one part the other's child."
                            )
                        },
                    ));
                }
            }
        }
        Ok(())
    }
}

impl MapRules {
    /// A part that holds a child of its own kind groups them, and needs no
    /// layers of its own.
    fn is_grouping(part: &MapPart<'_>, parts: &[MapPart<'_>]) -> bool {
        let Some(kind) = part.content.kind.as_deref() else {
            return false;
        };
        parts.iter().any(|child| {
            child.parent == Some(part.key)
                && child.content.kind.as_deref().map(str::trim) == Some(kind.trim())
        })
    }
}

impl MapRules {
    /// The layers of every part, against the standards' catalog.
    fn check_layers(
        parts: &[MapPart<'_>],
        standards: &EffectiveStandards,
    ) -> Result<(), MapRefusal> {
        for part in parts {
            let catalog = standards.layers(part.content.kind.as_deref().unwrap_or_default().trim());
            for (index, layer) in part.content.layers.iter().enumerate() {
                if !catalog.iter().any(|c| c.id == layer.layer) {
                    return Err(MapRefusal::new(
                        "unknown_layer",
                        format!(
                            "part {:?} names unknown layer {:?}. Use a layer of its kind \
                             from the standards.",
                            part.content.title, layer.layer
                        ),
                    ));
                }
                if part.content.layers[..index]
                    .iter()
                    .any(|other| other.layer == layer.layer)
                {
                    return Err(MapRefusal::new(
                        "unknown_layer",
                        format!(
                            "part {:?} names layer {:?} twice. Give each layer one entry.",
                            part.content.title, layer.layer
                        ),
                    ));
                }
            }
            for layer in &part.content.layers {
                let Some(reason) = &layer.not_used else {
                    continue;
                };
                if reason.trim().is_empty() {
                    return Err(MapRefusal::new(
                        "layer_not_used_without_reason",
                        format!(
                            "part {:?} layer {} is not used without a reason. \
                             Write why the layer is not used.",
                            part.content.title, layer.layer
                        ),
                    ));
                }
                if !layer.place.is_empty() {
                    return Err(MapRefusal::new(
                        "layer_not_used_without_reason",
                        format!(
                            "part {:?} layer {} is not used and still has the place {:?}. \
                             Give a reason and no place, or a place and no reason.",
                            part.content.title, layer.layer, layer.place
                        ),
                    ));
                }
            }
            if !MapRules::is_grouping(part, parts) {
                for required in catalog.iter().filter(|c| c.required) {
                    let provided = part
                        .content
                        .layers
                        .iter()
                        .find(|l| l.layer == required.id)
                        .is_some_and(|l| {
                            !l.place.is_empty()
                                || l.not_used
                                    .as_deref()
                                    .is_some_and(|reason| !reason.trim().is_empty())
                        });
                    if !provided {
                        return Err(MapRefusal::new(
                            "missing_layer",
                            format!(
                                "part {:?} has no {} layer. Give it a place in this \
                                 stack, or mark it not used with a reason.",
                                part.content.title, required.id
                            ),
                        ));
                    }
                }
            }
            let mut places = Vec::new();
            for layer in &part.content.layers {
                if layer.place.is_empty() {
                    if layer.not_used.is_none() {
                        return Err(MapRefusal::new(
                            "missing_layer",
                            format!(
                                "part {:?} layer {} has no place. Give it a place, \
                                 or remove the optional layer.",
                                part.content.title, layer.layer
                            ),
                        ));
                    }
                    continue;
                }
                let place = MapRules::normalise_place(&layer.place).map_err(|message| {
                    MapRefusal::new(
                        "invalid_path",
                        format!(
                            "part {:?} layer {}: {message}",
                            part.content.title, layer.layer
                        ),
                    )
                })?;
                places.push((layer.layer.as_str(), place.to_lowercase()));
            }
            for (index, (id, place)) in places.iter().enumerate() {
                for (other_id, other_place) in &places[index + 1..] {
                    if place == other_place {
                        return Err(MapRefusal::new(
                            "layer_place_shared",
                            format!(
                                "part {:?} layers {id} and {other_id} name the same \
                                 place. Give each layer its own place.",
                                part.content.title
                            ),
                        ));
                    }
                }
            }
        }
        Ok(())
    }
}

impl MapRules {
    pub fn missing_kinds(
        parts: &[MapPart<'_>],
        waived: &[String],
        standards: &EffectiveStandards,
    ) -> Vec<String> {
        standards
            .mandatory_parts()
            .iter()
            .filter(|kind| {
                !waived.iter().any(|w| w == **kind)
                    && !parts.iter().any(|p| {
                        p.content
                            .kind
                            .as_deref()
                            .is_some_and(|k| k.trim() == **kind)
                    })
            })
            .map(|k| k.to_string())
            .collect()
    }
}

impl MapRules {
    pub fn unserved_items(parts: &[MapPart<'_>], vision: &VisionContent) -> Vec<String> {
        let served: BTreeSet<_> = parts
            .iter()
            .flat_map(|part| &part.content.serves)
            .map(String::as_str)
            .collect();
        crate::VisionField::ALL
            .into_iter()
            .flat_map(|field| vision.items(field))
            .filter(|item| !served.contains(item.id.as_str()))
            .map(|item| item.id.clone())
            .collect()
    }

    pub fn unserved_goals(parts: &[MapPart<'_>], vision: &VisionContent) -> Vec<String> {
        vision
            .goals
            .iter()
            .filter(|g| !parts.iter().any(|p| p.content.serves.contains(&g.id)))
            .map(|g| g.id.clone())
            .collect()
    }
}

impl MapRules {
    pub fn drift(
        parts: &[(MapPart<'_>, &BTreeMap<String, i64>)],
        vision: &VisionContent,
    ) -> Vec<String> {
        let mut result = Vec::new();
        for (p, revisions) in parts {
            for id in &p.content.serves {
                match vision.item(id) {
                    None => result.push(format!(
                        "{} serves {id}, which was removed",
                        p.content.title
                    )),
                    Some(item) if revisions.get(id) != Some(&item.revision) => {
                        result.push(format!(
                            "{}: {id} changed since the part was reviewed",
                            p.content.title
                        ))
                    }
                    _ => {}
                }
            }
        }
        let views = parts
            .iter()
            .map(|(p, _)| MapPart {
                key: p.key,
                parent: p.parent,
                content: p.content,
            })
            .collect::<Vec<_>>();
        result.extend(
            MapRules::unserved_goals(&views, vision)
                .into_iter()
                .map(|g| format!("{g} is served by no part")),
        );
        result
    }
}

pub struct MapRules;
