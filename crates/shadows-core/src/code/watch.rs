//! One job: one active project's worker, with its watcher and its periodic
//! scan (spec §15.4).
//!
//! The worker is one `tokio` task. It does the project's scans, the files its
//! watcher reports and the files a question re-checks, one after another, so
//! two writes of one file never race. The watcher is the fast path only: it
//! can lose changes without saying so (PROBE.md), so a periodic scan runs
//! beside it, and a question checks its own hits.
//!
//! The watcher's handler runs on `notify`'s own thread. It never awaits: it
//! drops what is under `target/` or `node_modules/` and `try_send`s the rest;
//! a full channel sets `lost`, which the worker answers with a scan.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use tokio::sync::{Notify, mpsc, oneshot};
use tokio::task::JoinHandle;
use tokio::time::{Instant, MissedTickBehavior};

use super::Code;
use super::model::{CodeConfig, IndexState};
use super::scan::{Progress, folder_exists, keeps, kept_now, never_walked, path_key, relative};
use crate::projects::ProjectId;

/// Paths the watcher may hold for the worker before it counts as lost.
const SEEN_CAPACITY: usize = 8192;

/// What a worker does, one at a time.
pub(super) enum Job {
    /// Steps 1–6 over the whole folder.
    Scan,
    /// Index these relative paths, or delete their rows, by what is on disk.
    Files(Vec<String>),
    /// Index these relative paths if they changed, delete the rows of those
    /// that are gone, then reply.
    Recheck(Vec<String>, oneshot::Sender<()>),
}

/// What the watcher's thread passes on.
enum Seen {
    Path(String),
    Rescan,
}

/// What a worker shares with the rest of `Code`: how it stands.
#[derive(Default)]
struct Shared {
    progress: Progress,
    scanning: AtomicBool,
    scanned: AtomicBool,
    missing: AtomicBool,
    /// Test only: every `Files` job is dropped (`pause_watcher_for_test`).
    paused: AtomicBool,
    updated_at: Mutex<Option<String>>,
    stopping: Notify,
}

/// One active project's worker, as `Code` holds it.
pub(super) struct Worker {
    jobs: mpsc::Sender<Job>,
    shared: Arc<Shared>,
    task: JoinHandle<()>,
}

impl Worker {
    /// Starts the worker; its first act is a scan, which starts the watcher.
    /// With `before`, the task of the project's previous worker, still on its
    /// last file, it waits for that task first: one writer per project.
    pub(super) fn spawn(
        code: Code,
        project: ProjectId,
        dir: PathBuf,
        config: CodeConfig,
        before: Option<JoinHandle<()>>,
    ) -> Self {
        let (jobs, jobs_rx) = mpsc::channel(64);
        let (seen, seen_rx) = mpsc::channel(SEEN_CAPACITY);
        let shared = Arc::new(Shared::default());
        let run = Run {
            code,
            project,
            dir,
            shared: shared.clone(),
            watcher: None,
            seen,
            lost: Arc::new(AtomicBool::new(false)),
            pending: HashSet::new(),
        };
        let task = tokio::spawn(async move {
            if let Some(before) = before {
                let _ = before.await;
            }
            run.run(jobs_rx, seen_rx, config).await;
        });
        Self { jobs, shared, task }
    }

    /// The project's state and when its last scan ended.
    pub(super) fn state(&self) -> (IndexState, Option<String>) {
        let s = &self.shared;
        let state = if s.missing.load(Ordering::Relaxed) {
            IndexState::DirectoryMissing
        } else if s.scanning.load(Ordering::Relaxed) || !s.scanned.load(Ordering::Relaxed) {
            IndexState::Indexing {
                done: s.progress.done.load(Ordering::Relaxed),
                found: s.progress.found.load(Ordering::Relaxed),
            }
        } else {
            IndexState::Ready
        };
        let updated_at = s.updated_at.lock().map(|u| u.clone()).unwrap_or(None);
        (state, updated_at)
    }

    /// Where to send a job.
    pub(super) fn jobs(&self) -> mpsc::Sender<Job> {
        self.jobs.clone()
    }

    /// Test only: drop every `Files` job from now on.
    #[cfg(feature = "test-support")]
    pub(super) fn pause(&self) {
        self.shared.paused.store(true, Ordering::Relaxed);
    }

    /// Stops the worker after the file it is on, and its watcher with it.
    /// Answers the task, for a caller that waits.
    pub(super) fn stop(self) -> JoinHandle<()> {
        self.shared.progress.stop.store(true, Ordering::Relaxed);
        self.shared.stopping.notify_one();
        self.task
    }
}

/// The worker's own state, owned by its task.
struct Run {
    code: Code,
    project: ProjectId,
    dir: PathBuf,
    shared: Arc<Shared>,
    /// `None` before the first scan, while the folder is missing, and after
    /// the watcher failed to start.
    watcher: Option<RecommendedWatcher>,
    /// Kept so the channel outlives any one watcher.
    seen: mpsc::Sender<Seen>,
    /// The watcher could not pass a path on: only a scan is sure now.
    lost: Arc<AtomicBool>,
    /// Paths reported and not yet handled, until none arrives for `debounce`.
    pending: HashSet<String>,
}

impl Run {
    async fn run(
        mut self,
        mut jobs: mpsc::Receiver<Job>,
        mut seen: mpsc::Receiver<Seen>,
        config: CodeConfig,
    ) {
        let mut interval = tokio::time::interval(config.rescan_every);
        interval.set_missed_tick_behavior(MissedTickBehavior::Delay);
        let mut last = Instant::now();
        loop {
            let quiet = tokio::time::sleep_until(last + config.debounce);
            tokio::select! {
                biased;
                _ = self.shared.stopping.notified() => break,
                job = jobs.recv() => match job {
                    Some(job) => self.handle(job).await,
                    None => break,
                },
                // The first tick is at once: the worker's first scan.
                _ = interval.tick() => self.handle(Job::Scan).await,
                s = seen.recv() => match s {
                    Some(Seen::Path(path)) => {
                        self.pending.insert(path);
                        last = Instant::now();
                    }
                    Some(Seen::Rescan) | None => self.handle(Job::Scan).await,
                },
                _ = quiet, if !self.pending.is_empty() => {
                    // A folder renamed or deleted is reported alone, never
                    // the files in it: the whole batch becomes one scan,
                    // which takes every pending path with it.
                    if self.folder_changed().await {
                        tracing::info!(project = self.project.as_str(), "code.folder_changed");
                        self.handle(Job::Scan).await;
                    } else {
                        let paths: Vec<String> = self
                            .pending
                            .drain()
                            .filter(|p| keeps(Path::new(p)))
                            .collect();
                        if !paths.is_empty() {
                            self.handle(Job::Files(paths)).await;
                        }
                    }
                }
            }
            if self.stopping() {
                break;
            }
            if self.lost.swap(false, Ordering::Relaxed) {
                tracing::warn!(project = self.project.as_str(), "code.watcher_overflow");
                self.handle(Job::Scan).await;
            }
        }
    }

    async fn handle(&mut self, job: Job) {
        match job {
            Job::Scan => self.scan().await,
            Job::Files(paths) => {
                if !self.shared.paused.load(Ordering::Relaxed) {
                    self.files(paths).await;
                }
            }
            Job::Recheck(paths, reply) => {
                self.recheck(paths).await;
                let _ = reply.send(());
            }
        }
    }

    /// A scan, after checking the folder is there (a watched folder can be
    /// renamed without a word). A missing folder drops the watcher; a folder
    /// found again gets a new one, since the old one stays dead (PROBE.md).
    /// A scan that finds changes nobody reported restarts the watcher: it may
    /// have stopped without telling.
    async fn scan(&mut self) {
        // The scan covers every path still gathered.
        let unreported = self.pending.is_empty();
        self.pending.clear();
        if !folder_exists(&self.dir).await {
            self.folder_missing();
            return;
        }
        if self.shared.missing.swap(false, Ordering::Relaxed) {
            tracing::info!(project = self.project.as_str(), "code.directory_back");
            self.watcher = None;
        }
        let watched = self.watcher.is_some();
        if !watched {
            self.watcher = self.watch();
        }
        let s = &self.shared;
        s.scanning.store(true, Ordering::Relaxed);
        let result = self.code.scan_project(&self.project, &s.progress).await;
        s.scanning.store(false, Ordering::Relaxed);
        // A scan cut short by `stop` is no evidence the watcher stopped.
        if self.stopping() {
            return;
        }
        s.scanned.store(true, Ordering::Relaxed);
        if let Ok(mut at) = s.updated_at.lock() {
            *at = Some(crate::db::now());
        }
        match result {
            Ok(indexed) if indexed > 0 && watched && unreported => {
                tracing::info!(
                    project = self.project.as_str(),
                    indexed,
                    "code.watcher_restarted"
                );
                // The new one watches before the old one is dropped, so no
                // change falls between them; a failure keeps the old one.
                if let Some(new) = self.watch() {
                    self.watcher = Some(new);
                }
            }
            Ok(_) => {}
            Err(error) => {
                tracing::warn!(project = self.project.as_str(), error = %error, "code.scan_failed")
            }
        }
    }

    /// The watcher's batch, decided by what is on disk now, never by the
    /// events' kinds: a save through rename arrives as `Remove` then `Create`.
    /// A regular file the walk keeps is indexed; one that is gone, or is no
    /// longer a regular file, has its rows deleted by `index_file`; one an
    /// ignore file names is left alone.
    async fn files(&mut self, paths: Vec<String>) {
        if !folder_exists(&self.dir).await {
            self.folder_missing();
            return;
        }
        let kept = {
            let (dir, paths) = (self.dir.clone(), paths.clone());
            tokio::task::spawn_blocking(move || kept_now(&dir, &paths))
                .await
                .unwrap_or_default()
        };
        for path in paths {
            if self.stopping() {
                return;
            }
            let regular = tokio::fs::symlink_metadata(self.dir.join(&path))
                .await
                .is_ok_and(|m| m.is_file());
            if regular && !kept.contains(&path_key(&path)) {
                continue;
            }
            self.index(&path).await;
        }
    }

    /// Whether a gathered path may be a folder renamed or deleted: one that
    /// `keeps` does not take, is not hidden, and is gone, or is a folder the
    /// index holds nothing under (moved in, its files never reported).
    /// Windows reports a folder on every save inside it; that folder has
    /// rows, and its files' own paths are handled as `Files`. A gone file
    /// the table does not know (a `README.md`) passes too: the scan it
    /// starts reads only what changed.
    async fn folder_changed(&self) -> bool {
        for path in &self.pending {
            if keeps(Path::new(path)) || path.split('/').any(|c| c.starts_with('.')) {
                continue;
            }
            match tokio::fs::symlink_metadata(self.dir.join(path)).await {
                Ok(meta) if meta.is_dir() => {
                    let storage = &self.code.inner.storage;
                    // A failed read counts as new: a scan is the safe answer.
                    if !storage
                        .code_files_under(&self.project, &path_key(path))
                        .await
                        .unwrap_or(false)
                    {
                        return true;
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => return true,
                _ => {}
            }
        }
        false
    }

    /// A question's hits whose files changed: `index_file` for each.
    async fn recheck(&mut self, paths: Vec<String>) {
        for path in paths {
            if self.stopping() {
                return;
            }
            self.index(&path).await;
        }
    }

    /// The worker was told to stop: like a scan, a batch ends after the file
    /// it is on (§15.8).
    fn stopping(&self) -> bool {
        self.shared.progress.stop.load(Ordering::Relaxed)
    }

    async fn index(&self, path: &str) {
        if let Err(error) = self.code.index_file(&self.project, &self.dir, path).await {
            tracing::warn!(project = self.project.as_str(), path = %path, error = %error, "code.index_failed");
        }
    }

    fn folder_missing(&mut self) {
        if !self.shared.missing.swap(true, Ordering::Relaxed) {
            tracing::warn!(project = self.project.as_str(), dir = %self.dir.display(), "code.directory_missing");
        }
        self.watcher = None;
    }

    /// A new recursive watcher on the folder, or `None` (logged): the next
    /// periodic scan tries again.
    fn watch(&self) -> Option<RecommendedWatcher> {
        let (dir, seen, lost) = (self.dir.clone(), self.seen.clone(), self.lost.clone());
        let made = notify::recommended_watcher(move |event| forward(&dir, &seen, &lost, event))
            .and_then(|mut w| w.watch(&self.dir, RecursiveMode::Recursive).map(|()| w));
        match made {
            Ok(w) => Some(w),
            Err(error) => {
                tracing::warn!(project = self.project.as_str(), error = %error, "code.watcher_failed");
                None
            }
        }
    }
}

/// On `notify`'s thread: little, and never an await. A path under `target/`
/// or `node_modules/` is dropped first (a build sends hundreds); a changed
/// `.gitignore`, an error or `need_rescan()` asks for a scan; the rest go on,
/// relative to the folder.
fn forward(
    dir: &Path,
    seen: &mpsc::Sender<Seen>,
    lost: &AtomicBool,
    event: notify::Result<notify::Event>,
) {
    let send = |s: Seen| {
        if seen.try_send(s).is_err() {
            lost.store(true, Ordering::Relaxed);
        }
    };
    let event = match event {
        Ok(event) if !event.need_rescan() => event,
        Ok(_) => {
            tracing::warn!(dir = %dir.display(), "code.watcher_rescan");
            return send(Seen::Rescan);
        }
        Err(error) => {
            tracing::warn!(dir = %dir.display(), error = %error, "code.watcher_error");
            return send(Seen::Rescan);
        }
    };
    for path in &event.paths {
        let Some(rel) = relative(dir, path) else {
            continue;
        };
        if rel.split('/').any(never_walked) {
            continue;
        }
        if rel.rsplit('/').next() == Some(".gitignore") {
            send(Seen::Rescan);
        } else if !rel.is_empty() {
            send(Seen::Path(rel));
        }
    }
}
