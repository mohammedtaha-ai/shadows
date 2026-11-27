//! One job: the paged file inventory of a project's folder.
//!
//! The listing names every path a person would expect to see, whatever its
//! language; it never reads a file's content. A folder never walked, a path
//! an ignore file names, a secret's name and a link are each one `excluded`
//! entry, never entered, so a huge dependency folder is one line.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use super::Codebase;
use super::model::{InventoryPage, InventoryQuery, InventoryState};
use super::scan::Only;
use super::scope::Asker;
use super::store::InventoryRow;
use super::watch::Job;
use crate::CodebaseError;
use core_db::Storage;
use core_projects::ProjectId;

/// How long a refresh waits for the worker's scan before it answers anyway.
const REFRESH_WAIT: Duration = Duration::from_secs(30);
/// The most entries one page carries.
const MAX_LIMIT: u16 = 200;

/// What one listing found, and whether any entry could not be read.
struct Listed {
    rows: Vec<InventoryRow>,
    /// Each listed file's size and modified time, by its path.
    stamps: HashMap<String, (i64, i64)>,
    incomplete: bool,
}

/// The files one listing found, with their stamps: what a project's worker
/// compares with its last scan to tell the language servers what changed.
pub(super) struct FileStamps {
    files: HashMap<String, (i64, i64)>,
    complete: bool,
}

impl FileStamps {
    /// The paths added, changed or removed since `known`, which becomes this
    /// listing. An incomplete listing removes nothing: a file it could not read
    /// may still be there.
    pub(super) fn follow(self, known: &mut HashMap<String, (i64, i64)>) -> Vec<String> {
        let mut changed: Vec<String> = self
            .files
            .iter()
            .filter(|(path, stamp)| known.get(*path) != Some(*stamp))
            .map(|(path, _)| path.clone())
            .collect();
        if self.complete {
            changed.extend(
                known
                    .keys()
                    .filter(|path| !self.files.contains_key(*path))
                    .cloned(),
            );
            *known = self.files;
        } else {
            known.extend(self.files);
        }
        changed
    }
}

impl Codebase {
    /// A name whose file holds a secret: its name is listed, its content never.
    /// A required input is refused by the same rule.
    pub(crate) fn secret_name(name: &str) -> bool {
        let n = name.to_lowercase();
        n == ".env"
            || (n.starts_with(".env.") && !n.ends_with(".example"))
            || [".pem", ".key", ".p12", ".pfx"]
                .iter()
                .any(|s| n.ends_with(s))
            || n == "id_rsa"
            || n == "id_ed25519"
    }
}

/// The keys the listing's ignore walk yields (blocking), limited to `only`
/// when given, and whether an entry could not be read.
fn walked(dir: &Path, only: Option<Arc<Only>>) -> (HashSet<String>, bool) {
    let mut incomplete = false;
    let mut seen = HashSet::new();
    let root = dir.to_path_buf();
    let walk = Codebase::walker(dir, false)
        .filter_entry(move |e| {
            let is_dir = e.file_type().is_some_and(|t| t.is_dir());
            let name = e.file_name().to_str();
            !(is_dir && name.is_some_and(|n| n == ".git" || Codebase::never_walked(n)))
                && only
                    .as_ref()
                    .is_none_or(|only| only.holds(&root, e.path(), is_dir))
        })
        .build();
    for entry in walk {
        match entry {
            Ok(entry) => {
                if let Some(rel) = Codebase::relative(dir, entry.path()).filter(|r| !r.is_empty()) {
                    seen.insert(Codebase::path_key(&rel));
                }
            }
            Err(_) => incomplete = true,
        }
    }
    (seen, incomplete)
}

/// How the listing shows one entry, by the first rule that decides: its
/// kind and why it is excluded; `None` for one it does not list.
fn entry_kind(
    name: &str,
    kind: std::fs::FileType,
    walked: bool,
) -> Option<(&'static str, Option<&'static str>)> {
    if name == ".git" {
        None
    } else if kind.is_dir() && Codebase::never_walked(name) {
        Some(("excluded", Some("dependencies")))
    } else if Codebase::secret_name(name) {
        Some(("excluded", Some("secret")))
    } else if kind.is_symlink() {
        Some(("excluded", Some("link")))
    } else if !walked {
        Some(("excluded", Some("ignored")))
    } else if kind.is_dir() {
        Some(("directory", None))
    } else if kind.is_file() {
        Some(("file", None))
    } else {
        None
    }
}

impl Codebase {
    /// Of `paths`, the keys of those a listing made now would hold as files
    /// (blocking): each folder on the way listed as a directory, the path
    /// itself as a file. A folder, an ignored path, a secret, a link or a path
    /// not on disk is not among them.
    pub(super) fn listed_now(dir: &Path, paths: &[String]) -> HashSet<String> {
        let (seen, _) = walked(dir, Some(Only::over(paths)));
        let file = |path: &str| {
            let parts: Vec<&str> = path.split('/').collect();
            (1..=parts.len()).all(|depth| {
                let at = parts[..depth].join("/");
                let Ok(meta) = std::fs::symlink_metadata(dir.join(&at)) else {
                    return false;
                };
                let walked = seen.contains(&Codebase::path_key(&at));
                let want = if depth == parts.len() {
                    "file"
                } else {
                    "directory"
                };
                entry_kind(parts[depth - 1], meta.file_type(), walked)
                    .is_some_and(|(kind, _)| kind == want)
            })
        };
        paths
            .iter()
            .filter(|p| !p.is_empty() && file(p))
            .map(|p| Codebase::path_key(p))
            .collect()
    }
}

/// Lists `dir` (blocking). The ignore walk decides what is ignored; the
/// folder's own entries, read one folder at a time, decide what is listed.
fn listed(dir: &Path) -> Listed {
    let (seen, mut incomplete) = walked(dir, None);
    let mut rows = Vec::new();
    let mut stamps = HashMap::new();
    let mut folders = vec![String::new()];
    while let Some(folder) = folders.pop() {
        let Ok(children) = std::fs::read_dir(dir.join(&folder)) else {
            incomplete = true;
            continue;
        };
        for child in children {
            let Ok(child) = child else {
                incomplete = true;
                continue;
            };
            let Ok(name) = child.file_name().into_string() else {
                continue;
            };
            let Ok(kind) = child.file_type() else {
                incomplete = true;
                continue;
            };
            let path = if folder.is_empty() {
                name.clone()
            } else {
                format!("{folder}/{name}")
            };
            let key = Codebase::path_key(&path);
            let listed = entry_kind(&name, kind, seen.contains(&key));
            match listed {
                Some(("directory", _)) => folders.push(path.clone()),
                Some(("file", _)) => {
                    // Asked of the file itself: Windows' directory entry can lag a write.
                    if let Ok(meta) = std::fs::symlink_metadata(dir.join(&path)) {
                        stamps.insert(path.clone(), Codebase::stamp(&meta));
                    }
                }
                _ => {}
            }
            if let Some((kind, excluded_reason)) = listed {
                rows.push(InventoryRow {
                    path_key: key,
                    path,
                    kind,
                    excluded_reason,
                });
            }
        }
    }
    Listed {
        rows,
        stamps,
        incomplete,
    }
}

impl Codebase {
    /// One page of the asker's own project's file list, in path order.
    pub async fn inventory(
        &self,
        asker: Asker<'_>,
        query: InventoryQuery,
    ) -> Result<InventoryPage, CodebaseError> {
        if !(1..=MAX_LIMIT).contains(&query.limit) {
            return Err(CodebaseError::InvalidCommand(
                "limit must be 1 to 200".into(),
            ));
        }
        let prefix = Codebase::path_key(&Codebase::inside(&query.prefix)?);
        let project = self.inner.projects.get(asker.home()).await?;
        self.touch(asker.home()).await;
        let empty = |state| InventoryPage {
            state,
            generation: 0,
            entries: Vec::new(),
            next: None,
            observed_at: None,
        };
        let Some(directory) = project.directory else {
            return Ok(empty(InventoryState::Missing));
        };
        if !Codebase::folder_exists(Path::new(&directory)).await {
            return Ok(empty(InventoryState::Missing));
        }
        let cursor = match &query.after {
            None => None,
            Some(after) => Some(Self::cursor(after, &prefix)?),
        };
        let after_key = cursor.as_ref().map(|(_, key)| key.as_str());
        let limit = i64::from(query.limit);
        let store = &self.inner.store;
        let Some((generation, incomplete, observed_at, mut rows)) = store
            .inventory_page(&project.id, &prefix, after_key, limit + 1)
            .await?
        else {
            return Ok(empty(InventoryState::Indexing));
        };
        if cursor.is_some_and(|(g, _)| g != generation) {
            return Err(CodebaseError::InventoryStale);
        }
        let more = rows.len() as i64 > limit;
        rows.truncate(query.limit as usize);
        let next = if more {
            rows.last().map(|(key, _)| format!("{generation}:{key}"))
        } else {
            None
        };
        let entries = rows
            .into_iter()
            .map(|(_, mut entry)| {
                if entry.kind == super::model::InventoryKind::File {
                    entry.language =
                        Codebase::language_for(Path::new(&entry.path)).map(str::to_string);
                }
                entry
            })
            .collect();
        Ok(InventoryPage {
            state: if incomplete {
                InventoryState::Incomplete
            } else {
                InventoryState::Ready
            },
            generation: generation as u64,
            entries,
            next,
            observed_at: Some(observed_at),
        })
    }

    /// Which of `paths` the project's file list holds as files, without paging
    /// the whole list. `None` when the list cannot answer: the project has no
    /// folder, or its first listing has not finished. A path the list leaves
    /// out (ignored, secret, a link) is not a file here.
    pub async fn files_present(
        &self,
        project: &ProjectId,
        paths: &[String],
    ) -> Result<Option<HashSet<String>>, CodebaseError> {
        let project = self.inner.projects.get(project).await?;
        let Some(directory) = project.directory else {
            return Ok(None);
        };
        if !Codebase::folder_exists(Path::new(&directory)).await {
            return Ok(None);
        }
        let store = &self.inner.store;
        let mut present = HashSet::new();
        for path in paths {
            let Ok(inside) = Codebase::inside(path) else {
                continue;
            };
            let key = Codebase::path_key(&inside);
            let Some((_, _, _, rows)) = store.inventory_page(&project.id, &key, None, 1).await?
            else {
                return Ok(None);
            };
            let file = rows.first().is_some_and(|(found, entry)| {
                *found == key && entry.kind == super::model::InventoryKind::File
            });
            if file {
                present.insert(path.clone());
            }
        }
        Ok(Some(present))
    }

    /// Lists the folder again now, then answers the first page.
    pub async fn refresh_inventory(
        &self,
        asker: Asker<'_>,
    ) -> Result<InventoryPage, CodebaseError> {
        let project = self.inner.projects.get(asker.home()).await?.id;
        self.touch(&project).await;
        let worker = self
            .inner
            .active
            .lock()
            .await
            .worker(&project)
            .map(|w| w.jobs());
        match worker {
            Some(worker) => {
                let wait = async {
                    let (reply, done) = tokio::sync::oneshot::channel();
                    if worker.send(Job::Refresh(reply)).await.is_ok() {
                        let _ = done.await;
                    }
                };
                if tokio::time::timeout(REFRESH_WAIT, wait).await.is_err() {
                    tracing::warn!(project = project.as_str(), "code.refresh_timeout");
                }
            }
            None => self.scan(&project).await?,
        }
        self.inventory(asker, InventoryQuery::default()).await
    }

    /// `"<generation>:<path_key>"`, its key at or under `prefix`, or the refusal.
    fn cursor(after: &str, prefix: &str) -> Result<(i64, String), CodebaseError> {
        let refuse =
            || CodebaseError::InvalidCommand("the cursor is not one this listing gave".into());
        let (generation, key) = after.split_once(':').ok_or_else(refuse)?;
        let generation: i64 = generation.parse().map_err(|_| refuse())?;
        let under = prefix.is_empty()
            || key == prefix
            || key.strip_prefix(prefix).is_some_and(|r| r.starts_with('/'));
        if !under || key.is_empty() {
            return Err(refuse());
        }
        Ok((generation, key.to_string()))
    }

    /// Lists `dir` and stores what it found; a failure is logged, never
    /// answered, so a scan never fails because of its listing. One listing of
    /// a project runs at a time, so an older listing never writes over a newer
    /// one. Keeps the listed files' extensions for `extensions`. Answers the
    /// listed files' stamps, or `None` when nothing was listed.
    pub(super) async fn record_inventory(
        &self,
        project: &ProjectId,
        dir: &Path,
    ) -> Option<FileStamps> {
        let turn = self.listing_lock(project);
        let _one_at_a_time = turn.lock().await;
        let owned: PathBuf = dir.to_path_buf();
        let found = match tokio::task::spawn_blocking(move || listed(&owned)).await {
            Ok(found) => found,
            Err(error) => {
                tracing::warn!(project = project.as_str(), error = %error, "code.inventory_failed");
                return None;
            }
        };
        // A folder that vanished while it was listed keeps its stored list.
        if !Codebase::folder_exists(dir).await {
            return None;
        }
        let stamps = FileStamps {
            files: found.stamps,
            complete: !found.incomplete,
        };
        let extensions: HashSet<String> = found
            .rows
            .iter()
            .filter(|row| row.kind == "file")
            .filter_map(|row| Codebase::extension(&row.path).map(String::from))
            .collect();
        let store = &self.inner.store;
        self.inner
            .extensions
            .lock()
            .expect("extensions lock")
            .remove(project.as_str());
        match store
            .write_inventory(project, found.rows, found.incomplete, &Storage::now())
            .await
        {
            Ok(()) => {
                let mut held = self.inner.extensions.lock().expect("extensions lock");
                held.insert(project.as_str().to_string(), extensions);
            }
            Err(error) => {
                tracing::warn!(project = project.as_str(), error = %error, "code.inventory_failed");
            }
        }
        Some(stamps)
    }

    /// The extensions of the files the project's last listing found, as
    /// written (`.rs`). Held in memory from the listing; read from the store
    /// once when nothing is held, as after a restart.
    pub(super) async fn extensions(
        &self,
        project: &ProjectId,
    ) -> Result<HashSet<String>, CodebaseError> {
        if let Some(held) = self
            .inner
            .extensions
            .lock()
            .expect("extensions lock")
            .get(project.as_str())
        {
            return Ok(held.clone());
        }
        let read: HashSet<String> = self
            .inner
            .store
            .file_paths(project)
            .await?
            .iter()
            .filter_map(|path| Codebase::extension(path).map(String::from))
            .collect();
        // A listing that ended meanwhile holds the newer answer: keep it.
        let mut held = self.inner.extensions.lock().expect("extensions lock");
        Ok(held
            .entry(project.as_str().to_string())
            .or_insert(read)
            .clone())
    }

    /// The lock that keeps a project's listings one at a time.
    fn listing_lock(&self, project: &ProjectId) -> std::sync::Arc<tokio::sync::Mutex<()>> {
        self.inner
            .listings
            .lock()
            .expect("listings lock")
            .entry(project.as_str().to_string())
            .or_default()
            .clone()
    }
}

#[cfg(test)]
#[path = "../tests/inventory/invariants.rs"]
mod tests;
