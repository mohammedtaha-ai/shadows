//! Owning the live adapter cache at session opening.
use super::assistant::Opening;
use super::commands::keep_commands;
use super::offers::intercept;
use super::sessions::{OpenError, OpenSession, Sessions, stop_handle};
use super::titles::keep_titles;
use crate::threads::ThreadId;
use shadows_agent::events::HarnessEvent;
use shadows_agent::{
    acp::{Connection, SessionStart},
    policy,
};
use shadows_process::{self as process, ProcessHandle};
use std::collections::HashMap;
use std::sync::{Arc, atomic::Ordering};
use tokio::{
    sync::{Mutex, mpsc},
    time::Instant,
};

pub(super) struct Live {
    pub(super) handle: ProcessHandle,
    pub(super) opened: OpenSession,
    /// `None` while a turn (or a `/context` read) has taken them.
    pub(super) events: Option<mpsc::UnboundedReceiver<HarnessEvent>>,
    pub(super) last_used: Instant,
    /// Whether a Planner turn has answered in this adapter: before one has,
    /// `/context` can stall a fresh session for tens of seconds (§12.8).
    pub(super) answered: bool,
}

/// One thread's place in `Sessions`: empty until an adapter opens, and while
/// one is replaced. Its own lock is held through an opening, a close or a
/// stop; the map's lock only while a slot is found or added (§16.6). A slot
/// is never removed, so two callers can never hold two slots for one thread.
pub(super) type Slot = Arc<Mutex<Option<Live>>>;

pub(super) type LiveMap = Mutex<HashMap<ThreadId, Slot>>;

impl Sessions {
    pub(super) async fn open_live(&self, thread: &ThreadId) -> Result<OpenSession, OpenError> {
        let slot = self.slot(thread).await;
        let mut live = slot.lock().await;
        // Deletion commits before Stop/close. Even a cached adapter cannot
        // make a removed thread available to a new opening in that interval.
        let context = crate::threads::Threads::turn_context_in(
            &mut *self
                .storage
                .reader()
                .begin()
                .await
                .map_err(crate::db::StorageError::from)?,
            thread,
        )
        .await?;
        self.check_context(&context).await?;
        if let Some(current) = live.as_mut() {
            if !current.handle.has_exited() && !current.opened.connection.is_closed() {
                current.last_used = Instant::now();
                return Ok(current.opened.clone());
            }
            stop_handle(&mut current.handle)
                .await
                .map_err(|e| OpenError::Start(format!("could not close dead adapter: {e}")))?;
            *live = None;
            self.forget(thread).await;
        }
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
        let preference = super::preference_key(&context.harness, context.profile.as_deref());
        let mut remembered = self.storage.remembered(&preference).await?;
        let Opening {
            cwd,
            setup,
            spec,
            aylul_models,
            aylul_usage,
        } = self.opening(thread, &context).await?;
        if let Some((models, default)) = &aylul_models {
            if !remembered
                .model
                .as_ref()
                .is_some_and(|m| models.contains(m))
            {
                remembered.model = Some(default.clone());
            }
            self.offers
                .restrict_models(thread, models.clone(), default.clone());
        }
        let mut handle = match process::spawn(spec) {
            Ok(handle) => handle,
            Err(error) => {
                self.setups.forget(thread).await;
                return Err(OpenError::Start(error.to_string()));
            }
        };
        let (tx, rx) = mpsc::unbounded_channel();
        let events = intercept(self.offers.clone(), thread.clone(), tx);
        let events = keep_titles(self.storage.clone(), thread.clone(), events);
        let events = keep_commands(self.commands.clone(), thread.clone(), events);
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
                aylul_usage,
                how,
                connection,
                generation: self.generations.fetch_add(1, Ordering::Relaxed),
            };
            let mut offered = self.offers.record(thread, &session.options)?;
            if let Some((models, default)) = &aylul_models {
                if !models.iter().all(|id| offered.offers_model(id)) {
                    return Err("adapter did not offer all provider models".into());
                }
                let raw = shadows_agent::choices::parse(&session.options)?;
                if !models.contains(&raw.current.model) {
                    offered = self
                        .set_option(thread, &opened, &offered.ids.model, default)
                        .await
                        .map_err(|e| e.to_string())?;
                }
            }
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
                self.forget(thread).await;
                if let Err(cleanup) = stop_handle(&mut handle).await {
                    tracing::error!(%cleanup, "sessions.failed_open_cleanup");
                }
                return Err(OpenError::Start(error));
            }
        };
        *live = Some(Live {
            handle,
            opened: opened.clone(),
            events: Some(rx),
            last_used: Instant::now(),
            answered: false,
        });
        Ok(opened)
    }
}
