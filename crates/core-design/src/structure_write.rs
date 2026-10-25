//! Checking and publishing the recorded files without replacing foreign content.
use super::map_rules::MapRules;
use super::structure::Planned;
use super::structure::Structure;
use crate::StorageError;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use tokio::io::AsyncWriteExt;

fn io(error: std::io::Error) -> crate::DesignError {
    StorageError::Unavailable(error.to_string()).into()
}
fn blocked(message: String) -> crate::DesignError {
    crate::DesignError::StageBlocked(message)
}

impl StructureWrite {
    pub(super) fn joined(root: &Path, path: &str) -> Result<PathBuf, crate::DesignError> {
        let relative = MapRules::normalise_folder(path).map_err(blocked)?;
        let joined = root.join(relative);
        if !joined.starts_with(root) {
            return Err(blocked(format!("invalid relative path: {path}")));
        }
        Ok(joined)
    }
}

fn reparse(meta: &std::fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        meta.file_attributes() & 0x400 != 0
    }
    #[cfg(not(windows))]
    {
        meta.file_type().is_symlink()
    }
}

impl StructureWrite {
    pub(super) async fn root(directory: &str) -> Result<PathBuf, crate::DesignError> {
        let meta = tokio::fs::symlink_metadata(directory)
            .await
            .map_err(|_| blocked(format!("project directory missing: {directory}")))?;
        if !meta.is_dir() || reparse(&meta) {
            return Err(blocked(format!("project directory missing: {directory}")));
        }
        tokio::fs::canonicalize(directory).await.map_err(io)
    }
}

fn directories(plan: &Planned) -> BTreeSet<String> {
    let mut dirs: BTreeSet<_> = plan.folders.iter().cloned().collect();
    for file in &plan.files {
        let mut parts = file.path.split('/').collect::<Vec<_>>();
        parts.pop();
        while !parts.is_empty() {
            dirs.insert(parts.join("/"));
            parts.pop();
        }
    }
    dirs
}

fn temporary_paths(plan: &Planned) -> BTreeMap<&str, String> {
    let mut occupied: BTreeSet<_> = directories(plan)
        .into_iter()
        .chain(plan.files.iter().map(|f| f.path.clone()))
        .map(|p| p.to_lowercase())
        .collect();
    let mut result = BTreeMap::new();
    for file in &plan.files {
        let mut temp = format!("{}.shadows-tmp", file.path);
        while !occupied.insert(temp.to_lowercase()) {
            temp.push_str(".shadows-tmp");
        }
        result.insert(file.path.as_str(), temp);
    }
    result
}

/// Read the entire directory before deleting even a single temporary file.
impl StructureWrite {
    pub(super) async fn check(
        root: &Path,
        plan: &Planned,
        resumed: bool,
    ) -> Result<Vec<PathBuf>, crate::DesignError> {
        let files: BTreeMap<_, _> = plan
            .files
            .iter()
            .map(|f| (f.path.as_str(), &f.bytes))
            .collect();
        let temps: BTreeSet<_> = temporary_paths(plan).into_values().collect();
        let dirs = directories(plan);
        let mut stack = vec![String::new()];
        let mut unexpected = Vec::new();
        let mut cleanup = Vec::new();
        while let Some(parent) = stack.pop() {
            let path = if parent.is_empty() {
                root.to_owned()
            } else {
                StructureWrite::joined(root, &parent)?
            };
            let mut entries = tokio::fs::read_dir(path).await.map_err(io)?;
            while let Some(entry) = entries.next_entry().await.map_err(io)? {
                let name = entry.file_name().to_string_lossy().into_owned();
                if parent.is_empty() && name == ".git" {
                    continue;
                }
                let relative = if parent.is_empty() {
                    name
                } else {
                    format!("{parent}/{name}")
                };
                if !resumed {
                    unexpected.push(relative);
                    continue;
                }
                let meta = tokio::fs::symlink_metadata(entry.path())
                    .await
                    .map_err(io)?;
                if reparse(&meta) {
                    unexpected.push(relative);
                } else if meta.is_dir() && dirs.contains(&relative) {
                    stack.push(relative);
                } else if meta.is_file() && files.contains_key(relative.as_str()) {
                    let bytes = tokio::fs::read(entry.path()).await.map_err(io)?;
                    if Structure::sha256_hex(&bytes)
                        != Structure::sha256_hex(files[relative.as_str()])
                    {
                        unexpected.push(relative);
                    }
                } else if meta.is_file() && temps.contains(&relative) {
                    cleanup.push(entry.path());
                } else {
                    unexpected.push(relative);
                }
            }
        }
        if !unexpected.is_empty() {
            unexpected.sort();
            let reason = if resumed {
                "unexpected files"
            } else {
                "directory not empty"
            };
            return Err(blocked(format!(
                "{reason}: {}",
                unexpected
                    .into_iter()
                    .take(10)
                    .collect::<Vec<_>>()
                    .join(", ")
            )));
        }
        Ok(cleanup)
    }
}

impl StructureWrite {
    pub(super) async fn write(
        root: &Path,
        plan: &Planned,
        cleanup: Vec<PathBuf>,
        stop: Option<usize>,
    ) -> Result<(), crate::DesignError> {
        for path in cleanup {
            tokio::fs::remove_file(path).await.map_err(io)?;
        }
        let mut dirs: Vec<_> = directories(plan).into_iter().collect();
        dirs.sort_by_key(|p| (p.matches('/').count(), p.clone()));
        for dir in dirs {
            let path = StructureWrite::joined(root, &dir)?;
            match tokio::fs::create_dir(&path).await {
                Ok(()) => {}
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                    let meta = tokio::fs::symlink_metadata(path).await.map_err(io)?;
                    if !meta.is_dir() || reparse(&meta) {
                        return Err(blocked(format!("unexpected files: {dir}")));
                    }
                }
                Err(e) => return Err(io(e)),
            }
        }
        let mut written = 0;
        let temps = temporary_paths(plan);
        for file in &plan.files {
            let path = StructureWrite::joined(root, &file.path)?;
            match tokio::fs::symlink_metadata(&path).await {
                Ok(meta) => {
                    if !meta.is_file()
                        || reparse(&meta)
                        || tokio::fs::read(&path).await.map_err(io)? != file.bytes
                    {
                        return Err(blocked(format!("unexpected files: {}", file.path)));
                    }
                    continue;
                }
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => return Err(io(e)),
            }
            let temp = StructureWrite::joined(root, &temps[file.path.as_str()])?;
            let mut output = tokio::fs::OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(&temp)
                .await
                .map_err(io)?;
            output.write_all(&file.bytes).await.map_err(io)?;
            output.flush().await.map_err(io)?;
            output.sync_all().await.map_err(io)?;
            drop(output);
            if stop == Some(written) {
                return Err(
                    StorageError::Unavailable("test: stopped structure write".into()).into(),
                );
            }
            // A same-directory hard link publishes atomically without overwriting a file
            // added after preflight (rename replaces destinations on Unix).
            tokio::fs::hard_link(&temp, &path).await.map_err(|e| {
                if e.kind() == std::io::ErrorKind::AlreadyExists {
                    blocked(format!("unexpected files: {}", file.path))
                } else {
                    io(e)
                }
            })?;
            tokio::fs::remove_file(temp).await.map_err(io)?;
            written += 1;
        }
        Ok(())
    }
}

pub struct StructureWrite;
