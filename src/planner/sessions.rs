//! One job: the live adapter connection each open thread holds.

use std::{collections::HashMap, io, path::PathBuf, sync::Arc, time::Duration};

use serde_json::Value;
use tokio::{
    sync::{Mutex, mpsc},
    time::Instant,
};
use tracing::Instrument;

use crate::{
    agent::{
        acp::{Connection, SessionStart},
        claude::ClaudeAdapter,
        events::HarnessEvent,
    },
    process::{self, ProcessHandle},
    storage::{Storage, StorageError},
    thread::{ThreadId, TurnContext},
};

#[derive(Debug, Clone, Copy)]
pub struct SessionsConfig {
    pub idle_after: Duration,
    pub cancel_wait: Duration,
}

impl Default for SessionsConfig {
    fn default() -> Self {
        Self {
            idle_after: Duration::from_secs(15 * 60),
            cancel_wait: Duration::from_secs(10),
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

#[derive(Clone)]
pub struct OpenSession {
    pub session_id: String,
    pub how: &'static str,
    pub options: Value,
    connection: Connection,
}

impl OpenSession {
    pub fn connection(&self) -> &Connection {
        &self.connection
    }
}

struct Live {
    handle: ProcessHandle,
    opened: OpenSession,
    events: Option<mpsc::UnboundedReceiver<HarnessEvent>>,
    last_used: Instant,
}

pub struct Sessions {
    adapter: Arc<ClaudeAdapter>,
    storage: Storage,
    config: SessionsConfig,
    live: Mutex<HashMap<ThreadId, Live>>,
}

impl Sessions {
    pub fn new(adapter: Arc<ClaudeAdapter>, storage: Storage, config: SessionsConfig) -> Arc<Self> {
        let sessions = Arc::new(Self {
            adapter,
            storage,
            config,
            live: Mutex::new(HashMap::new()),
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
        }
        let context = self.storage.turn_context(thread).await?;
        let cwd = workspace(&context).map_err(OpenError::Workspace)?;
        let (how, start) = match context.harness_session_id {
            Some(id) => ("resume", SessionStart::Resume(id)),
            None => ("new", SessionStart::New),
        };
        let mut handle = process::spawn(self.adapter.process_spec(&cwd))
            .map_err(|e| OpenError::Start(e.to_string()))?;
        let (tx, rx) = mpsc::unbounded_channel();
        let setup = tokio::time::timeout(Duration::from_secs(5), async {
            let connection = Connection::open(&mut handle, tx)
                .await
                .map_err(|e| e.to_string())?;
            let session = connection
                .start_session(&cwd, start)
                .await
                .map_err(|e| e.to_string())?;
            let options = connection
                .set_option(&session.session_id, "mode", "acceptEdits")
                .await
                .map_err(|e| e.to_string())?;
            Ok::<_, String>(OpenSession {
                session_id: session.session_id,
                how,
                options,
                connection,
            })
        })
        .await
        .unwrap_or_else(|_| Err("harness setup timed out".into()));
        let opened = match setup {
            Ok(opened) => opened,
            Err(error) => {
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
            },
        );
        Ok(opened)
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
