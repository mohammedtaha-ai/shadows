//! One job: which projects are active.
//!
//! A project is active while it is used: a sent message, a code question or a
//! running piece of work (an `ActiveHold`) makes it so, and a sweep stops its
//! worker once it has gone unused for the idle time. Each active project has
//! exactly one worker. Nothing is stored: after a restart a project waits for
//! its next use.

use std::collections::{BTreeSet, HashMap};
use std::path::PathBuf;
use std::sync::{Arc, Mutex, Weak};
use std::time::{Duration, Instant};

use tokio::task::JoinHandle;

use super::Codebase;
use super::model::CodebaseConfig;
use super::watch::Worker;
use core_projects::ProjectId;

/// How long a project goes unused before its worker stops.
pub(super) const IDLE: Duration = Duration::from_secs(10 * 60);
/// The longest the sweep sleeps between two looks.
const SWEEP_EVERY: Duration = Duration::from_secs(30);

/// When a project was last used, and how many holds keep it active now.
#[derive(Debug, Clone, Copy)]
pub(super) struct Usage {
    last: Instant,
    holds: usize,
}

/// The use of every project, kept apart from `Active` so a hold's drop, which
/// cannot await, records its end at once.
#[derive(Default)]
pub(super) struct Uses {
    by_project: HashMap<ProjectId, Usage>,
    /// What each living hold of a project has seen change, by project.
    seen: HashMap<ProjectId, Vec<Weak<Mutex<BTreeSet<String>>>>>,
}

impl Uses {
    /// A use now: the idle time starts again.
    pub(super) fn used(&mut self, project: &ProjectId) {
        let usage = self.by_project.entry(project.clone()).or_insert(Usage {
            last: Instant::now(),
            holds: 0,
        });
        usage.last = Instant::now();
    }

    pub(super) fn hold(&mut self, project: &ProjectId) {
        self.used(project);
        if let Some(usage) = self.by_project.get_mut(project) {
            usage.holds += 1;
        }
    }

    /// A hold ended: it counts as a use, so the idle time starts now.
    pub(super) fn release(&mut self, project: &ProjectId) {
        if let Some(usage) = self.by_project.get_mut(project) {
            usage.holds = usage.holds.saturating_sub(1);
            usage.last = Instant::now();
        }
    }

    /// Whether the project went unused, with no hold, for `idle`.
    pub(super) fn idle(&self, project: &ProjectId, idle: Duration) -> bool {
        self.by_project
            .get(project)
            .is_none_or(|u| u.holds == 0 && u.last.elapsed() >= idle)
    }

    pub(super) fn forget(&mut self, project: &ProjectId) {
        self.by_project.remove(project);
        self.seen.remove(project);
    }

    /// The watcher saw `path` change: every living hold of the project keeps
    /// it; a path under `.git/` is git's own bookkeeping, not a project file.
    pub(super) fn saw(&mut self, project: &ProjectId, path: &str) {
        if path == ".git" || path.starts_with(".git/") {
            return;
        }
        if let Some(holds) = self.seen.get_mut(project) {
            holds.retain(|hold| match hold.upgrade() {
                Some(seen) => {
                    if let Ok(mut seen) = seen.lock() {
                        seen.insert(path.to_string());
                    }
                    true
                }
                None => false,
            });
        }
    }

    fn watch_for(&mut self, project: &ProjectId) -> Arc<Mutex<BTreeSet<String>>> {
        let seen = Arc::new(Mutex::new(BTreeSet::new()));
        self.seen
            .entry(project.clone())
            .or_default()
            .push(Arc::downgrade(&seen));
        seen
    }
}

/// Keeps a project active while it lives: a running turn or attempt holds one.
/// Its drop counts as a use, so the idle time starts when the work ends. It
/// also keeps the paths the project's watcher saw change while it lived.
pub struct ActiveHold {
    code: Codebase,
    project: ProjectId,
    seen: Arc<Mutex<BTreeSet<String>>>,
}

impl ActiveHold {
    /// The project-relative paths the watcher saw change while this hold
    /// lived, sorted, `.git/` left out. A change made before the project's
    /// watcher started, or that the watcher lost, is not among them.
    pub fn changed(&self) -> Vec<String> {
        self.seen
            .lock()
            .map(|seen| seen.iter().cloned().collect())
            .unwrap_or_default()
    }
}

impl Drop for ActiveHold {
    fn drop(&mut self) {
        if let Ok(mut uses) = self.code.inner.uses.lock() {
            uses.release(&self.project);
        }
    }
}

impl std::fmt::Debug for ActiveHold {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ActiveHold")
            .field("project", &self.project)
            .finish()
    }
}

impl Codebase {
    /// Keeps `project` active until the answer drops; activates it now.
    pub async fn hold(&self, project: &ProjectId) -> ActiveHold {
        let seen = match self.inner.uses.lock() {
            Ok(mut uses) => {
                uses.hold(project);
                uses.watch_for(project)
            }
            Err(_) => Arc::default(),
        };
        self.activate(project).await;
        ActiveHold {
            code: self.clone(),
            project: project.clone(),
            seen,
        }
    }

    /// The watcher of `project` saw `path` change; called on `notify`'s thread.
    pub(super) fn saw(&self, project: &ProjectId, path: &str) {
        if let Ok(mut uses) = self.inner.uses.lock() {
            uses.saw(project, path);
        }
    }

    /// Stops the worker of every active project that went unused for the idle
    /// time with no hold. Its rows stay.
    pub(super) async fn sweep(&self) {
        let idle = self.idle_time();
        let mut active = self.inner.active.lock().await;
        let unused: Vec<ProjectId> = match self.inner.uses.lock() {
            Ok(uses) => active
                .active()
                .into_iter()
                .filter(|p| uses.idle(p, idle))
                .collect(),
            Err(_) => Vec::new(),
        };
        for project in unused {
            active.stop(&project);
        }
    }

    /// The idle time, which a test shortens.
    pub(super) fn idle_time(&self) -> Duration {
        self.inner.idle.lock().map(|d| *d).unwrap_or(IDLE)
    }

    /// The sweep's task: it holds the service weakly, so it ends with it, and
    /// looks every `SWEEP_EVERY` or every idle time, whichever is shorter.
    pub(super) fn spawn_sweep(&self) {
        let weak = std::sync::Arc::downgrade(&self.inner);
        tokio::spawn(async move {
            loop {
                let every = match weak.upgrade() {
                    Some(inner) => {
                        let code = Codebase { inner };
                        if !code.inner.active.lock().await.running() {
                            return;
                        }
                        code.idle_time().min(SWEEP_EVERY)
                    }
                    None => return,
                };
                tokio::time::sleep(every).await;
                match weak.upgrade() {
                    Some(inner) => Codebase { inner }.sweep().await,
                    None => return,
                }
            }
        });
    }
}

/// The active projects' workers.
#[derive(Default)]
pub(super) struct Active {
    /// Uses before `start`, oldest first.
    early: Vec<ProjectId>,
    running: Option<Running>,
    /// `shut_down` ran: nothing starts again.
    closed: bool,
}

struct Running {
    config: CodebaseConfig,
    workers: HashMap<ProjectId, Worker>,
    /// The tasks of stopped workers, finishing the file they are on, by
    /// project: a project active again waits for its own before its new
    /// worker starts, so one project never has two writers, and `close`
    /// hands the rest to shutdown, so none outlives it.
    leaving: HashMap<ProjectId, JoinHandle<()>>,
}

impl Active {
    /// Whether `start` has run and `shut_down` has not.
    pub(super) fn running(&self) -> bool {
        self.running.is_some()
    }

    /// Before `start`, a use only records the project, once, at its last use.
    pub(super) fn record(&mut self, project: &ProjectId) {
        if !self.closed {
            self.early.retain(|p| p != project);
            self.early.push(project.clone());
        }
    }

    /// `start`: answers the uses recorded before it, oldest first, for the
    /// caller to activate.
    pub(super) fn begin(&mut self, config: CodebaseConfig) -> Vec<ProjectId> {
        if self.closed || self.running.is_some() {
            return Vec::new();
        }
        self.running = Some(Running {
            config,
            workers: HashMap::new(),
            leaving: HashMap::new(),
        });
        std::mem::take(&mut self.early)
    }

    /// The project gets a worker, which scans at once, unless it has one.
    pub(super) fn activate(&mut self, code: &Codebase, project: &ProjectId, dir: PathBuf) {
        let Some(running) = &mut self.running else {
            return;
        };
        running.leaving.retain(|_, task| !task.is_finished());
        if running.workers.contains_key(project) {
            return;
        }
        tracing::info!(project = project.as_str(), "code.active");
        let before = running.leaving.remove(project);
        let worker = Worker::spawn(code.clone(), project.clone(), dir, running.config, before);
        running.workers.insert(project.clone(), worker);
    }

    /// The project's worker stops after the file it is on; its index stays.
    pub(super) fn stop(&mut self, project: &ProjectId) {
        if let Some(running) = &mut self.running
            && let Some(worker) = running.workers.remove(project)
        {
            tracing::info!(project = project.as_str(), "code.inactive");
            running.leaving.insert(project.clone(), worker.stop());
        }
    }

    /// The project was removed: its worker stops, and an early use is dropped.
    pub(super) fn forget(&mut self, project: &ProjectId) {
        self.early.retain(|p| p != project);
        self.stop(project);
    }

    /// The active project's worker, if it is active.
    pub(super) fn worker(&self, project: &ProjectId) -> Option<&Worker> {
        self.running.as_ref()?.workers.get(project)
    }

    /// The active projects, by id.
    pub(super) fn active(&self) -> Vec<ProjectId> {
        let mut active: Vec<ProjectId> = self
            .running
            .as_ref()
            .map(|r| r.workers.keys().cloned().collect())
            .unwrap_or_default();
        active.sort_by(|a, b| a.as_str().cmp(b.as_str()));
        active
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
