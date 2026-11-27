//! One job: a project's code on disk — the `Codebase` service. It walks
//! a project's folder into SQLite, keeps it current while the project is
//! active, runs the language servers its files want, and answers its
//! questions through them.
//!
//! `model` holds the types, `store` the queries, `scan` the walk and the
//! indexing of one file, `scope` who may read what, `watch` one active
//! project's worker, `active` which projects have one, `links` the links as
//! commands, `inventory` the paged file list, `inputs` the real-path check
//! of a project path, `lsp` the language servers.

mod active;
mod inputs;
mod inventory;
mod links;
mod lsp;
mod model;
mod scan;
mod scope;
mod store;
mod watch;

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

pub use active::ActiveHold;
pub use model::{
    AddedServer, CodebaseConfig, IndexState, InventoryEntry, InventoryKind, InventoryPage,
    InventoryQuery, InventoryState, LspAnswer, LspOperation, LspQuestion, ProjectLink,
    ProjectStatus, ServerState, ServerStatus, Skipped,
};
pub use scope::Asker;

mod error;
pub use error::CodebaseError;
#[cfg(feature = "test-support")]
mod testing;
use active::{Active, Uses};
use core_db::Storage;
use core_projects::{ProjectId, Projects};
use store::{CodeStore, ScopeRow};

struct Inner {
    store: CodeStore,
    projects: Projects,
    active: tokio::sync::Mutex<Active>,
    /// When each project was last used, and its holds.
    uses: std::sync::Mutex<Uses>,
    /// How long a project goes unused before its worker stops.
    idle: std::sync::Mutex<Duration>,
    /// Per project, the lock that keeps its listings one at a time.
    listings: std::sync::Mutex<std::collections::HashMap<String, Arc<tokio::sync::Mutex<()>>>>,
    /// Per project, its last listing's file extensions, for the servers it wants.
    extensions:
        std::sync::Mutex<std::collections::HashMap<String, std::collections::HashSet<String>>>,
}

/// A project's code on disk. Cheap to clone: `ProjectLifecycle`, `Turns` and `Runs`
/// hold a clone, and each worker holds one.
#[derive(Clone)]
pub struct Codebase {
    inner: Arc<Inner>,
}

impl Codebase {
    pub fn new(storage: Arc<Storage>, projects: Projects) -> Self {
        Self {
            inner: Arc::new(Inner {
                store: CodeStore::with_storage(storage),
                projects,
                active: Default::default(),
                uses: Default::default(),
                idle: std::sync::Mutex::new(active::IDLE),
                listings: Default::default(),
                extensions: Default::default(),
            }),
        }
    }

    /// Starts the workers of the projects used before it, and the sweep that
    /// stops idle ones; it waits for no scan. `AppCore::start` calls it with the
    /// default; a test calls it when it wants workers.
    pub async fn start(&self, config: CodebaseConfig) -> Result<(), CodebaseError> {
        let early = self.inner.active.lock().await.begin(config);
        for project in early {
            self.activate(&project).await;
        }
        self.spawn_sweep();
        Ok(())
    }

    /// The project was used: the idle time starts again, and its worker starts
    /// if none runs; the scan runs in the worker. Its links are not activated.
    /// It never fails its caller: a failure is logged. Before `start`, it only
    /// records the use.
    pub async fn touch(&self, project: &ProjectId) {
        if let Ok(mut uses) = self.inner.uses.lock() {
            uses.used(project);
        }
        self.activate(project).await;
    }

    /// Gives `project` a worker if it has a folder and none runs.
    async fn activate(&self, project: &ProjectId) {
        let mut active = self.inner.active.lock().await;
        if !active.running() {
            active.record(project);
            return;
        }
        if active.worker(project).is_some() {
            return;
        }
        if let Some(dir) = self.folder(project).await {
            active.activate(self, project, dir);
        }
    }

    /// The project was removed: its watcher and worker stop. It never fails
    /// its caller; the rows went in the removal's own write.
    pub async fn forget(&self, project: &ProjectId) {
        if let Ok(mut uses) = self.inner.uses.lock() {
            uses.forget(project);
        }
        self.inner.active.lock().await.forget(project);
    }

    /// Stops every watcher and worker; a worker finishes the file it is on.
    pub async fn shut_down(&self) {
        let tasks = self.inner.active.lock().await.close();
        for task in tasks {
            let _ = task.await;
        }
    }

    /// Test only: `project`'s worker drops every path its watcher reports from now on.
    #[cfg(feature = "test-support")]
    pub(crate) async fn pause_watcher(&self, project: &ProjectId) {
        if let Some(worker) = self.inner.active.lock().await.worker(project) {
            worker.pause();
        }
    }

    /// The active projects.
    #[cfg(feature = "test-support")]
    pub(crate) async fn active_projects(&self) -> Vec<ProjectId> {
        self.inner.active.lock().await.active()
    }

    /// The project's folder, or `None`: no folder, or an unknown project (logged
    /// unless unknown).
    async fn folder(&self, project: &ProjectId) -> Option<PathBuf> {
        match self.inner.store.code_scope(project).await {
            Ok(rows) => rows
                .into_iter()
                .find(|(id, _, _)| id == project)
                .and_then(|(_, _, dir)| dir.map(PathBuf::from)),
            Err(core_db::StorageError::NotFound(_)) => None,
            Err(error) => {
                tracing::warn!(project = project.as_str(), error = %error, "code.touch_failed");
                None
            }
        }
    }

    /// How the project's index stands. It does not touch the project.
    pub async fn status(&self, project: &ProjectId) -> Result<ProjectStatus, CodebaseError> {
        let p = self.inner.projects.get(project).await?;
        self.status_of(&(p.id, p.slug, p.directory)).await
    }

    /// Scans `project` once, now, on the caller's task: tests, and a refresh
    /// with no worker. A worker calls `scan_project` itself.
    pub(crate) async fn scan(&self, project: &ProjectId) -> Result<(), CodebaseError> {
        self.scan_project(project, &scan::Progress::default())
            .await
            .map(|_| ())
    }

    /// `NoDirectory` for a project with no folder; an active project's
    /// worker's state and last scan; `Inactive` for any other.
    async fn status_of(
        &self,
        (id, slug, directory): &ScopeRow,
    ) -> Result<ProjectStatus, CodebaseError> {
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
