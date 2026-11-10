//! One job: the paged file inventory of a project's folder.
//!
//! The listing names every path a person would expect to see, whatever its
//! language; it never reads a file's content. A folder never walked, a path
//! an ignore file names, a secret's name and a link are each one `excluded`
//! entry, never entered, so a huge dependency folder is one line.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::time::Duration;

use shadows_index::language_for;

use super::Code;
use super::model::{InventoryPage, InventoryQuery, InventoryState};
use super::scope::Asker;
use super::store::InventoryRow;
use super::watch::Job;
use crate::CodeError;
use core_db::Storage;
use core_projects::ProjectId;

/// How long a refresh waits for the worker's scan before it answers anyway.
const REFRESH_WAIT: Duration = Duration::from_secs(30);
/// The most entries one page carries.
const MAX_LIMIT: u16 = 200;

/// What one listing found, and whether any entry could not be read.
struct Listed {
    rows: Vec<InventoryRow>,
    incomplete: bool,
}

/// A name whose file holds a secret: its name is listed, its content never.
fn secret_name(name: &str) -> bool {
    let n = name.to_lowercase();
    n == ".env"
        || (n.starts_with(".env.") && !n.ends_with(".example"))
        || [".pem", ".key", ".p12", ".pfx"]
            .iter()
            .any(|s| n.ends_with(s))
        || n == "id_rsa"
        || n == "id_ed25519"
}

/// Lists `dir` (blocking). The ignore walk decides what is ignored; the
/// folder's own entries, read one folder at a time, decide what is listed.
fn listed(dir: &Path) -> Listed {
    let mut incomplete = false;
    let mut seen = HashSet::new();
    let walk = Code::walker(dir, false)
        .filter_entry(|e| {
            let is_dir = e.file_type().is_some_and(|t| t.is_dir());
            let name = e.file_name().to_str();
            !(is_dir && name.is_some_and(|n| n == ".git" || Code::never_walked(n)))
        })
        .build();
    for entry in walk {
        match entry {
            Ok(entry) => {
                if let Some(rel) = Code::relative(dir, entry.path()).filter(|r| !r.is_empty()) {
                    seen.insert(Code::path_key(&rel));
                }
            }
            Err(_) => incomplete = true,
        }
    }
    let mut rows = Vec::new();
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
            let key = Code::path_key(&path);
            let listed = if name == ".git" {
                None
            } else if kind.is_dir() && Code::never_walked(&name) {
                Some(("excluded", Some("dependencies")))
            } else if secret_name(&name) {
                Some(("excluded", Some("secret")))
            } else if kind.is_symlink() {
                Some(("excluded", Some("link")))
            } else if !seen.contains(&key) {
                Some(("excluded", Some("ignored")))
            } else if kind.is_dir() {
                folders.push(path.clone());
                Some(("directory", None))
            } else if kind.is_file() {
                Some(("file", None))
            } else {
                None
            };
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
    Listed { rows, incomplete }
}

impl Code {
    /// One page of the asker's own project's file list, in path order.
    pub async fn inventory(
        &self,
        asker: Asker<'_>,
        query: InventoryQuery,
    ) -> Result<InventoryPage, CodeError> {
        if !(1..=MAX_LIMIT).contains(&query.limit) {
            return Err(CodeError::InvalidCommand("limit must be 1 to 200".into()));
        }
        let prefix = Code::path_key(&Code::inside(&query.prefix)?);
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
        if !Code::folder_exists(Path::new(&directory)).await {
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
            return Err(CodeError::InventoryStale);
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
                        language_for(Path::new(&entry.path)).map(|l| l.name.to_string());
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

    /// Lists the folder again now, then answers the first page.
    pub async fn refresh_inventory(&self, asker: Asker<'_>) -> Result<InventoryPage, CodeError> {
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
    fn cursor(after: &str, prefix: &str) -> Result<(i64, String), CodeError> {
        let refuse = || CodeError::InvalidCommand("the cursor is not one this listing gave".into());
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
    /// answered, so a scan never fails because of its listing.
    pub(super) async fn record_inventory(&self, project: &ProjectId, dir: &Path) {
        let owned: PathBuf = dir.to_path_buf();
        let found = match tokio::task::spawn_blocking(move || listed(&owned)).await {
            Ok(found) => found,
            Err(error) => {
                tracing::warn!(project = project.as_str(), error = %error, "code.inventory_failed");
                return;
            }
        };
        // A folder that vanished while it was listed keeps its stored list.
        if !Code::folder_exists(dir).await {
            return;
        }
        let store = &self.inner.store;
        if let Err(error) = store
            .write_inventory(project, found.rows, found.incomplete, &Storage::now())
            .await
        {
            tracing::warn!(project = project.as_str(), error = %error, "code.inventory_failed");
        }
    }
}

#[cfg(test)]
#[path = "../tests/inventory/invariants.rs"]
mod tests;
