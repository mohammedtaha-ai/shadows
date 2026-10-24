//! Pure map ownership, dependency and coverage rules.
use super::{PartContent, VisionContent};
use core_standards::EffectiveStandards;
use std::collections::{BTreeMap, BTreeSet};

pub(crate) struct MapPart<'a> {
    pub key: &'a str,
    pub parent: Option<&'a str>,
    pub content: &'a PartContent,
}

impl MapRules {
    pub(crate) fn normalise_folder(raw: &str) -> Result<String, String> {
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
    pub(crate) fn check_reserved<'a>(
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
    pub(crate) fn normalise(content: &mut PartContent) -> Result<(), String> {
        content.title = content.title.trim().into();
        if content.title.is_empty() {
            return Err("a part needs a title".into());
        }
        for folder in &mut content.folders {
            *folder = MapRules::normalise_folder(folder)?;
        }
        for file in &mut content.files {
            file.path = MapRules::normalise_folder(&file.path)?;
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
    pub(crate) fn check_shape(
        parts: &[MapPart<'_>],
        written: &[&str],
        vision: &VisionContent,
        standards: &EffectiveStandards,
    ) -> Result<(), String> {
        let mut keys = BTreeSet::new();
        let mut identities = BTreeSet::new();
        let mut folders = Vec::new();
        let mut files = BTreeSet::new();
        let rules: BTreeSet<_> = standards
            .base
            .rules
            .iter()
            .chain(standards.additions.iter().flat_map(|a| &a.content.rules))
            .map(|r| r.id.as_str())
            .collect();
        for part in parts {
            if !keys.insert(part.key) {
                return Err("duplicate part key".into());
            }
            if !identities.insert((part.parent, part.content.title.trim().to_lowercase())) {
                return Err(format!(
                    "duplicate sibling identity: {}",
                    part.content.title
                ));
            }
            if let Some(parent) = part.parent
                && (!parts.iter().any(|p| p.key == parent)
                    || parent == part.key
                    || descendant(parts, parent, part.key))
            {
                return Err("missing parent or containment cycle".into());
            }
            if written.contains(&part.key) {
                for id in &part.content.serves {
                    if vision.item(id).is_none() {
                        return Err(format!("unknown vision item: {id}"));
                    }
                }
                for rule in &part.content.contract.rules {
                    if !rules.contains(rule.as_str()) {
                        return Err(format!("unknown rule: {rule}"));
                    }
                }
            }
            for folder in &part.content.folders {
                folders.push((part.key, MapRules::normalise_folder(folder)?.to_lowercase()));
            }
            for file in &part.content.files {
                // Only the parts this write changes: an older part's reserved file
                // must not block the change that removes another's.
                if written.contains(&part.key) {
                    MapRules::check_reserved(
                        &file.path,
                        parts.iter().filter_map(|p| p.content.folders.first()),
                    )?;
                }
                let path = MapRules::normalise_folder(&file.path)?.to_lowercase();
                if !part
                    .content
                    .folders
                    .iter()
                    .any(|f| inside(&path, &f.to_lowercase()))
                {
                    return Err(format!("file outside its part's folders: {}", file.path));
                }
                if !files.insert(path) {
                    return Err(format!("duplicate file: {}", file.path));
                }
            }
        }
        for file in &files {
            if folders
                .iter()
                .any(|(_, folder)| folder == file || inside(folder, file))
                || files.iter().any(|other| inside(other, file))
            {
                return Err(format!("file path is also a directory: {file}"));
            }
        }
        for (i, (owner, folder)) in folders.iter().enumerate() {
            for (other, path) in &folders[i + 1..] {
                if folder == path
                    || (inside(path, folder) && !descendant(parts, other, owner))
                    || (inside(folder, path) && !descendant(parts, owner, other))
                {
                    return Err(format!("folder ownership conflict: {folder}, {path}"));
                }
            }
            // A parent's file cannot claim a child's folder.
            for part in parts
                .iter()
                .filter(|p| p.key != *owner && !descendant(parts, p.key, owner))
            {
                if part
                    .content
                    .files
                    .iter()
                    .any(|f| inside(&f.path.to_lowercase(), folder))
                {
                    return Err(format!("file owned by another part under {folder}"));
                }
            }
        }
        Ok(())
    }
}

impl MapRules {
    pub(crate) fn missing_kinds(
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
    pub(crate) fn unserved_goals(parts: &[MapPart<'_>], vision: &VisionContent) -> Vec<String> {
        vision
            .goals
            .iter()
            .filter(|g| !parts.iter().any(|p| p.content.serves.contains(&g.id)))
            .map(|g| g.id.clone())
            .collect()
    }
}

impl MapRules {
    pub(crate) fn drift(
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

pub(in crate::design) struct MapRules;

#[cfg(test)]
mod tests {
    use super::{EffectiveStandards, MapPart, MapRules, PartContent, VisionContent};

    #[test]
    fn windows_device_names_are_refused() {
        for path in ["nul", "backend/con.txt", "Com1", "LPT9.rs"] {
            assert!(MapRules::normalise_folder(path).is_err(), "{path}");
        }
    }

    #[test]
    fn reserved_initial_files_are_refused() {
        for path in ["backend/CONTRACT.yaml", "SHADOWS.md"] {
            let content = PartContent {
                title: "Backend".into(),
                folders: vec!["backend".into()],
                files: vec![super::super::InitialFile {
                    path: path.into(),
                    content: "".into(),
                }],
                ..Default::default()
            };
            let parts = [MapPart {
                key: "p1",
                parent: None,
                content: &content,
            }];
            let standards = EffectiveStandards {
                base: core_standards::Standards::base().clone(),
                additions: None,
            };
            assert_eq!(
                MapRules::check_shape(&parts, &["p1"], &VisionContent::default(), &standards),
                Err(format!("reserved path: {path}"))
            );
            // An unchanged part's reserved file never blocks another part's
            // change: each can then be repaired one at a time. (A root
            // SHADOWS.md is also outside its part's folders, refused anyway.)
            if path != "SHADOWS.md" {
                assert_eq!(
                    MapRules::check_shape(&parts, &[], &VisionContent::default(), &standards),
                    Ok(())
                );
            }
        }
    }
}
