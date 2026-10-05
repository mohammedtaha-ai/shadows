//! One job: walking a project's folder and indexing what changed (spec §15.4
//! steps 1–6).
//!
//! Two functions hold the rules, and every path that indexes goes through
//! them: `keeps`, the walk's filter, and `index_file`, one file read, parsed
//! and written. The scan, the watcher's batches and a question's re-check all
//! call them; a second copy of either would drift, and the index would then
//! depend on which path saw the change.

use std::collections::HashSet;
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::time::{Instant, UNIX_EPOCH};

use shadows_index::{extract, language_for};
use tokio::io::AsyncReadExt;

use super::Code;
use super::store::FileRow;
use crate::error::CoreError;
use crate::projects::ProjectId;

/// A file over this many bytes is not parsed: `too_large`.
const MAX_BYTES: u64 = 1_048_576;
/// A NUL byte in this many first bytes makes a file `binary`.
const BINARY_PROBE: usize = 8 * 1024;

/// How far a running scan is, and whether its worker is stopping. A scan
/// checks `stop` between files, so it ends after the file it is on (§15.8).
#[derive(Default)]
pub(super) struct Progress {
    pub(super) found: AtomicU32,
    pub(super) done: AtomicU32,
    pub(super) stop: AtomicBool,
}

/// Folders never walked, with or without a `.gitignore`.
pub(super) fn never_walked(name: &str) -> bool {
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
pub(super) async fn folder_exists(dir: &Path) -> bool {
    tokio::fs::metadata(dir).await.is_ok_and(|m| m.is_dir())
}

/// A file's size and modified time in milliseconds, as `code_file` stores them.
pub(super) fn stamp(meta: &std::fs::Metadata) -> (i64, i64) {
    let modified_ms = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map_or(0, |d| d.as_millis() as i64);
    (meta.len() as i64, modified_ms)
}

/// What the walk found: every kept file relative to the folder with '/', and
/// whether any entry could not be read.
struct Walked {
    paths: Vec<String>,
    incomplete: bool,
}

/// The paths a walk is limited to: those files, and the folders above them.
struct Only {
    files: HashSet<String>,
    folders: HashSet<String>,
}

/// A path relative to `dir`, with '/'; `None` outside it or not UTF-8.
pub(super) fn relative(dir: &Path, path: &Path) -> Option<String> {
    let parts: Option<Vec<&str>> = path
        .strip_prefix(dir)
        .ok()?
        .components()
        .map(|c| c.as_os_str().to_str())
        .collect();
    Some(parts?.join("/"))
}

/// Step 1, blocking: the folder's own ignore files, no parent's and no
/// global one, no link followed, nothing hidden (`WalkBuilder`'s default).
/// With `only`, it enters only the folders above those files, so it reads
/// the same ignore files a full walk would for them, and nothing else.
fn walk(dir: &Path, only: Option<Arc<Only>>) -> Walked {
    let mut out = Walked {
        paths: Vec::new(),
        incomplete: false,
    };
    let root = dir.to_path_buf();
    let walker = ignore::WalkBuilder::new(dir)
        .parents(false)
        .git_global(false)
        .require_git(false)
        .follow_links(false)
        .filter_entry(move |e| {
            let is_dir = e.file_type().is_some_and(|t| t.is_dir());
            if is_dir && e.file_name().to_str().is_some_and(never_walked) {
                return false;
            }
            let Some(only) = &only else {
                return true;
            };
            let Some(key) = relative(&root, e.path()).map(|r| path_key(&r)) else {
                return false;
            };
            if is_dir {
                only.folders.contains(&key)
            } else {
                only.files.contains(&key)
            }
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
        let Ok(rel) = entry.path().strip_prefix(dir) else {
            continue;
        };
        if !keeps(rel) {
            continue;
        }
        if let Some(path) = relative(dir, entry.path()) {
            out.paths.push(path);
        }
    }
    out
}

/// Of `paths`, the keys of those the walk would keep now: on disk, passing
/// `keeps`, and named by no ignore file on their way (blocking).
pub(super) fn kept_now(dir: &Path, paths: &[String]) -> HashSet<String> {
    let files: HashSet<String> = paths.iter().map(|p| path_key(p)).collect();
    let mut folders = HashSet::from([String::new()]);
    for f in &files {
        let mut at = f.as_str();
        while let Some((up, _)) = at.rsplit_once('/') {
            folders.insert(up.to_string());
            at = up;
        }
    }
    let walked = walk(dir, Some(Arc::new(Only { files, folders })));
    walked.paths.iter().map(|p| path_key(p)).collect()
}

/// Whether the file `real` (canonical) is `path` inside `canonical_dir`,
/// component for component: a link or a junction on the way, or a name
/// Windows normalises (`a.rs.`), names another file, which is not opened.
fn names_itself(real: &Path, canonical_dir: &Path, path: &str) -> bool {
    relative(canonical_dir, real).is_some_and(|r| path_key(&r) == path_key(path))
}

impl Code {
    /// Steps 2–5 for one file, `path` relative to `dir` with '/': skipped
    /// when its path, size and modified time equal its row, else read, parsed
    /// on a blocking thread and written in one transaction. A file that is
    /// gone, or is no longer a regular file (a link is never followed), has
    /// its rows deleted, but only while the folder itself exists (§15.4): a
    /// folder that vanished keeps its index. Before it opens the file, its
    /// canonical path must be `path` inside the folder's canonical path, or
    /// it is skipped and logged. On failure the file keeps its old row, so
    /// the next scan does it again. `true` when it wrote.
    pub(super) async fn index_file(
        &self,
        project: &ProjectId,
        dir: &Path,
        path: &str,
    ) -> anyhow::Result<bool> {
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
                return Ok(true);
            }
            return Ok(false);
        };
        let Some(language) = language_for(Path::new(path)) else {
            return Ok(false);
        };
        let (size, modified_ms) = stamp(&meta);
        let stored = (path.to_string(), size, modified_ms);
        if storage.code_file_stamp(project, &key).await? == Some(stored) {
            return Ok(false);
        }
        let canonical_dir = tokio::fs::canonicalize(dir).await?;
        let real = tokio::fs::canonicalize(&full).await?;
        if !names_itself(&real, &canonical_dir, path) {
            tracing::warn!(
                project = project.as_str(),
                path = %path,
                real = %real.display(),
                "code.outside_folder"
            );
            return Ok(false);
        }
        // At most one byte past the cap is read: a file that grew past it
        // since its size was taken is too_large, and the next scan sees its
        // new stamp.
        let bytes = if size as u64 > MAX_BYTES {
            None
        } else {
            let mut bytes = Vec::new();
            let file = tokio::fs::File::open(&real).await?;
            file.take(MAX_BYTES + 1).read_to_end(&mut bytes).await?;
            Some(bytes).filter(|b| b.len() as u64 <= MAX_BYTES)
        };
        let (skipped, tags) = match bytes {
            None => (Some("too_large"), Vec::new()),
            Some(b) if b[..b.len().min(BINARY_PROBE)].contains(&0) => (Some("binary"), Vec::new()),
            Some(b) => match String::from_utf8(b) {
                Err(_) => (Some("not_utf8"), Vec::new()),
                Ok(text) => {
                    let tags =
                        tokio::task::spawn_blocking(move || extract(language, &text)).await?;
                    (None, tags)
                }
            },
        };
        let row = FileRow {
            path_key: key,
            path: path.to_string(),
            size,
            modified_ms,
            language: language.name,
            skipped,
        };
        storage.write_code_file(project, row, tags).await?;
        Ok(true)
    }

    /// Steps 1–6 on the project's folder, one file after another, reporting
    /// into `progress` and ending early when it says stop. A project with no
    /// folder, or whose folder is missing, keeps its index untouched. Logs
    /// `code.scan`; answers how many files it wrote or deleted.
    pub(super) async fn scan_project(
        &self,
        project: &ProjectId,
        progress: &Progress,
    ) -> Result<u32, CoreError> {
        let started = Instant::now();
        let storage = &self.inner.storage;
        let Some(directory) = storage.get_project(project).await?.directory else {
            return Ok(0);
        };
        let dir = PathBuf::from(directory);
        if !folder_exists(&dir).await {
            return Ok(0);
        }
        let walked = {
            let dir = dir.clone();
            match tokio::task::spawn_blocking(move || walk(&dir, None)).await {
                Ok(walked) => walked,
                Err(error) => {
                    tracing::warn!(project = project.as_str(), error = %error, "code.walk_failed");
                    return Ok(0);
                }
            }
        };
        progress
            .found
            .store(walked.paths.len() as u32, Ordering::Relaxed);
        progress.done.store(0, Ordering::Relaxed);
        let mut seen = HashSet::new();
        let mut indexed = 0;
        for path in &walked.paths {
            if progress.stop.load(Ordering::Relaxed) {
                return Ok(indexed);
            }
            seen.insert(path_key(path));
            match self.index_file(project, &dir, path).await {
                Ok(wrote) => indexed += u32::from(wrote),
                Err(error) => {
                    tracing::warn!(
                        project = project.as_str(),
                        path = %path,
                        error = %error,
                        "code.index_failed"
                    )
                }
            }
            progress.done.fetch_add(1, Ordering::Relaxed);
        }
        // Step 6. A walk that could not read an entry may have missed files
        // that are still there, so it deletes nothing; the next scan does.
        // Nor does a folder that vanished during the scan: its index is kept.
        if walked.incomplete || !folder_exists(&dir).await {
            tracing::warn!(project = project.as_str(), "code.walk_incomplete");
            return Ok(indexed);
        }
        for key in storage.code_file_keys(project).await? {
            if seen.contains(&key) {
                continue;
            }
            match storage.delete_code_file(project, &key).await {
                Ok(()) => indexed += 1,
                Err(error) => {
                    tracing::warn!(
                        project = project.as_str(),
                        path = %key,
                        error = %error,
                        "code.index_failed"
                    )
                }
            }
        }
        tracing::info!(
            project = project.as_str(),
            seen = walked.paths.len(),
            indexed,
            elapsed_ms = started.elapsed().as_millis() as u64,
            "code.scan"
        );
        Ok(indexed)
    }
}

impl Code {
    /// Of a question's hit files, those whose size or modified time differ
    /// from their rows, or that are gone or no longer regular files (§15.5).
    /// It stats them and opens none. A folder that is missing answers none:
    /// its index is kept as it is.
    pub(super) async fn stale<'a>(
        &self,
        project: &ProjectId,
        dir: &Path,
        paths: impl IntoIterator<Item = &'a str>,
    ) -> Vec<String> {
        if !folder_exists(dir).await {
            return Vec::new();
        }
        let mut changed = Vec::new();
        for path in paths {
            let stored = self
                .inner
                .storage
                .code_file_stamp(project, &path_key(path))
                .await;
            let on_disk = tokio::fs::symlink_metadata(dir.join(path))
                .await
                .ok()
                .filter(std::fs::Metadata::is_file)
                .map(|m| stamp(&m));
            match (stored, on_disk) {
                (Ok(Some((_, size, ms))), Some(now)) if (size, ms) == now => {}
                _ => changed.push(path.to_string()),
            }
        }
        changed
    }
}
