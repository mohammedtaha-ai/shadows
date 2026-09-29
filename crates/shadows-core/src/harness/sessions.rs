//! One job: the live adapter connection each open thread holds.

use std::{
    collections::HashMap,
    io,
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::Duration,
};

use shadows_process::{self as process, ProcessHandle};
use tokio::{
    sync::{Mutex, broadcast, mpsc},
    time::Instant,
};
use tracing::Instrument;

use super::offers::{Offers, intercept};
use super::setup::Setups;
use crate::{
    db::{Storage, StorageError},
    threads::{ThreadId, TurnContext},
};
use shadows_agent::{
    acp::{Connection, SessionStart},
    choices::Offered,
    claude::ClaudeAdapter,
    events::HarnessEvent,
    policy,
};

#[derive(Debug, Clone)]
pub struct SessionsConfig {
    pub idle_after: Duration,
    pub cancel_wait: Duration,
    /// How long an adapter has to start and open its session (§12.2).
    pub setup_wait: Duration,
    /// How long the on-demand context breakdown waits for `/context` (§12.8).
    pub context_wait: Duration,
    /// Shadows' `/mcp`, as the daemon bound it; `None` in tests that need no MCP.
    pub mcp_url: Option<String>,
}

impl Default for SessionsConfig {
    fn default() -> Self {
        Self {
            idle_after: Duration::from_secs(15 * 60),
            cancel_wait: Duration::from_secs(10),
            setup_wait: Duration::from_secs(20),
            context_wait: Duration::from_secs(5),
            mcp_url: None,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum OpenError {
    #[error(transparent)]
    Storage(#[from] StorageError),
    #[error("harness start failed: {0}")]
    Start(String),
    /// The project's directory cannot be run in. The text is written here,
    /// about the user's own project, so a client may show it (§3.2).
    #[error("{0}")]
    Workspace(String),
}

/// A turn cannot take the thread's session (spec §12.7).
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum LeaseError {
    /// Another user of the session — a turn, or a `/context` read that did
    /// not finish in time — holds it.
    #[error("the thread's session is in use")]
    Busy,
    /// The adapter this session ran on is gone.
    #[error("the harness session closed")]
    Closed,
}

/// An open harness session. What it offers now is `Sessions::offered`: the
/// choices change after opening, so a copy here would go stale.
///
/// A resumed session keeps its id across adapters, so each opening also
/// carries a `generation`: a turn that outlives its adapter must not give its
/// events to, mark, or terminate the adapter that replaced it.
#[derive(Clone)]
pub struct OpenSession {
    pub session_id: String,
    pub how: &'static str,
    connection: Connection,
    generation: u64,
}

impl OpenSession {
    pub fn connection(&self) -> &Connection {
        &self.connection
    }
}

pub(super) struct Live {
    pub(super) handle: ProcessHandle,
    pub(super) opened: OpenSession,
    /// `None` while a turn (or a `/context` read) has taken them.
    pub(super) events: Option<mpsc::UnboundedReceiver<HarnessEvent>>,
    last_used: Instant,
    /// Whether a Planner turn has answered in this adapter: before one has,
    /// `/context` can stall a fresh session for tens of seconds (§12.8).
    pub(super) answered: bool,
}

pub struct Sessions {
    adapter: Arc<ClaudeAdapter>,
    storage: Arc<Storage>,
    pub(super) config: SessionsConfig,
    pub(super) live: Mutex<HashMap<ThreadId, Live>>,
    pub(super) offers: Arc<Offers>,
    generations: AtomicU64,
    setups: Setups,
}

impl Sessions {
    /// Shares the core's `storage`, so the grant events an opening and a
    /// close append raise the committed signal `Events` watches (§13.7).
    pub fn new(
        adapter: Arc<ClaudeAdapter>,
        storage: Arc<Storage>,
        config: SessionsConfig,
    ) -> Arc<Self> {
        let period = (config.idle_after / 4)
            .min(Duration::from_secs(60))
            .max(Duration::from_millis(1));
        let sessions = Arc::new(Self {
            setups: Setups::new(storage.clone(), config.mcp_url.clone()),
            adapter,
            storage,
            config,
            live: Mutex::new(HashMap::new()),
            offers: Arc::new(Offers::new()),
            generations: AtomicU64::new(0),
        });
        let reaper = Arc::downgrade(&sessions);
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(period);
            loop {
                interval.tick().await;
                let Some(sessions) = reaper.upgrade() else {
                    break;
                };
                sessions.reap_idle().await;
            }
        });
        sessions
    }

    pub async fn open(&self, thread: &ThreadId) -> Result<OpenSession, OpenError> {
        self.open_live(thread)
            .instrument(tracing::info_span!("sessions.open", thread_id = %thread))
            .await
    }

    async fn open_live(&self, thread: &ThreadId) -> Result<OpenSession, OpenError> {
        let mut live = self.live.lock().await;
        if let Some(current) = live.get_mut(thread) {
            if !current.handle.has_exited() && !current.opened.connection.is_closed() {
                current.last_used = Instant::now();
                return Ok(current.opened.clone());
            }
            if let Some(dead) = live.get_mut(thread) {
                stop_handle(&mut dead.handle)
                    .await
                    .map_err(|e| OpenError::Start(format!("could not close dead adapter: {e}")))?;
            }
            live.remove(thread);
            self.offers.forget(thread);
            self.setups.forget(thread).await;
        }
        let context = self.storage.turn_context(thread).await?;
        let cwd = workspace(&context).map_err(OpenError::Workspace)?;
        // A fork's first opening forks the source's session (§12.9); once
        // the fork's first turn has recorded its own, it resumes that.
        let (how, start) = match (
            context.harness_session_id.clone(),
            context.fork_session_id.clone(),
        ) {
            (Some(id), _) => ("resume", SessionStart::Resume(id)),
            (None, Some(source)) => ("fork", SessionStart::Fork(source)),
            (None, None) => ("new", SessionStart::New),
        };
        let default_mode = policy::default_mode(&context.harness).ok_or_else(|| {
            OpenError::Start(format!("no mode policy for harness {}", context.harness))
        })?;
        let remembered = self.storage.remembered_settings(&context.harness).await?;
        let setup = (self.setups.for_opening(thread, &context.project_id).await)
            .map_err(OpenError::Start)?;
        let mut handle = match process::spawn(self.adapter.process_spec(&cwd)) {
            Ok(handle) => handle,
            Err(error) => {
                self.setups.forget(thread).await;
                return Err(OpenError::Start(error.to_string()));
            }
        };
        let (tx, rx) = mpsc::unbounded_channel();
        let events = intercept(self.offers.clone(), thread.clone(), tx);
        let setup = tokio::time::timeout(self.config.setup_wait, async {
            let connection = Connection::open(&mut handle, events)
                .await
                .map_err(|e| e.to_string())?;
            let session = connection
                .start_session(&cwd, start, &setup)
                .await
                .map_err(|e| e.to_string())?;
            let opened = OpenSession {
                session_id: session.session_id,
                how,
                connection,
                generation: self.generations.fetch_add(1, Ordering::Relaxed),
            };
            let offered = self.offers.record(thread, &session.options)?;
            self.apply_opening_settings(thread, &opened, offered, default_mode, remembered)
                .await
                .map_err(|e| e.to_string())?;
            Ok::<_, String>(opened)
        })
        .await
        .unwrap_or_else(|_| Err("harness setup timed out".into()));
        let opened = match setup {
            Ok(opened) => opened,
            Err(error) => {
                self.offers.forget(thread);
                self.setups.forget(thread).await;
                if let Err(cleanup) = stop_handle(&mut handle).await {
                    tracing::error!(%cleanup, "sessions.failed_open_cleanup");
                }
                return Err(OpenError::Start(error));
            }
        };
        live.insert(
            thread.clone(),
            Live {
                handle,
                opened: opened.clone(),
                events: Some(rx),
                last_used: Instant::now(),
                answered: false,
            },
        );
        Ok(opened)
    }

    /// What the thread's open session offers now, if it is open.
    pub async fn offered(&self, thread: &ThreadId) -> Option<Offered> {
        self.offers.get(thread)
    }

    /// Every change to any thread's offer, as it happens.
    pub fn watch_options(&self) -> broadcast::Receiver<(ThreadId, Offered)> {
        self.offers.subscribe()
    }

    #[cfg(feature = "test-support")]
    pub async fn take_events(
        &self,
        thread: &ThreadId,
    ) -> Option<mpsc::UnboundedReceiver<HarnessEvent>> {
        let mut live = self.live.lock().await;
        let item = live.get_mut(thread)?;
        item.last_used = Instant::now();
        item.events.take()
    }

    /// Takes the session's events for a turn: whoever holds them has the
    /// session to itself (spec §12.7). A `/context` read gives them back
    /// within `context_wait` plus, when it had to cancel, `cancel_wait`; a
    /// turn waits that long before it is `Busy`.
    pub async fn lease_events(
        &self,
        thread: &ThreadId,
        opened: &OpenSession,
    ) -> Result<mpsc::UnboundedReceiver<HarnessEvent>, LeaseError> {
        let deadline = Instant::now()
            + self.config.context_wait
            + self.config.cancel_wait
            + Duration::from_secs(1);
        loop {
            {
                let mut live = self.live.lock().await;
                let item = live
                    .get_mut(thread)
                    .filter(|item| item.opened.generation == opened.generation)
                    .ok_or(LeaseError::Closed)?;
                if let Some(rx) = item.events.take() {
                    item.last_used = Instant::now();
                    return Ok(rx);
                }
            }
            if Instant::now() >= deadline {
                return Err(LeaseError::Busy);
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }

    /// Gives the events back to the adapter they came from; to nothing when
    /// that adapter has since been closed or replaced.
    pub async fn give_back_events(
        &self,
        thread: &ThreadId,
        opened: &OpenSession,
        rx: mpsc::UnboundedReceiver<HarnessEvent>,
    ) {
        if let Some(item) = same_adapter(&mut *self.live.lock().await, thread, opened) {
            item.events = Some(rx);
            item.last_used = Instant::now();
        }
    }

    /// Records that a Planner turn has answered in `opened`'s adapter.
    pub async fn mark_answered(&self, thread: &ThreadId, opened: &OpenSession) {
        if let Some(item) = same_adapter(&mut *self.live.lock().await, thread, opened) {
            item.answered = true;
        }
    }

    pub async fn touch(&self, thread: &ThreadId) {
        if let Some(item) = self.live.lock().await.get_mut(thread) {
            item.last_used = Instant::now();
        }
    }

    pub async fn terminate(&self, thread: &ThreadId) -> io::Result<()> {
        let mut live = self.live.lock().await;
        if let Some(item) = live.get_mut(thread) {
            stop_handle(&mut item.handle).await?;
        }
        live.remove(thread);
        self.offers.forget(thread);
        self.setups.forget(thread).await;
        Ok(())
    }

    /// Terminates `opened`'s adapter and reaps it. An adapter no longer in
    /// the map was already reaped by whoever removed it, so `Ok` then: a
    /// replacement opened since is another adapter, and is left alone.
    pub async fn terminate_adapter(
        &self,
        thread: &ThreadId,
        opened: &OpenSession,
    ) -> io::Result<()> {
        let mut live = self.live.lock().await;
        if let Some(item) = same_adapter(&mut live, thread, opened) {
            stop_handle(&mut item.handle).await?;
            live.remove(thread);
            self.offers.forget(thread);
            self.setups.forget(thread).await;
        }
        Ok(())
    }

    pub async fn close_all(&self) -> io::Result<()> {
        let mut live = self.live.lock().await;
        let mut first = None;
        let threads: Vec<_> = live.keys().cloned().collect();
        for thread in threads {
            let result = stop_handle(
                &mut live
                    .get_mut(&thread)
                    .expect("thread collected above")
                    .handle,
            )
            .await;
            match result {
                Ok(()) => {
                    live.remove(&thread);
                    self.offers.forget(&thread);
                    self.setups.forget(&thread).await;
                }
                Err(error) if first.is_none() => first = Some(error),
                Err(_) => {}
            }
        }
        match first {
            Some(error) => Err(error),
            None => Ok(()),
        }
    }

    async fn reap_idle(&self) {
        let mut live = self.live.lock().await;
        let expired: Vec<_> = live
            .iter()
            .filter(|(_, item)| {
                item.events.is_some() && item.last_used.elapsed() >= self.config.idle_after
            })
            .map(|(id, _)| id.clone())
            .collect();
        for id in expired {
            if let Some(item) = live.get_mut(&id)
                && let Err(error) = stop_handle(&mut item.handle).await
            {
                tracing::warn!(thread_id = %id, %error, "sessions.idle_close_failed");
                continue;
            }
            live.remove(&id);
            self.offers.forget(&id);
            self.setups.forget(&id).await;
        }
    }

    #[cfg(feature = "test-support")]
    pub async fn live_count(&self) -> usize {
        self.live.lock().await.len()
    }

    /// The process id of the thread's adapter, while one is live.
    #[cfg(feature = "test-support")]
    pub async fn pid(&self, thread: &ThreadId) -> Option<u32> {
        self.live
            .lock()
            .await
            .get(thread)
            .and_then(|item| item.handle.id())
    }

    /// Arms the thread's adapter so its next termination fails — see
    /// `ProcessHandle::force_termination_failure`. Returns whether an adapter
    /// was there to arm, so a test cannot pass by arming nothing.
    #[cfg(feature = "test-support")]
    pub async fn force_termination_failure(&self, thread: &ThreadId) -> bool {
        match self.live.lock().await.get_mut(thread) {
            Some(item) => {
                item.handle.force_termination_failure();
                true
            }
            None => false,
        }
    }

    /// The adapter this daemon runs, whose paths and versions every
    /// invocation records (§12.7).
    pub fn adapter(&self) -> &ClaudeAdapter {
        &self.adapter
    }

    pub fn cancel_wait(&self) -> Duration {
        self.config.cancel_wait
    }

    pub(crate) fn setups(&self) -> &Setups {
        &self.setups
    }
}

async fn stop_handle(handle: &mut ProcessHandle) -> io::Result<()> {
    if !handle.has_exited() {
        handle.terminate_tree()?;
    }
    tokio::time::timeout(Duration::from_secs(5), handle.wait())
        .await
        .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "adapter did not exit in time"))?
        .map(|_| ())
}

/// Resolve the project's directory at opening, so a moved or deleted directory
/// fails instead of running in the daemon's own working directory.
pub(super) fn workspace(context: &TurnContext) -> Result<PathBuf, String> {
    let Some(dir) = &context.project_directory else {
        return Err(
            "the project has no directory: it was created before projects owned one".into(),
        );
    };
    if !dir.is_dir() {
        return Err(format!(
            "the project directory is missing or no longer a directory: {}",
            dir.display()
        ));
    }
    Ok(dir.clone())
}

/// The thread's live adapter, if it is still the one `opened` was made on.
fn same_adapter<'a>(
    live: &'a mut HashMap<ThreadId, Live>,
    thread: &ThreadId,
    opened: &OpenSession,
) -> Option<&'a mut Live> {
    live.get_mut(thread)
        .filter(|item| item.opened.generation == opened.generation)
}
