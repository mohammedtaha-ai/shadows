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

impl MapRules {
    pub fn check_reserved(path: &str) -> Result<(), String> {
        if path.eq_ignore_ascii_case("SHADOWS.md") {
            return Err(format!("reserved path: {path}"));
        }
        Ok(())
    }
}

/// The marker a place holds when the stack gives every module its own folder.
pub(crate) const MODULE_MARKER: &str = "{module}";

impl MapRules {
    /// A layer's place: a path that may hold one `{module}` marker.
    fn normalise_place(raw: &str) -> Result<String, String> {
        let path = MapRules::normalise_folder(raw)?;
        if path.matches(MODULE_MARKER).count() > 1 {
            return Err(format!(
                "{raw} holds the {MODULE_MARKER} marker twice. Keep one marker"
            ));
        }
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
                         inside the part, with at most one {MODULE_MARKER}.",
                        content.title, layer.layer
                    ),
                )
            })?;
        }
        for file in &mut content.files {
            file.path = MapRules::normalise_folder(&file.path)
                .map_err(|message| MapRefusal::path(&content.title, message))?;
        }
        Ok(())
    }
}

fn inside(inner: &str, outer: &str) -> bool {
    inner.starts_with(&format!("{outer}/"))
}

/// Whether two paths can be the same path, or one inside the other, for
/// some module name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Overlap {
    Equal,
    AInsideB,
    BInsideA,
    None,
}

/// The path's segments, lowercased.
fn segments(path: &str) -> Vec<String> {
    path.replace('\\', "/")
        .split('/')
        .map(|s| s.to_lowercase())
        .collect()
}

/// The text before and after the marker, when a segment holds one.
fn marker_parts(segment: &str) -> Option<(&str, &str)> {
    segment.split_once(MODULE_MARKER)
}

/// Whether a literal segment can be the module name a marker segment names.
fn literal_matches_marker(literal: &str, before: &str, after: &str) -> bool {
    literal.len() > before.len() + after.len()
        && literal.starts_with(before)
        && literal.ends_with(after)
}

/// Whether two marker segments can name the same module.
fn markers_may_match(a_before: &str, a_after: &str, b_before: &str, b_after: &str) -> bool {
    (a_before.starts_with(b_before) || b_before.starts_with(a_before))
        && (a_after.ends_with(b_after) || b_after.ends_with(a_after))
}

/// Whether two segments can name the same path segment.
fn segments_may_match(a: &str, b: &str) -> bool {
    match (marker_parts(a), marker_parts(b)) {
        (None, None) => a == b,
        (Some((before, after)), None) => literal_matches_marker(b, before, after),
        (None, Some((before, after))) => literal_matches_marker(a, before, after),
        (Some((a_before, a_after)), Some((b_before, b_after))) => {
            markers_may_match(a_before, a_after, b_before, b_after)
        }
    }
}

/// Whether one module name can make the two paths overlap: the same path,
/// or one inside the other.
fn may_overlap(a: &str, b: &str) -> Overlap {
    let a = segments(a);
    let b = segments(b);
    if !a.iter().zip(&b).all(|(x, y)| segments_may_match(x, y)) {
        return Overlap::None;
    }
    match a.len().cmp(&b.len()) {
        std::cmp::Ordering::Equal => Overlap::Equal,
        std::cmp::Ordering::Less => Overlap::BInsideA,
        std::cmp::Ordering::Greater => Overlap::AInsideB,
    }
}

/// Whether every module name keeps the place inside the folder. The folder's
/// segments must be a literal prefix of the place, and the place must be
/// deeper than the folder.
fn always_inside(place: &str, folder: &str) -> bool {
    let place = segments(place);
    let folder = segments(folder);
    if place.len() <= folder.len() {
        return false;
    }
    folder
        .iter()
        .enumerate()
        .all(|(index, segment)| !segment.contains(MODULE_MARKER) && place[index] == *segment)
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
        let mut folders = Vec::new();
        let mut files = BTreeMap::new();
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
            if !part.content.folder.is_empty() {
                let path = MapRules::normalise_folder(&part.content.folder)
                    .map_err(|message| MapRefusal::path(&part.content.title, message))?;
                folders.push((part.key, path.to_lowercase()));
            }
            for file in &part.content.files {
                // Only the parts this write changes: an older part's reserved file
                // must not block the change that removes another's.
                if written.contains(&part.key) {
                    MapRules::check_reserved(&file.path).map_err(|message| {
                        MapRefusal::new(
                            "reserved_file",
                            format!(
                                "part {:?}: {message}. \
                                 Remove the generated file from the proposal.",
                                part.content.title
                            ),
                        )
                    })?;
                }
                let path = MapRules::normalise_folder(&file.path)
                    .map_err(|message| MapRefusal::path(&part.content.title, message))?
                    .to_lowercase();
                if !inside(&path, &part.content.folder.to_lowercase()) {
                    return Err(MapRefusal::new(
                        "file_outside_part",
                        format!(
                            "part {:?} lists {}, which is outside its folder {:?}. \
                        Move the file into it, or change the part's folder.",
                            part.content.title, file.path, part.content.folder
                        ),
                    ));
                }
                if let Some(owner_key) = files.insert(path, part.key) {
                    let owner = parts
                        .iter()
                        .find(|part| part.key == owner_key)
                        .expect("listed file has an owner");
                    return Err(MapRefusal::new(
                        "duplicate_file",
                        format!(
                            "parts {:?} and {:?} both list {}. \
                            Give each file one owner and one entry.",
                            owner.content.title, part.content.title, file.path
                        ),
                    ));
                }
            }
        }
        for (file, owner_key) in &files {
            if folders
                .iter()
                .any(|(_, folder)| folder == file || inside(folder, file))
                || files.keys().any(|other| inside(other, file))
                || parts
                    .iter()
                    .flat_map(|part| &part.content.layers)
                    .any(|layer| {
                        if layer.place.is_empty() || layer.place.contains(MODULE_MARKER) {
                            return false;
                        }
                        let path = layer.place.to_lowercase();
                        path == *file || inside(&path, file)
                    })
            {
                let owner = parts
                    .iter()
                    .find(|part| part.key == *owner_key)
                    .expect("listed file has an owner");
                return Err(MapRefusal::new(
                    "file_is_folder",
                    format!(
                        "part {:?} lists {file} as a file and a directory. \
                    Use separate paths for the file and directory.",
                        owner.content.title
                    ),
                ));
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
    /// Two parts cannot own the same folder, and a file cannot stand where
    /// another part's folder is.
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
            // A parent's file cannot claim a child's folder.
            for part in parts
                .iter()
                .filter(|p| p.key != *owner && !descendant(parts, p.key, owner))
            {
                if let Some(file) = part
                    .content
                    .files
                    .iter()
                    .find(|f| inside(&f.path.to_lowercase(), folder))
                {
                    let owner = parts
                        .iter()
                        .find(|part| part.key == *owner)
                        .expect("listed folder has an owner");
                    return Err(MapRefusal::new(
                        "folder_claimed_twice",
                        format!(
                            "part {:?} lists {} under {:?}'s folder {folder}. \
                        Give the file to that folder's owner or change the part containment.",
                            part.content.title, file.path, owner.content.title
                        ),
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
                                "part {:?} has no {} layer. Give it a place inside its \
                                 folder, or mark it not used with a reason.",
                                part.content.title, required.id
                            ),
                        ));
                    }
                }
            }
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
                if !always_inside(&layer.place, &part.content.folder) {
                    return Err(MapRefusal::new(
                        "layer_outside_part",
                        format!(
                            "part {:?} layer {} has the place {:?}, outside its folder {:?}. \
                             Put the place inside the folder.",
                            part.content.title, layer.layer, layer.place, part.content.folder
                        ),
                    ));
                }
            }
            let places: Vec<(&str, &str)> = part
                .content
                .layers
                .iter()
                .filter(|l| !l.place.is_empty())
                .map(|l| (l.layer.as_str(), l.place.as_str()))
                .collect();
            for (index, (id, place)) in places.iter().enumerate() {
                for (other_id, other_place) in &places[index + 1..] {
                    if may_overlap(place, other_place) != Overlap::None {
                        return Err(MapRefusal::new(
                            "layer_place_shared",
                            format!(
                                "part {:?} layers {id} and {other_id} can name the same \
                                 place. Give each layer its own place.",
                                part.content.title
                            ),
                        ));
                    }
                }
            }
            for (id, place) in &places {
                for child in parts.iter().filter(|c| c.parent == Some(part.key)) {
                    if child.content.folder.is_empty() {
                        continue;
                    }
                    if matches!(
                        may_overlap(place, &child.content.folder),
                        Overlap::Equal | Overlap::AInsideB
                    ) {
                        return Err(MapRefusal::new(
                            "layer_inside_child",
                            format!(
                                "part {:?} layer {id} has the place {place:?}, which can \
                                 reach its child {:?}'s folder {:?}. Keep the place out \
                                 of the child's folder.",
                                part.content.title, child.content.title, child.content.folder
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
                        p.parent.is_none()
                            && p.content
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
