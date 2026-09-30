//! One job: which projects are active, in order of use (spec §15.6).
//!
//! The order is kept in memory, never stored: after a restart it starts
//! again from the last turns. The first `active_limit` projects in it are
//! active, and each active project has exactly one worker; `settle` makes the
//! workers follow the order after every change.

use std::collections::HashMap;
use std::path::PathBuf;

use tokio::task::JoinHandle;

use super::Code;
use super::model::CodeConfig;
use super::watch::Worker;
use crate::projects::ProjectId;

/// The order of use and the workers of the active projects.
#[derive(Default)]
pub(super) struct Active {
    /// Most recent first; only projects with a folder.
    order: Vec<ProjectId>,
    /// Each ordered project's folder, which never changes (§15.4).
    dirs: HashMap<ProjectId, PathBuf>,
    /// Touches before `start`, oldest first.
    early: Vec<ProjectId>,
    running: Option<Running>,
    /// `shut_down` ran: nothing starts again.
    closed: bool,
}

struct Running {
    config: CodeConfig,
    limit: usize,
    workers: HashMap<ProjectId, Worker>,
    /// The tasks of workers past the limit, finishing the file they are on,
    /// by project: a project active again waits for its own before its new
    /// worker starts, so one project never has two writers, and `close`
    /// hands the rest to shutdown, so none outlives it.
    leaving: HashMap<ProjectId, JoinHandle<()>>,
}

impl Active {
    /// Whether `start` has run and `shut_down` has not.
    pub(super) fn running(&self) -> bool {
        self.running.is_some()
    }

    /// Before `start`, a touch only records the use: once per project, at
    /// its last use, so a core whose `start` never ran does not grow it.
    pub(super) fn record(&mut self, project: &ProjectId) {
        if !self.closed {
            self.early.retain(|p| p != project);
            self.early.push(project.clone());
        }
    }

    /// `start`: the order the database gives, most recent first; answers the
    /// touches recorded before it, oldest first, for the caller to apply.
    pub(super) fn begin(
        &mut self,
        order: Vec<(ProjectId, PathBuf)>,
        limit: usize,
        config: CodeConfig,
    ) -> Vec<ProjectId> {
        if self.closed || self.running.is_some() {
            return Vec::new();
        }
        for (id, dir) in order {
            self.order.push(id.clone());
            self.dirs.insert(id, dir);
        }
        self.running = Some(Running {
            config,
            limit,
            workers: HashMap::new(),
            leaving: HashMap::new(),
        });
        std::mem::take(&mut self.early)
    }

    /// A use of the first of `rows`: its direct links (the rest) become the
    /// most recent, in their order, then it does, so it is always first and
    /// the links touched first are the first to leave (§15.6). A project
    /// with no folder takes no place.
    pub(super) fn used(&mut self, rows: &[(ProjectId, Option<PathBuf>)]) {
        for (id, dir) in rows.iter().skip(1).chain(rows.first()) {
            let Some(dir) = dir else {
                continue;
            };
            self.order.retain(|p| p != id);
            self.order.insert(0, id.clone());
            self.dirs.insert(id.clone(), dir.clone());
        }
    }

    /// A new `active_limit`, which the next `settle` follows. Before `start`
    /// there is nothing to change: `start` reads the stored limit.
    pub(super) fn limit(&mut self, limit: usize) {
        if let Some(running) = &mut self.running {
            running.limit = limit;
        }
    }

    /// Every active project gets a worker, which scans at once; every
    /// project past the limit loses its worker, and its index stays.
    pub(super) fn settle(&mut self, code: &Code) {
        let Some(running) = &mut self.running else {
            return;
        };
        running.leaving.retain(|_, task| !task.is_finished());
        let active = &self.order[..self.order.len().min(running.limit)];
        let gone: Vec<ProjectId> = running
            .workers
            .keys()
            .filter(|p| !active.contains(p))
            .cloned()
            .collect();
        for p in gone {
            if let Some(worker) = running.workers.remove(&p) {
                tracing::info!(project = p.as_str(), "code.inactive");
                running.leaving.insert(p, worker.stop());
            }
        }
        for p in active {
            if !running.workers.contains_key(p)
                && let Some(dir) = self.dirs.get(p)
            {
                tracing::info!(project = p.as_str(), "code.active");
                let before = running.leaving.remove(p);
                let worker =
                    Worker::spawn(code.clone(), p.clone(), dir.clone(), running.config, before);
                running.workers.insert(p.clone(), worker);
            }
        }
    }

    /// The project was removed (§15.4): it leaves the order, and its worker
    /// stops after the file it is on, its task kept in `leaving` for
    /// `close`. The caller's `settle` gives its place to the next project.
    pub(super) fn forget(&mut self, project: &ProjectId) {
        self.order.retain(|p| p != project);
        self.dirs.remove(project);
        self.early.retain(|p| p != project);
        if let Some(running) = &mut self.running
            && let Some(worker) = running.workers.remove(project)
        {
            tracing::info!(project = project.as_str(), "code.removed");
            running.leaving.insert(project.clone(), worker.stop());
        }
    }

    /// The active project's worker, if it is active.
    pub(super) fn worker(&self, project: &ProjectId) -> Option<&Worker> {
        self.running.as_ref()?.workers.get(project)
    }

    /// The active projects, most recent first.
    #[cfg(feature = "test-support")]
    pub(super) fn active(&self) -> Vec<ProjectId> {
        let limit = self.running.as_ref().map_or(0, |r| r.limit);
        self.order.iter().take(limit).cloned().collect()
    }

    /// Stops every worker; answers their tasks, and those of the workers
    /// still leaving, to wait for outside the lock.
    pub(super) fn close(&mut self) -> Vec<JoinHandle<()>> {
        self.closed = true;
        self.early.clear();
        let Some(running) = self.running.take() else {
            return Vec::new();
        };
        let mut tasks: Vec<_> = running.leaving.into_values().collect();
        tasks.extend(running.workers.into_values().map(Worker::stop));
        tasks
    }
}
