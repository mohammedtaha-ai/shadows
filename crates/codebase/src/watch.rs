//! One job: one active project's worker, with its watcher and its periodic
//! scan.
//!
//! The worker is one `tokio` task. It does the project's scans, the files its
//! watcher reports and the files a question re-checks, one after another, so
//! two writes of one file never race. The watcher is the fast path only: it
//! can lose changes without saying so, so a periodic scan runs
//! beside it, and a question checks its own hits.
//!
//! The watcher's handler runs on `notify`'s own thread. It never awaits: it
//! drops reads, and what is under `target/` or `node_modules/`, and
//! `try_send`s the rest; a full channel sets `lost`, which the worker answers
//! with a scan. A read is dropped because Linux reports one: `notify`'s
//! inotify backend watches `OPEN` and `CLOSE_NOWRITE`, so without the filter
//! a scan's own reads would trigger more scans without an end.
//! A `Close(Write)` is kept: it ends a write, and
//! inotify reports a write through `mmap(2)` by nothing else.
//!
//! The worker also owns the project's language servers: after each scan and each
//! watcher batch it starts the servers the files now want and stops the rest, and
//! tells every running server which of its files changed on disk, since a server
//! reads a closed file again only when told.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use notify::event::{AccessKind, AccessMode};
use notify::{EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use tokio::sync::{Notify, mpsc, oneshot};
use tokio::task::JoinHandle;
use tokio::time::{Instant, MissedTickBehavior};

use super::Codebase;
use super::lsp::Wanted;
use super::model::{CodebaseConfig, IndexState, ServerState};
use super::scan::Progress;
use core_projects::ProjectId;

/// How long a language server may take to start and initialize.
const START_WAIT: std::time::Duration = std::time::Duration::from_secs(120);

/// A server's start, as its task reports it back to the worker.
type Started = (Wanted, Result<shadows_lsp::Server, shadows_lsp::LspError>);

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
    /// A scan, which lists the folder again, then reply.
    Refresh(oneshot::Sender<()>),
    /// A language-server question: the changes gathered before it reach the servers
    /// first, then the reply carries the running server, if any.
    Lsp(
        String,
        oneshot::Sender<Option<(Wanted, Arc<shadows_lsp::Server>)>>,
    ),
}

/// What the watcher's thread passes on.
enum Seen {
    Path(String),
    Rescan,
}

/// What a worker shares with the rest of `Codebase`: how it stands.
#[derive(Default)]
struct Shared {
    progress: Progress,
    scanning: AtomicBool,
    scanned: AtomicBool,
    missing: AtomicBool,
    /// Test only: every path the watcher reports is dropped, so neither the index
    /// nor a server hears of it (`pause_watcher_for_test`).
    paused: AtomicBool,
    updated_at: Mutex<Option<String>>,
    stopping: Notify,
    /// Each wanted server's state: `Starting`, `Running` or `Failed`.
    servers: Mutex<HashMap<String, ServerState>>,
}

/// One active project's worker, as `Codebase` holds it.
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
        code: Codebase,
        project: ProjectId,
        dir: PathBuf,
        config: CodebaseConfig,
        before: Option<JoinHandle<()>>,
    ) -> Self {
        let (jobs, jobs_rx) = mpsc::channel(64);
        let (seen, seen_rx) = mpsc::channel(SEEN_CAPACITY);
        let (started, started_rx) = mpsc::unbounded_channel();
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
            servers: HashMap::new(),
            extra: HashSet::new(),
            files: HashMap::new(),
            started,
        };
        let task = tokio::spawn(async move {
            if let Some(before) = before {
                let _ = before.await;
            }
            run.run(jobs_rx, seen_rx, started_rx, config).await;
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

    /// How one of the project's wanted servers stands, if the worker holds it.
    pub(super) fn server_state(&self, id: &str) -> Option<ServerState> {
        self.shared.servers.lock().ok()?.get(id).cloned()
    }

    /// Where to send a job.
    pub(super) fn jobs(&self) -> mpsc::Sender<Job> {
        self.jobs.clone()
    }

    /// Test only: drop every path the watcher reports from now on.
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
    code: Codebase,
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
    /// The running language servers, by id.
    servers: HashMap<String, (Wanted, Arc<shadows_lsp::Server>)>,
    /// Extensions of files the watcher saw since the last listing.
    extra: HashSet<String>,
    /// Each file the last scan listed, with its size and modified time: what the
    /// next scan compares against to tell the servers what changed.
    files: HashMap<String, (i64, i64)>,
    /// Where a server's start task reports.
    started: mpsc::UnboundedSender<Started>,
}

impl Run {
    async fn run(
        mut self,
        mut jobs: mpsc::Receiver<Job>,
        mut seen: mpsc::Receiver<Seen>,
        mut started: mpsc::UnboundedReceiver<Started>,
        config: CodebaseConfig,
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
                        if !self.shared.paused.load(Ordering::Relaxed) {
                            self.pending.insert(path);
                            last = Instant::now();
                        }
                    }
                    Some(Seen::Rescan) | None => self.handle(Job::Scan).await,
                },
                Some((wanted, result)) = started.recv() => self.server_started(wanted, result),
                _ = quiet, if !self.pending.is_empty() => {
                    // A folder renamed or deleted is reported alone, never
                    // the files in it: the whole batch becomes one scan,
                    // which takes every pending path with it.
                    if self.folder_changed().await {
                        tracing::info!(project = self.project.as_str(), "code.folder_changed");
                        self.handle(Job::Scan).await;
                    } else {
                        let all: Vec<String> = self.pending.drain().collect();
                        self.tell_servers(&all).await;
                        let paths: Vec<String> = all
                            .into_iter()
                            .filter(|p| Codebase::keeps(Path::new(p)))
                            .collect();
                        if !paths.is_empty() {
                            self.handle(Job::Files(paths)).await;
                        }
                        self.sync_servers(false).await;
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
        self.stop_servers().await;
    }

    async fn handle(&mut self, job: Job) {
        match job {
            Job::Scan => self.scan().await,
            Job::Files(paths) => self.files(paths).await,
            Job::Recheck(paths, reply) => {
                self.recheck(paths).await;
                let _ = reply.send(());
            }
            Job::Refresh(reply) => {
                self.scan().await;
                let _ = reply.send(());
            }
            Job::Lsp(server, reply) => {
                let gathered: Vec<String> = self.pending.iter().cloned().collect();
                self.tell_servers(&gathered).await;
                let _ = reply.send(self.servers.get(&server).cloned());
            }
        }
    }

    /// A scan, after checking the folder is there (a watched folder can be
    /// renamed without a word). A missing folder drops the watcher; a folder
    /// found again gets a new one, since the old one stays dead.
    /// A scan that finds changes nobody reported restarts the watcher: it may
    /// have stopped without telling.
    async fn scan(&mut self) {
        // The scan covers every path still gathered; the servers hear of them with
        // what the scan finds, as a watcher batch's would.
        let unreported = self.pending.is_empty();
        let gathered: Vec<String> = self.pending.drain().collect();
        if !Codebase::folder_exists(&self.dir).await {
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
            *at = Some(core_db::Storage::now());
        }
        let scanned = match result {
            Ok(scanned) => scanned,
            Err(error) => {
                tracing::warn!(project = self.project.as_str(), error = %error, "code.scan_failed");
                Default::default()
            }
        };
        if scanned.indexed > 0 && watched && unreported {
            tracing::info!(
                project = self.project.as_str(),
                indexed = scanned.indexed,
                "code.watcher_restarted"
            );
            // The new one watches before the old one is dropped, so no
            // change falls between them; a failure keeps the old one.
            if let Some(new) = self.watch() {
                self.watcher = Some(new);
            }
        }
        // What the listing finds changed since the last scan reaches the running
        // servers before any later job, so a `Job::Lsp` queued after it sees it.
        let mut told: HashSet<String> = gathered.into_iter().collect();
        if let Some(files) = scanned.files {
            told.extend(files.follow(&mut self.files));
        }
        if !self.servers.is_empty() {
            let told: Vec<String> = told.into_iter().collect();
            self.tell_servers(&told).await;
        }
        // The listing is current again; a failed server is tried once more.
        self.extra.clear();
        self.sync_servers(true).await;
    }

    /// Starts the servers the project's files want and are not running, and stops the
    /// running ones they do not want now. A failed server is tried again only on
    /// `retry` (after a scan); one whose command is not found is not started.
    async fn sync_servers(&mut self, retry: bool) {
        let wanted = match self.code.wanted_servers(&self.project, &self.extra).await {
            Ok(wanted) => wanted,
            Err(error) => {
                tracing::warn!(
                    project = self.project.as_str(),
                    error = %error,
                    "code.servers_failed"
                );
                return;
            }
        };
        let ids: HashSet<&str> = wanted.iter().map(|w| w.id.as_str()).collect();
        let mut states = match self.shared.servers.lock() {
            Ok(mut shared) => std::mem::take(&mut *shared),
            Err(_) => HashMap::new(),
        };
        states.retain(|id, _| ids.contains(id.as_str()));
        let gone: Vec<String> = self
            .servers
            .keys()
            .filter(|id| !ids.contains(id.as_str()))
            .cloned()
            .collect();
        for id in gone {
            if let Some((_, server)) = self.servers.remove(&id) {
                tracing::info!(project = self.project.as_str(), server = %id, "code.server_stop");
                tokio::spawn(Run::shut(server));
            }
        }
        let path = std::env::var_os("PATH").unwrap_or_default();
        for server in wanted {
            if self.servers.contains_key(&server.id) {
                continue;
            }
            match states.get(&server.id) {
                Some(ServerState::Starting) => continue,
                Some(ServerState::Failed { .. }) if !retry => continue,
                _ => {}
            }
            let Some(executable) = Codebase::find_command(&server.command, &path) else {
                states.remove(&server.id);
                continue;
            };
            states.insert(server.id.clone(), ServerState::Starting);
            tracing::info!(
                project = self.project.as_str(),
                server = %server.id,
                "code.server_start"
            );
            let spec = shadows_lsp::ServerSpec {
                executable,
                args: server.args.clone(),
                root: self.dir.clone(),
                env: std::env::vars().collect(),
            };
            let started = self.started.clone();
            tokio::spawn(async move {
                let result = shadows_lsp::Server::start(spec, START_WAIT).await;
                // A worker that stopped meanwhile drops the server, which kills it.
                let _ = started.send((server, result));
            });
        }
        if let Ok(mut shared) = self.shared.servers.lock() {
            *shared = states;
        }
    }

    /// A start task's report: the server runs, or reads `Failed` with its last line.
    fn server_started(
        &mut self,
        server: Wanted,
        result: Result<shadows_lsp::Server, shadows_lsp::LspError>,
    ) {
        let state = match result {
            Ok(running) => {
                let id = server.id.clone();
                self.servers.insert(id, (server.clone(), Arc::new(running)));
                ServerState::Running
            }
            Err(error) => {
                let error = match error {
                    shadows_lsp::LspError::Exited(Some(line)) => line,
                    other => other.to_string(),
                };
                tracing::warn!(
                    project = self.project.as_str(),
                    server = %server.id,
                    error = %error,
                    "code.server_failed"
                );
                ServerState::Failed { error: Some(error) }
            }
        };
        if let Ok(mut states) = self.shared.servers.lock() {
            states.insert(server.id, state);
        }
    }

    /// The watcher's paths go to every running server that reads them, as changed
    /// or deleted by what is on disk now; an extension not listed yet is kept so the
    /// next `sync_servers` sees it.
    async fn tell_servers(&mut self, paths: &[String]) {
        let mut on_disk = Vec::with_capacity(paths.len());
        for path in paths {
            let full = self.dir.join(path);
            let exists = tokio::fs::symlink_metadata(&full)
                .await
                .is_ok_and(|m| m.is_file());
            if exists && let Some(ext) = Codebase::extension(path) {
                self.extra.insert(ext.to_string());
            }
            on_disk.push((path, full, exists));
        }
        for (server, running) in self.servers.values() {
            let changes: Vec<_> = on_disk
                .iter()
                .filter(|(path, _, _)| server.reads(path))
                .map(|(_, full, exists)| {
                    let change = if *exists {
                        shadows_lsp::FileChange::Changed
                    } else {
                        shadows_lsp::FileChange::Deleted
                    };
                    (full.clone(), change)
                })
                .collect();
            if !changes.is_empty()
                && let Err(error) = running.files_changed(&changes).await
            {
                tracing::warn!(
                    project = self.project.as_str(),
                    server = %server.id,
                    error = %error,
                    "code.server_tell_failed"
                );
            }
        }
    }

    /// Shuts every running server down: the worker is stopping.
    async fn stop_servers(&mut self) {
        for (_, (_, server)) in self.servers.drain() {
            Run::shut(server).await;
        }
        if let Ok(mut states) = self.shared.servers.lock() {
            states.clear();
        }
    }

    /// Shuts one server down once no question holds it; a held one is killed when
    /// the last holder lets go.
    async fn shut(server: Arc<shadows_lsp::Server>) {
        if let Ok(server) = Arc::try_unwrap(server) {
            server.shutdown().await;
        }
    }

    /// The watcher's batch, decided by what is on disk now, never by the
    /// events' kinds: a save through rename arrives as `Remove` then `Create`.
    /// A regular file the walk keeps is indexed; one that is gone, or is no
    /// longer a regular file, has its rows deleted by `index_file`; one an
    /// ignore file names is left alone.
    async fn files(&mut self, paths: Vec<String>) {
        if !Codebase::folder_exists(&self.dir).await {
            self.folder_missing();
            return;
        }
        let kept = {
            let (dir, paths) = (self.dir.clone(), paths.clone());
            tokio::task::spawn_blocking(move || Codebase::kept_now(&dir, &paths))
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
            if regular && !kept.contains(&Codebase::path_key(&path)) {
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
            if Codebase::keeps(Path::new(path)) || path.split('/').any(|c| c.starts_with('.')) {
                continue;
            }
            match tokio::fs::symlink_metadata(self.dir.join(path)).await {
                Ok(meta) if meta.is_dir() => {
                    let store = &self.code.inner.store;
                    // A failed read counts as new: a scan is the safe answer.
                    if !store
                        .code_files_under(&self.project, &Codebase::path_key(path))
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
    /// it is on.
    fn stopping(&self) -> bool {
        self.shared.progress.stop.load(Ordering::Relaxed)
    }

    async fn index(&self, path: &str) {
        if let Err(error) = self.code.index_file(&self.project, &self.dir, path).await {
            tracing::warn!(
                project = self.project.as_str(),
                path = %path,
                error = %error,
                "code.index_failed"
            );
        }
    }

    fn folder_missing(&mut self) {
        if !self.shared.missing.swap(true, Ordering::Relaxed) {
            tracing::warn!(
                project = self.project.as_str(),
                dir = %self.dir.display(),
                "code.directory_missing"
            );
        }
        self.watcher = None;
    }

    /// A new recursive watcher on the folder, or `None` (logged): the next
    /// periodic scan tries again.
    fn watch(&self) -> Option<RecommendedWatcher> {
        let (dir, seen, lost) = (self.dir.clone(), self.seen.clone(), self.lost.clone());
        let (code, project) = (self.code.clone(), self.project.clone());
        let saw = move |rel: &str| code.saw(&project, rel);
        let made =
            notify::recommended_watcher(move |event| forward(&dir, &seen, &lost, &saw, event))
                .and_then(|mut w| w.watch(&self.dir, RecursiveMode::Recursive).map(|()| w));
        match made {
            Ok(w) => Some(w),
            Err(error) => {
                tracing::warn!(
                    project = self.project.as_str(),
                    error = %error,
                    "code.watcher_failed"
                );
                None
            }
        }
    }
}

/// On `notify`'s thread: little, and never an await. A read (any `Access`
/// but `Close(Write)`) is dropped: the scan's own reads would come back as
/// changes. A path under `target/` or `node_modules/` is dropped next (a
/// build sends hundreds); a changed
/// `.gitignore`, an error or `need_rescan` asks for a scan; the rest go on,
/// relative to the folder. Every path kept is also told to the project's holds
/// (`saw`), which keep what changed while they live.
fn forward(
    dir: &Path,
    seen: &mpsc::Sender<Seen>,
    lost: &AtomicBool,
    saw: &impl Fn(&str),
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
    if let EventKind::Access(kind) = event.kind
        && kind != AccessKind::Close(AccessMode::Write)
    {
        return;
    }
    for path in &event.paths {
        let Some(rel) = Codebase::relative(dir, path) else {
            continue;
        };
        if rel.split('/').any(Codebase::never_walked) {
            continue;
        }
        if !rel.is_empty() {
            saw(&rel);
        }
        if rel.rsplit('/').next() == Some(".gitignore") {
            send(Seen::Rescan);
        } else if !rel.is_empty() {
            send(Seen::Path(rel));
        }
    }
}

#[cfg(test)]
#[path = "../tests/watch/invariants.rs"]
mod tests;
