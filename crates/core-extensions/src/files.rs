//! Bounded immutable library copies, without executing imported components.
use super::{ExtensionKind, Extensions, ExtensionsError, Item};
use sha2::{Digest, Sha256};
use std::io::Read;
use std::path::{Path, PathBuf};

fn invalid() -> ExtensionsError {
    ExtensionsError::Invalid("invalid extension source".into())
}
fn checked(path: &Path) -> Result<std::fs::Metadata, ExtensionsError> {
    let meta = std::fs::symlink_metadata(path).map_err(|_| invalid())?;
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if meta.file_attributes() & 0x400 != 0 {
            return Err(invalid());
        }
    }
    if meta.file_type().is_symlink() || (!meta.is_file() && !meta.is_dir()) {
        return Err(invalid());
    }
    Ok(meta)
}
fn files(
    root: &Path,
    path: &Path,
    out: &mut Vec<(PathBuf, Vec<u8>)>,
    size: &mut usize,
    visited: &mut usize,
    depth: usize,
) -> Result<(), ExtensionsError> {
    *visited += 1;
    if *visited > 20000 || depth > 256 {
        return Err(invalid());
    }
    let meta = checked(path)?;
    if meta.is_file() {
        if out.len() >= 10000 || meta.len() > 64 * 1024 * 1024 {
            return Err(invalid());
        }
        let mut bytes = Vec::new();
        let limit = (64 * 1024 * 1024usize).saturating_sub(*size) + 1;
        std::fs::File::open(path)
            .map_err(|_| invalid())?
            .take(limit as u64)
            .read_to_end(&mut bytes)
            .map_err(|_| invalid())?;
        *size = size.checked_add(bytes.len()).ok_or_else(invalid)?;
        if *size > 64 * 1024 * 1024 {
            return Err(invalid());
        }
        out.push((
            path.strip_prefix(root).map_err(|_| invalid())?.into(),
            bytes,
        ));
    } else {
        let mut children = Vec::new();
        for child in std::fs::read_dir(path).map_err(|_| invalid())? {
            if children.len() >= 20000 {
                return Err(invalid());
            }
            children.push(child.map_err(|_| invalid())?.path());
        }
        children.sort();
        for child in children {
            files(root, &child, out, size, visited, depth + 1)?;
        }
    }
    Ok(())
}
fn name_valid(name: &str) -> bool {
    let b = name.as_bytes();
    (1..=64).contains(&b.len())
        && b[0].is_ascii_lowercase()
        && b.iter()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || *c == b'-')
}
fn metadata(root: &Path, kind: ExtensionKind, id: &str) -> Result<Item, ExtensionsError> {
    let (name, description) = match kind {
        ExtensionKind::Plugin => {
            let value: serde_json::Value = serde_json::from_slice(
                &std::fs::read(root.join(".claude-plugin/plugin.json")).map_err(|_| invalid())?,
            )
            .map_err(|_| invalid())?;
            (
                value["name"].as_str().ok_or_else(invalid)?.to_owned(),
                value["description"]
                    .as_str()
                    .ok_or_else(invalid)?
                    .to_owned(),
            )
        }
        ExtensionKind::Skill => {
            let text = std::fs::read_to_string(root.join("SKILL.md")).map_err(|_| invalid())?;
            let text = text.replace("\r\n", "\n");
            let rest = text.strip_prefix("---\n").ok_or_else(invalid)?;
            let (front, _) = rest.split_once("\n---").ok_or_else(invalid)?;
            let docs = yaml_rust2::YamlLoader::load_from_str(front).map_err(|_| invalid())?;
            let doc = docs.first().ok_or_else(invalid)?;
            (
                doc["name"].as_str().ok_or_else(invalid)?.to_owned(),
                doc["description"].as_str().ok_or_else(invalid)?.to_owned(),
            )
        }
        ExtensionKind::Mcp => return Err(invalid()),
    };
    if !name_valid(&name) || description.trim().is_empty() || description.len() > 4096 {
        return Err(invalid());
    }
    Ok(Item {
        id: id.into(),
        name,
        description,
        bundled: false,
    })
}

impl Extensions {
    /// Copies a bounded source into the library under `id`, answering its metadata and digest.
    pub(super) fn publish(
        source: &Path,
        library: &Path,
        kind: ExtensionKind,
        id: &str,
    ) -> Result<(Item, String), ExtensionsError> {
        publish(source, library, kind, id)
    }

    /// Copies one library item's regular files into a managed snapshot.
    pub fn copy_tree(source: &Path, target: &Path) -> Result<(), ExtensionsError> {
        copy_tree(source, target)
    }

    /// Metadata deletion is authoritative; failed cleanup leaves only an inert owned copy.
    pub(super) fn retire(library: &Path, id: &str) {
        retire(library, id)
    }
}

fn publish(
    source: &Path,
    library: &Path,
    kind: ExtensionKind,
    id: &str,
) -> Result<(Item, String), ExtensionsError> {
    for ancestor in source.ancestors() {
        if ancestor.exists() {
            checked(ancestor)?;
        }
    }
    let source = dunce::canonicalize(source).map_err(|_| invalid())?;
    std::fs::create_dir_all(library).map_err(|_| invalid())?;
    let library = dunce::canonicalize(library).map_err(|_| invalid())?;
    if library.starts_with(&source) || source.starts_with(&library) {
        return Err(invalid());
    }
    let mut entries = Vec::new();
    files(&source, &source, &mut entries, &mut 0, &mut 0, 0)?;
    let stage = library.join(format!(".import-{id}"));
    let final_path = library.join(id);
    let result = (|| {
        std::fs::create_dir(&stage).map_err(|_| invalid())?;
        let mut digest = Sha256::new();
        for (relative, bytes) in entries {
            let name = relative.to_string_lossy().replace('\\', "/");
            digest.update((name.len() as u64).to_le_bytes());
            digest.update(name.as_bytes());
            digest.update((bytes.len() as u64).to_le_bytes());
            digest.update(&bytes);
            let target = stage.join(relative);
            std::fs::create_dir_all(target.parent().ok_or_else(invalid)?).map_err(|_| invalid())?;
            std::fs::write(target, bytes).map_err(|_| invalid())?;
        }
        let item = metadata(&stage, kind, id)?;
        std::fs::rename(&stage, &final_path).map_err(|_| invalid())?;
        Ok((item, format!("{:x}", digest.finalize())))
    })();
    if result.is_err() {
        let _ = std::fs::remove_dir_all(stage);
    }
    result
}

fn copy_tree(source: &Path, target: &Path) -> Result<(), ExtensionsError> {
    let mut entries = Vec::new();
    files(source, source, &mut entries, &mut 0, &mut 0, 0)?;
    std::fs::create_dir_all(target).map_err(|_| invalid())?;
    for (relative, bytes) in entries {
        let path = target.join(relative);
        std::fs::create_dir_all(path.parent().ok_or_else(invalid)?).map_err(|_| invalid())?;
        std::fs::write(path, bytes).map_err(|_| invalid())?;
    }
    Ok(())
}

fn retire(library: &Path, id: &str) {
    if uuid::Uuid::parse_str(id).is_err() {
        return;
    }
    let target = library.join(id);
    let Ok(meta) = checked(&target) else {
        return;
    };
    if !meta.is_dir() {
        return;
    }
    let Ok(parent) = dunce::canonicalize(library) else {
        return;
    };
    let Ok(actual) = dunce::canonicalize(&target) else {
        return;
    };
    if actual.parent() == Some(parent.as_path()) {
        let _ = std::fs::remove_dir_all(target);
    }
}
