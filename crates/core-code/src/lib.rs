//! One job: the code index  — the `Code` service. It walks
//! a project's folder into SQLite, keeps it current while the project is
//! active, and answers three questions from it: where a name is defined, who
//! uses it, and what a path holds.
//!
//! `model` holds the types, `store` the queries, `scan` the walk and the
//! indexing of one file, `scope` who may read what, `watch` one active
//! project's worker, `active` which projects have one, `links` the links
//! and the active limit as commands.

mod active;
mod links;
mod model;
mod scan;
mod scope;
mod store;
mod watch;

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

pub use model::{
    Answer, CodeConfig, CodeSettings, Hit, IndexState, ProjectLink, ProjectStatus, Skipped,
};
pub use scope::Asker;

mod error;
pub use error::CodeError;
#[cfg(feature = "test-support")]
mod testing;
use active::Active;
use core_db::Storage;
use core_projects::{ProjectId, Projects};
use store::{CodeStore, ScopeRow};
use watch::Job;

/// The hits an answer carries at most.
const HITS: usize = 50;
/// How long a question waits for its re-check before it answers anyway.
const RECHECK_WAIT: Duration = Duration::from_secs(2);

struct Inner {
    store: CodeStore,
    projects: Projects,
    active: tokio::sync::Mutex<Active>,
}

/// The code index. Cheap to clone: `ProjectLifecycle`, `Threads` and `Turns` hold a
/// clone to call `touch`, and each worker holds one.
#[derive(Clone)]
pub struct Code {
    inner: Arc<Inner>,
}

impl Code {
    pub fn new(storage: Arc<Storage>, projects: Projects) -> Self {
        Self {
            inner: Arc::new(Inner {
                store: CodeStore::with_storage(storage),
                projects,
                active: Default::default(),
            }),
        }
    }

    /// Projects with a directory, in the owner's explicit activity order.
    async fn code_order(&self) -> Result<Vec<(ProjectId, String)>, CodeError> {
        Ok(self
            .inner
            .projects
            .by_recent_activity()
            .await?
            .into_iter()
            .filter_map(|p| p.directory.map(|dir| (p.id, dir)))
            .collect())
    }

    /// Chooses the active set and starts a worker per active project;
    /// it waits for no scan. `AppCore::start` calls it with the default; a
    /// test calls it when it wants workers.
    pub async fn start(&self, config: CodeConfig) -> Result<(), CodeError> {
        let store = &self.inner.store;
        let order = self.code_order().await?;
        let order = order
            .into_iter()
            .map(|(id, dir)| (id, PathBuf::from(dir)))
            .collect();
        let mut active = self.inner.active.lock().await;
        // Under the lock, as `set_active_limit` reads it: a limit set before
        // `begin` is not lost.
        let limit = store.code_active_limit().await?.max(1) as usize;
        for early in active.begin(order, limit, config) {
            if let Some(rows) = self.used_rows(&early).await {
                active.used(&rows);
            }
        }
        active.settle(self);
        Ok(())
    }

    /// The project was used: its links, then it, become the most
    /// recent, and active; the scan runs in its worker. It never fails its
    /// caller: a failure is logged. Before `start`, it only records the use.
    pub async fn touch(&self, project: &ProjectId) {
        let mut active = self.inner.active.lock().await;
        if !active.running() {
            active.record(project);
            return;
        }
        if let Some(rows) = self.used_rows(project).await {
            active.used(&rows);
            active.settle(self);
        }
    }

    /// The project was removed: its watcher and worker stop, and it
    /// leaves the active set, whose next project takes its place. It never
    /// fails its caller; the rows went in the removal's own write.
    pub async fn forget(&self, project: &ProjectId) {
        let mut active = self.inner.active.lock().await;
        active.forget(project);
        active.settle(self);
    }

    /// Stops every watcher and worker; a worker finishes the file it is on.
    pub async fn shut_down(&self) {
        let tasks = self.inner.active.lock().await.close();
        for task in tasks {
            let _ = task.await;
        }
    }

    /// Test only: `project`'s worker drops every `Files` job from now on.
    #[cfg(feature = "test-support")]
    pub(crate) async fn pause_watcher(&self, project: &ProjectId) {
        if let Some(worker) = self.inner.active.lock().await.worker(project) {
            worker.pause();
        }
    }

    /// The active projects, most recent first.
    #[cfg(feature = "test-support")]
    pub(crate) async fn active_projects(&self) -> Vec<ProjectId> {
        self.inner.active.lock().await.active()
    }

    /// The project and its direct links, each with its folder, or `None` logged.
    async fn used_rows(&self, project: &ProjectId) -> Option<Vec<(ProjectId, Option<PathBuf>)>> {
        match self.inner.store.code_scope(project).await {
            Ok(rows) => Some(
                rows.into_iter()
                    .map(|(id, _, dir)| (id, dir.map(PathBuf::from)))
                    .collect(),
            ),
            // An unknown project, as `Threads::list` may be asked for, is no use.
            Err(core_db::StorageError::NotFound(_)) => None,
            Err(error) => {
                tracing::warn!(project = project.as_str(), error = %error, "code.touch_failed");
                None
            }
        }
    }

    /// Where `name` is defined, exactly and case-sensitively.
    pub async fn definitions(
        &self,
        asker: Asker<'_>,
        only: Option<&str>,
        name: &str,
    ) -> Result<Answer, CodeError> {
        self.by_name(asker, only, name, "definition").await
    }

    /// Who uses `name`: matched by name only, so every hit says `matched_by: name`.
    pub async fn references(
        &self,
        asker: Asker<'_>,
        only: Option<&str>,
        name: &str,
    ) -> Result<Answer, CodeError> {
        let mut answer = self.by_name(asker, only, name, "reference").await?;
        for hit in &mut answer.hits {
            hit.matched_by = Some("name".into());
        }
        Ok(answer)
    }

    /// The definitions at or under `path`, relative to each project's folder;
    /// `""` or `"."` is the whole project. It reads the index and opens no path.
    pub async fn outline(
        &self,
        asker: Asker<'_>,
        only: Option<&str>,
        path: &str,
    ) -> Result<Answer, CodeError> {
        let path = Self::path_key(&Self::inside(path)?);
        let store = &self.inner.store;
        let scope = self.projects(&asker, only).await?;
        self.touch(asker.home()).await;
        let mut rows = store.code_outline(&scope, &path).await?;
        if self.recheck(&scope, &rows).await {
            rows = store.code_outline(&scope, &path).await?;
        }
        self.answer(&scope, rows, Vec::new()).await
    }

    /// How the project's index stands. It does not touch the project.
    pub async fn status(&self, project: &ProjectId) -> Result<ProjectStatus, CodeError> {
        let p = self.inner.projects.get(project).await?;
        self.status_of(&(p.id, p.slug, p.directory)).await
    }

    /// Scans `project` once, now, on the caller's task. Tests call it; a
    /// worker calls `scan_project` itself.
    #[cfg_attr(
        not(feature = "test-support"),
        expect(dead_code, reason = "only tests scan outside a worker")
    )]
    pub(crate) async fn scan(&self, project: &ProjectId) -> Result<(), CodeError> {
        self.scan_project(project, &scan::Progress::default())
            .await
            .map(|_| ())
    }

    async fn by_name(
        &self,
        asker: Asker<'_>,
        only: Option<&str>,
        name: &str,
        role: &str,
    ) -> Result<Answer, CodeError> {
        let store = &self.inner.store;
        let scope = self.projects(&asker, only).await?;
        self.touch(asker.home()).await;
        let mut rows = store.code_by_name(&scope, name, role).await?;
        if self.recheck(&scope, &rows).await {
            rows = store.code_by_name(&scope, name, role).await?;
        }
        let suggestions = if rows.is_empty() {
            store.code_suggestions(&scope, name, role).await?
        } else {
            Vec::new()
        };
        self.answer(&scope, rows, suggestions).await
    }

    /// The files of the first 50 hits whose size or modified time
    /// changed, or that are gone, go to their project's worker as one
    /// `Recheck`, awaited for at most 2 s in all. `true` for nonempty hits, so
    /// the caller asks again, once. A project with no worker answers from its
    /// rows; its status says `indexing` or `inactive`.
    async fn recheck(&self, scope: &[ScopeRow], hits: &[Hit]) -> bool {
        let mut jobs = Vec::new();
        for (id, slug, dir) in scope {
            let Some(dir) = dir else {
                continue;
            };
            let mut paths: Vec<&str> = hits
                .iter()
                .take(HITS)
                .filter(|h| &h.project == slug)
                .map(|h| h.path.as_str())
                .collect();
            paths.sort_unstable();
            paths.dedup();
            if paths.is_empty() {
                continue;
            }
            let changed = self.stale(id, Path::new(dir), paths).await;
            if changed.is_empty() {
                continue;
            }
            let worker = self.inner.active.lock().await.worker(id).map(|w| w.jobs());
            if let Some(worker) = worker {
                jobs.push((worker, changed));
            }
        }
        if jobs.is_empty() {
            // A worker may have committed between the hit query and the stamp
            // query. A current stamp does not make the earlier hits current.
            return !hits.is_empty();
        }
        let wait = async {
            for (worker, changed) in jobs {
                let (reply, done) = tokio::sync::oneshot::channel();
                if worker.send(Job::Recheck(changed, reply)).await.is_ok() {
                    let _ = done.await;
                }
            }
        };
        if tokio::time::timeout(RECHECK_WAIT, wait).await.is_err() {
            tracing::warn!("code.recheck_timeout");
        }
        true
    }

    /// At most 50 hits, `more` from the 51st, and each project's status.
    async fn answer(
        &self,
        scope: &[ScopeRow],
        mut hits: Vec<Hit>,
        suggestions: Vec<String>,
    ) -> Result<Answer, CodeError> {
        let more = hits.len() > HITS;
        hits.truncate(HITS);
        let mut status = Vec::with_capacity(scope.len());
        for project in scope {
            status.push(self.status_of(project).await?);
        }
        Ok(Answer {
            hits,
            more,
            suggestions,
            status,
        })
    }

    /// `NoDirectory` for a project with no folder; an active project's
    /// worker's state and last scan; `Inactive` for any other.
    async fn status_of(
        &self,
        (id, slug, directory): &ScopeRow,
    ) -> Result<ProjectStatus, CodeError> {
        let (files, skipped) = self.inner.store.code_counts(id).await?;
        let (state, updated_at) = match directory {
            None => (IndexState::NoDirectory, None),
            Some(_) => match self.inner.active.lock().await.worker(id) {
                Some(worker) => worker.state(),
                None => (IndexState::Inactive, None),
            },
        };
        Ok(ProjectStatus {
            project: slug.clone(),
            state,
            files,
            skipped,
            updated_at,
        })
    }
}

#[cfg(test)]
#[path = "../tests/recheck/invariants.rs"]
mod tests;
