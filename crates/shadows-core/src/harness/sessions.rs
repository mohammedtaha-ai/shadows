//! One job: the live adapter connection each open thread holds.
use core_db::{Storage, StorageError};

use std::{
    collections::HashMap,
    io,
    sync::{Arc, atomic::AtomicU64},
    time::Duration,
};

use shadows_process::ProcessHandle;
use tokio::{
    sync::{Mutex, broadcast, mpsc},
    time::Instant,
};
use tracing::Instrument;

use super::assistant::AylulConfig;
use super::commands::Commands;
use super::live::{Live, LiveMap, Slot};
use super::offers::Offers;
use super::setup::Setups;
use core_threads::ThreadId;
use shadows_agent::{
    acp::Connection, choices::Offered, claude::ClaudeAdapter, events::HarnessEvent,
};

#[derive(Debug, Clone)]
pub struct SessionsConfig {
    pub secrets: Option<Arc<core_secrets::Secrets>>,
    pub idle_after: Duration,
    pub cancel_wait: Duration,
    /// How long an adapter has to start and open its session.
    pub setup_wait: Duration,
    /// How long the on-demand context breakdown waits for `/context`.
    pub context_wait: Duration,
    /// Shadows' `/mcp`, as the daemon bound it; `None` in tests that need no MCP.
    pub mcp_url: Option<String>,
    /// AYLUL's profile; `None` when the daemon has no `aylul` section.
    pub aylul: Option<AylulConfig>,
}

impl Default for SessionsConfig {
    fn default() -> Self {
        Self {
            secrets: None,
            idle_after: Duration::from_secs(15 * 60),
            cancel_wait: Duration::from_secs(10),
            setup_wait: Duration::from_secs(20),
            context_wait: Duration::from_secs(5),
            mcp_url: None,
            aylul: None,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum OpenError {
    #[error(transparent)]
    Env(#[from] crate::CoreError),
    #[error(transparent)]
    Secrets(#[from] core_secrets::SecretsError),
    #[error(transparent)]
    Provider(#[from] core_providers::ProvidersError),
    #[error(transparent)]
    Person(#[from] core_person::PersonError),
    #[error(transparent)]
    Agent(#[from] core_agents::AgentsError),
    #[error(transparent)]
    Storage(#[from] StorageError),
    #[error("harness start failed: {0}")]
    Start(String),
    /// The project's directory cannot be run in. The text is written here,
    /// about the user's own project, so a client may show it.
    #[error("{0}")]
    Workspace(String),
}

/// A turn cannot take the thread's session.
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
    pub aylul_usage: Option<super::AylulUsageSnapshot>,
    pub how: &'static str,
    pub(super) connection: Connection,
    pub(super) generation: u64,
}

impl OpenSession {
    pub fn connection(&self) -> &Connection {
        &self.connection
    }
}

pub(super) struct Sessions {
    pub(super) owners: crate::harness::AylulOwners,
    pub(super) homes: super::profile_home::Homes,
    pub(super) adapter: Arc<ClaudeAdapter>,
    pub(super) storage: Arc<Storage>,
    pub(super) config: SessionsConfig,
    pub(super) live: LiveMap,
    pub(super) offers: Arc<Offers>,
    pub(super) commands: Arc<Commands>,
    pub(super) generations: AtomicU64,
    pub(super) setups: Setups,
}

impl Sessions {
    /// Reads the explicit configuration owners served by AppCore.
    pub(crate) fn with_owners(
        adapter: Arc<ClaudeAdapter>,
        storage: Arc<Storage>,
        config: SessionsConfig,
        owners: crate::harness::AylulOwners,
        instructions: core_instructions::Instructions,
        grants: core_grants::Grants,
    ) -> Arc<Self> {
        let period = (config.idle_after / 4)
            .min(Duration::from_secs(60))
            .max(Duration::from_millis(1));
        let sessions = Arc::new(Self {
            owners,
            homes: super::profile_home::Homes::default(),
            setups: Setups::new(
                storage.clone(),
                instructions,
                config.mcp_url.clone(),
                grants,
            ),
            adapter,
            storage,
            config,
            live: Mutex::new(HashMap::new()),
            offers: Arc::new(Offers::new()),
            commands: Arc::new(Commands::new()),
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

    /// Closes every live adapter of a deleted agent's threads. A close that fails is
    /// logged and does not undo the deletion.
    pub(crate) async fn close_agent(&self, slug: &str) -> Result<(), crate::CoreError> {
        for thread in core_threads::Threads::list_for_profile_in(
            &mut *self
                .storage
                .reader()
                .begin()
                .await
                .map_err(StorageError::from)?,
            slug,
        )
        .await?
        {
            if let Err(error) = self.terminate(&thread.id).await {
                tracing::warn!(thread_id=%thread.id, %error, "agent.adapter_close_failed");
            }
        }
        Ok(())
    }

    /// Drops what a closed or failed session left: its offer, its `/` list
    /// and its grant.
    pub(super) async fn forget(&self, thread: &ThreadId) {
        self.offers.forget(thread);
        self.commands.forget(thread);
        self.setups.forget(thread).await;
    }

    pub async fn open(&self, thread: &ThreadId) -> Result<OpenSession, OpenError> {
        self.open_with_images(thread, false).await
    }

    pub(super) async fn open_with_images(
        &self,
        thread: &ThreadId,
        has_images: bool,
    ) -> Result<OpenSession, OpenError> {
        self.open_live(thread, has_images)
            .instrument(tracing::info_span!("sessions.open", thread_id = %thread))
            .await
    }

    /// The thread's slot, added empty when it has none. The map's lock is
    /// released before the caller locks the slot.
    pub(super) async fn slot(&self, thread: &ThreadId) -> Slot {
        let mut live = self.live.lock().await;
        live.entry(thread.clone()).or_default().clone()
    }

    /// A copy of the map, so each slot is locked after the map's lock is gone.
    async fn slots(&self) -> HashMap<ThreadId, Slot> {
        self.live.lock().await.clone()
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
        let slot = self.slot(thread).await;
        let mut live = slot.lock().await;
        let item = live.as_mut()?;
        item.last_used = Instant::now();
        item.events.take()
    }

    /// Takes the session's events for a turn: whoever holds them has the
    /// session to itself. A `/context` read gives them back
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
        let slot = self.slot(thread).await;
        loop {
            {
                let mut live = slot.lock().await;
                let item = same_adapter(&mut live, opened).ok_or(LeaseError::Closed)?;
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
        if let Some(item) = same_adapter(&mut *self.slot(thread).await.lock().await, opened) {
            item.events = Some(rx);
            item.last_used = Instant::now();
        }
    }

    /// Records that a Planner turn has answered in `opened`'s adapter.
    pub async fn mark_answered(&self, thread: &ThreadId, opened: &OpenSession) {
        if let Some(item) = same_adapter(&mut *self.slot(thread).await.lock().await, opened) {
            item.answered = true;
        }
    }

    pub async fn touch(&self, thread: &ThreadId) {
        if let Some(item) = self.slot(thread).await.lock().await.as_mut() {
            item.last_used = Instant::now();
        }
    }

    pub async fn terminate(&self, thread: &ThreadId) -> io::Result<()> {
        let slot = self.slot(thread).await;
        let mut live = slot.lock().await;
        if let Some(item) = live.as_mut() {
            Self::stop_handle(&mut item.handle).await?;
        }
        *live = None;
        self.forget(thread).await;
        Ok(())
    }

    /// Terminates `opened`'s adapter and reaps it. An adapter absent from
    /// the map was already reaped by whoever removed it, so `Ok` then: a
    /// replacement opened since is another adapter, and is left alone.
    pub async fn terminate_adapter(
        &self,
        thread: &ThreadId,
        opened: &OpenSession,
    ) -> io::Result<()> {
        let slot = self.slot(thread).await;
        let mut live = slot.lock().await;
        if let Some(item) = same_adapter(&mut live, opened) {
            Self::stop_handle(&mut item.handle).await?;
            *live = None;
            self.forget(thread).await;
        }
        Ok(())
    }

    pub async fn close_all(&self) -> io::Result<()> {
        let mut first = None;
        for (thread, slot) in self.slots().await {
            let mut live = slot.lock().await;
            let Some(item) = live.as_mut() else { continue };
            match Self::stop_handle(&mut item.handle).await {
                Ok(()) => {
                    *live = None;
                    self.forget(&thread).await;
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
        for (id, slot) in self.slots().await {
            // A slot someone holds is in use, so it is not idle.
            let Ok(mut live) = slot.try_lock() else {
                continue;
            };
            let Some(item) = live.as_mut().filter(|item| {
                item.events.is_some() && item.last_used.elapsed() >= self.config.idle_after
            }) else {
                continue;
            };
            if let Err(error) = Self::stop_handle(&mut item.handle).await {
                tracing::warn!(thread_id = %id, %error, "sessions.idle_close_failed");
                continue;
            }
            *live = None;
            self.forget(&id).await;
        }
    }

    #[cfg(feature = "test-support")]
    pub async fn live_count(&self) -> usize {
        let mut count = 0;
        for (_, slot) in self.slots().await {
            count += usize::from(slot.lock().await.is_some());
        }
        count
    }

    /// The process id of the thread's adapter, while one is live.
    #[cfg(feature = "test-support")]
    pub async fn pid(&self, thread: &ThreadId) -> Option<u32> {
        let slot = self.slot(thread).await;
        let live = slot.lock().await;
        live.as_ref().and_then(|item| item.handle.id())
    }

    /// Arms the thread's adapter so its next termination fails — see
    /// `ProcessHandle::force_termination_failure`. Returns whether an adapter
    /// was there to arm, so a test cannot pass by arming nothing.
    #[cfg(feature = "test-support")]
    pub async fn force_termination_failure(&self, thread: &ThreadId) -> bool {
        match self.slot(thread).await.lock().await.as_mut() {
            Some(item) => {
                item.handle.force_termination_failure();
                true
            }
            None => false,
        }
    }

    /// The adapter this daemon runs, whose paths and versions every
    /// invocation records.
    pub fn adapter(&self) -> &ClaudeAdapter {
        &self.adapter
    }

    pub fn cancel_wait(&self) -> Duration {
        self.config.cancel_wait
    }
}

impl Sessions {
    pub(super) async fn stop_handle(handle: &mut ProcessHandle) -> io::Result<()> {
        if !handle.has_exited() {
            handle.terminate_tree()?;
        }
        tokio::time::timeout(Duration::from_secs(5), handle.wait())
            .await
            .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "adapter did not exit in time"))?
            .map(|_| ())
    }
}

/// The thread's live adapter, if it is still the one `opened` was made on.
fn same_adapter<'a>(live: &'a mut Option<Live>, opened: &OpenSession) -> Option<&'a mut Live> {
    live.as_mut()
        .filter(|item| item.opened.generation == opened.generation)
}
