//! One job: walking a project's folder and indexing what changed (spec §15.4
//! steps 1–6).
//!
//! Two functions hold the rules, and every path that indexes goes through
//! them: `keeps`, the walk's filter, and `index_file`, one file read, parsed
//! and written. A second copy of either would drift, and the index would then
//! depend on which path saw the change.

use std::collections::HashSet;
use std::path::{Component, Path, PathBuf};
use std::time::UNIX_EPOCH;

use shadows_index::{extract, language_for};

use super::Code;
use super::store::FileRow;
use crate::error::CoreError;
use crate::projects::ProjectId;

/// A file over this many bytes is not parsed: `too_large`.
const MAX_BYTES: u64 = 1_048_576;
/// A NUL byte in this many first bytes makes a file `binary`.
const BINARY_PROBE: usize = 8 * 1024;

/// Folders never walked, with or without a `.gitignore`.
fn never_walked(name: &str) -> bool {
    name == "target" || name == "node_modules"
}

/// The walk's filter for a path relative to the project's folder: nothing
/// under `target/` or `node_modules/`, nothing hidden, and only a language
/// the table knows. The ignore files are the walk's own.
pub(super) fn keeps(path: &Path) -> bool {
    let visible = path.components().all(|c| match c {
        Component::Normal(n) => n
            .to_str()
            .is_some_and(|n| !never_walked(n) && !n.starts_with('.')),
        _ => false,
    });
    visible && language_for(path).is_some()
}

/// The key a path is stored under: on Windows `Foo.rs` and `foo.rs` are one file.
pub(super) fn path_key(path: &str) -> String {
    if cfg!(windows) {
        path.to_lowercase()
    } else {
        path.to_string()
    }
}

/// Whether the project's folder is there: rows are deleted only while it is.
async fn folder_exists(dir: &Path) -> bool {
    tokio::fs::metadata(dir).await.is_ok_and(|m| m.is_dir())
}

/// What the walk found: every kept file relative to the folder with '/', and
/// whether any entry could not be read.
struct Walked {
    paths: Vec<String>,
    incomplete: bool,
}

/// Step 1, blocking: the folder's own ignore files, no parent's and no
/// global one, no link followed, nothing hidden (`WalkBuilder`'s default).
fn walk(dir: &Path) -> Walked {
    let mut out = Walked {
        paths: Vec::new(),
        incomplete: false,
    };
    let walker = ignore::WalkBuilder::new(dir)
        .parents(false)
        .git_global(false)
        .require_git(false)
        .follow_links(false)
        .filter_entry(|e| {
            let is_dir = e.file_type().is_some_and(|t| t.is_dir());
            !(is_dir && e.file_name().to_str().is_some_and(never_walked))
        })
        .build();
    for entry in walker {
        let Ok(entry) = entry else {
            out.incomplete = true;
            continue;
        };
        if !entry.file_type().is_some_and(|t| t.is_file()) {
            continue;
        }
        let Ok(relative) = entry.path().strip_prefix(dir) else {
            continue;
        };
        if !keeps(relative) {
            continue;
        }
        let parts: Option<Vec<&str>> = relative
            .components()
            .map(|c| c.as_os_str().to_str())
            .collect();
        if let Some(parts) = parts {
            out.paths.push(parts.join("/"));
        }
    }
    out
}

impl Code {
    /// Steps 2–5 for one file, `path` relative to `dir` with '/': skipped
    /// when its path, size and modified time equal its row, else read, parsed
    /// on a blocking thread and written in one transaction. A file that is
    /// gone, or is no longer a regular file (a link is never followed), has
    /// its rows deleted, but only while the folder itself exists (§15.4): a
    /// folder that vanished keeps its index. On failure the file keeps its
    /// old row, so the next scan does it again.
    pub(super) async fn index_file(
        &self,
        project: &ProjectId,
        dir: &Path,
        path: &str,
    ) -> anyhow::Result<()> {
        let storage = &self.inner.storage;
        let key = path_key(path);
        let full = dir.join(path);
        let meta = match tokio::fs::symlink_metadata(&full).await {
            Ok(m) if m.is_file() => Some(m),
            Ok(_) => None,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
            Err(e) => return Err(e.into()),
        };
        let Some(meta) = meta else {
            if folder_exists(dir).await {
                storage.delete_code_file(project, &key).await?;
            }
            return Ok(());
        };
        let Some(language) = language_for(Path::new(path)) else {
            return Ok(());
        };
        let size = meta.len();
        let modified_ms = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map_or(0, |d| d.as_millis() as i64);
        let stamp = (path.to_string(), size as i64, modified_ms);
        if storage.code_file_stamp(project, &key).await? == Some(stamp) {
            return Ok(());
        }
        let (skipped, tags) = if size > MAX_BYTES {
            (Some("too_large"), Vec::new())
        } else {
            let bytes = tokio::fs::read(&full).await?;
            if bytes[..bytes.len().min(BINARY_PROBE)].contains(&0) {
                (Some("binary"), Vec::new())
            } else {
                match String::from_utf8(bytes) {
                    Err(_) => (Some("not_utf8"), Vec::new()),
                    Ok(text) => {
                        let tags =
                            tokio::task::spawn_blocking(move || extract(language, &text)).await?;
                        (None, tags)
                    }
                }
            }
        };
        let row = FileRow {
            path_key: key,
            path: path.to_string(),
            size: size as i64,
            modified_ms,
            language: language.name,
            skipped,
        };
        Ok(storage.write_code_file(project, row, tags).await?)
    }

    /// Steps 1–6 on the project's folder, one file after another. A project
    /// with no folder, or whose folder is missing, keeps its index untouched.
    pub(super) async fn scan_project(&self, project: &ProjectId) -> Result<(), CoreError> {
        let storage = &self.inner.storage;
        let Some(directory) = storage.get_project(project).await?.directory else {
            return Ok(());
        };
        let dir = PathBuf::from(directory);
        if !folder_exists(&dir).await {
            return Ok(());
        }
        let walked = {
            let dir = dir.clone();
            match tokio::task::spawn_blocking(move || walk(&dir)).await {
                Ok(walked) => walked,
                Err(error) => {
                    tracing::warn!(project = project.as_str(), error = %error, "code.walk_failed");
                    return Ok(());
                }
            }
        };
        let mut seen = HashSet::new();
        for path in &walked.paths {
            seen.insert(path_key(path));
            if let Err(error) = self.index_file(project, &dir, path).await {
                tracing::warn!(project = project.as_str(), path = %path, error = %error, "code.index_failed");
            }
        }
        // Step 6. A walk that could not read an entry may have missed files
        // that are still there, so it deletes nothing; the next scan does.
        // Nor does a folder that vanished during the scan: its index is kept.
        if walked.incomplete || !folder_exists(&dir).await {
            tracing::warn!(project = project.as_str(), "code.walk_incomplete");
            return Ok(());
        }
        for key in storage.code_file_keys(project).await? {
            if !seen.contains(&key)
                && let Err(error) = storage.delete_code_file(project, &key).await
            {
                tracing::warn!(project = project.as_str(), path = %key, error = %error, "code.index_failed");
            }
        }
        Ok(())
    }
}
