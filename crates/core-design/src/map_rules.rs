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
                "part {title:?}: {message}. Use a relative path without parent traversal or reserved names."
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
    pub fn check_reserved<'a>(
        path: &str,
        first_folders: impl Iterator<Item = &'a String>,
    ) -> Result<(), String> {
        if path.eq_ignore_ascii_case("SHADOWS.md")
            || first_folders.into_iter().any(|f| {
                MapRules::normalise_folder(f)
                    .is_ok_and(|f| path.eq_ignore_ascii_case(&format!("{f}/contract.yaml")))
            })
        {
            return Err(format!("reserved path: {path}"));
        }
        Ok(())
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
        for folder in &mut content.folders {
            *folder = MapRules::normalise_folder(folder)
                .map_err(|message| MapRefusal::path(&content.title, message))?;
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
            for folder in &part.content.folders {
                let path = MapRules::normalise_folder(folder)
                    .map_err(|message| MapRefusal::path(&part.content.title, message))?;
                folders.push((part.key, path.to_lowercase()));
            }
            for file in &part.content.files {
                // Only the parts this write changes: an older part's reserved file
                // must not block the change that removes another's.
                if written.contains(&part.key) {
                    MapRules::check_reserved(
                        &file.path,
                        parts.iter().filter_map(|p| p.content.folders.first()),
                    ).map_err(|message| MapRefusal::new("reserved_file", format!(
                        "part {:?}: {message}. Remove the generated file from the proposal.",
                        part.content.title)))?;
                }
                let path = MapRules::normalise_folder(&file.path)
                    .map_err(|message| MapRefusal::path(&part.content.title, message))?
                    .to_lowercase();
                if !part
                    .content
                    .folders
                    .iter()
                    .any(|f| inside(&path, &f.to_lowercase()))
                {
                    return Err(MapRefusal::new(
                        "file_outside_part",
                        format!(
                            "part {:?} lists {}, which is outside its folders {:?}. \
                        Move the file into one of them, or add its folder to the part.",
                            part.content.title, file.path, part.content.folders
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
                        format!(
                            "{message} Give the folder to one part, or make one part the other's child."
                        ),
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
