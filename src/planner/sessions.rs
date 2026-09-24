//! One job: the live adapter connection each open thread holds.

use std::{collections::HashMap, io, path::PathBuf, sync::Arc, time::Duration};

use serde_json::Value;
use tokio::{
    sync::{Mutex, broadcast, mpsc},
    time::Instant,
};
use tracing::Instrument;

use super::offers::{Offers, intercept};
use crate::{
    agent::{
        acp::{AcpError, Connection, SessionStart},
        choices::Offered,
        claude::ClaudeAdapter,
        events::HarnessEvent,
        policy,
    },
    process::{self, ProcessHandle},
    storage::{Storage, StorageError},
    thread::{ThreadId, TurnContext},
};

#[derive(Debug, Clone, Copy)]
pub struct SessionsConfig {
    pub idle_after: Duration,
    pub cancel_wait: Duration,
    /// How long the on-demand context breakdown waits for `/context` (§12.8).
    pub context_wait: Duration,
}

impl Default for SessionsConfig {
    fn default() -> Self {
        Self {
            idle_after: Duration::from_secs(15 * 60),
            cancel_wait: Duration::from_secs(10),
            context_wait: Duration::from_secs(5),
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

/// An open harness session. What it offers now is `Sessions::offered`: the
/// choices change after opening, so a copy here would go stale.
#[derive(Clone)]
pub struct OpenSession {
    pub session_id: String,
    pub how: &'static str,
    connection: Connection,
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
    storage: Storage,
    pub(super) config: SessionsConfig,
    pub(super) live: Mutex<HashMap<ThreadId, Live>>,
    offers: Arc<Offers>,
}

impl Sessions {
    pub fn new(adapter: Arc<ClaudeAdapter>, storage: Storage, config: SessionsConfig) -> Arc<Self> {
        let sessions = Arc::new(Self {
            adapter,
            storage,
            config,
            live: Mutex::new(HashMap::new()),
            offers: Arc::new(Offers::new()),
        });
        let reaper = Arc::downgrade(&sessions);
        let period = (config.idle_after / 4)
            .min(Duration::from_secs(60))
            .max(Duration::from_millis(1));
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
        }
        let context = self.storage.turn_context(thread).await?;
        let cwd = workspace(&context).map_err(OpenError::Workspace)?;
        let (how, start) = match context.harness_session_id.clone() {
            Some(id) => ("resume", SessionStart::Resume(id)),
            None => ("new", SessionStart::New),
        };
        let default_mode = policy::default_mode(&context.harness).ok_or_else(|| {
            OpenError::Start(format!("no mode policy for harness {}", context.harness))
        })?;
        let remembered = self.storage.remembered_settings(&context.harness).await?;
        let mut handle = process::spawn(self.adapter.process_spec(&cwd))
            .map_err(|e| OpenError::Start(e.to_string()))?;
        let (tx, rx) = mpsc::unbounded_channel();
        let (raw_tx, raw_rx) = mpsc::unbounded_channel();
        intercept(self.offers.clone(), thread.clone(), raw_rx, tx);
        let setup = tokio::time::timeout(Duration::from_secs(5), async {
            let connection = Connection::open(&mut handle, raw_tx)
                .await
                .map_err(|e| e.to_string())?;
            let session = connection
                .start_session(&cwd, start)
                .await
                .map_err(|e| e.to_string())?;
            let opened = OpenSession {
                session_id: session.session_id,
                how,
                connection,
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

    /// §12.2: every opening sets the policy's default mode, since a new or
    /// resumed session starts at the person's own Claude defaults. Then the
    /// harness's remembered model and effort (§12.4), each skipped when the
    /// session does not offer it or the harness refuses it; a model change
    /// can move the mode, so the default is set again after.
    async fn apply_opening_settings(
        &self,
        thread: &ThreadId,
        opened: &OpenSession,
        mut offered: Offered,
        default_mode: &str,
        remembered: Option<(String, Option<String>)>,
    ) -> Result<(), AcpError> {
        if offered.current.mode != default_mode {
            let id = offered.ids.mode.clone();
            offered = self.set_option(thread, opened, &id, default_mode).await?;
        }
        let Some((model, effort)) = remembered else {
            return Ok(());
        };
        if offered.current.model != model {
            let id = offered.ids.model.clone();
            offered = match self
                .try_remembered(thread, opened, &id, &model, offered.offers_model(&model))
                .await?
            {
                Some(next) => next,
                None => offered,
            };
        }
        if let (Some(effort), Some(id)) = (effort, offered.ids.effort.clone())
            && offered.current.model == model
            && offered.current.effort.as_deref() != Some(effort.as_str())
        {
            let offers = offered.offers_effort(&effort);
            if let Some(next) = self
                .try_remembered(thread, opened, &id, &effort, offers)
                .await?
            {
                offered = next;
            }
        }
        if offered.current.mode != default_mode {
            let id = offered.ids.mode.clone();
            self.set_option(thread, opened, &id, default_mode).await?;
        }
        Ok(())
    }

    /// Sets one remembered value, or skips it with a line saying why: it is
    /// not on offer, or the harness refused it (an account that cannot use the
    /// model). Only a connection failure is an error.
    async fn try_remembered(
        &self,
        thread: &ThreadId,
        opened: &OpenSession,
        config_id: &str,
        value: &str,
        offered: bool,
    ) -> Result<Option<Offered>, AcpError> {
        if !offered {
            tracing::info!(config_id, value, "sessions.remembered_not_offered: dropped");
            return Ok(None);
        }
        match self.set_option(thread, opened, config_id, value).await {
            Ok(next) => Ok(Some(next)),
            Err(AcpError::Rpc(message)) => {
                tracing::info!(config_id, value, %message, "sessions.remembered_refused: dropped");
                Ok(None)
            }
            Err(error) => Err(error),
        }
    }

    /// Sets one of the session's options. The harness answers the complete
    /// set, which becomes the thread's latest offer and is published.
    pub async fn set_option(
        &self,
        thread: &ThreadId,
        opened: &OpenSession,
        config_id: &str,
        value: &str,
    ) -> Result<Offered, AcpError> {
        let answer: Value = opened
            .connection
            .set_option(&opened.session_id, config_id, value)
            .await?;
        self.offers.record(thread, &answer).map_err(AcpError::Rpc)
    }

    /// What the thread's open session offers now, if it is open.
    pub async fn offered(&self, thread: &ThreadId) -> Option<Offered> {
        self.offers.get(thread)
    }

    /// Every change to any thread's offer, as it happens.
    pub fn watch_options(&self) -> broadcast::Receiver<(ThreadId, Offered)> {
        self.offers.subscribe()
    }

    pub async fn take_events(
        &self,
        thread: &ThreadId,
    ) -> Option<mpsc::UnboundedReceiver<HarnessEvent>> {
        let mut live = self.live.lock().await;
        let item = live.get_mut(thread)?;
        item.last_used = Instant::now();
        item.events.take()
    }

    pub async fn give_back_events(
        &self,
        thread: &ThreadId,
        rx: mpsc::UnboundedReceiver<HarnessEvent>,
    ) {
        if let Some(item) = self.live.lock().await.get_mut(thread) {
            item.events = Some(rx);
            item.last_used = Instant::now();
        }
    }

    /// Records that a Planner turn has answered in the thread's adapter.
    pub async fn mark_answered(&self, thread: &ThreadId) {
        if let Some(item) = self.live.lock().await.get_mut(thread) {
            item.answered = true;
        }
    }

    pub async fn touch(&self, thread: &ThreadId) {
        if let Some(item) = self.live.lock().await.get_mut(thread) {
            item.last_used = Instant::now();
        }
    }

    pub async fn cancel(&self, thread: &ThreadId) {
        if let Some(item) = self.live.lock().await.get(thread) {
            item.opened.connection.cancel(&item.opened.session_id);
        }
    }

    pub async fn terminate(&self, thread: &ThreadId) -> io::Result<()> {
        let mut live = self.live.lock().await;
        if let Some(item) = live.get_mut(thread) {
            stop_handle(&mut item.handle).await?;
        }
        live.remove(thread);
        self.offers.forget(thread);
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
