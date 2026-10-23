//! Thread harness sessions and the context each invocation heard.
//!
//! Harness checks availability, then durable busyness before touching a session.
//! A lease protects setting changes from a turn that acquired the session after
//! those checks. Sessions is a private cache; other services use Harness methods.

mod access;
mod assistant;
mod commands;
mod context;
mod extension_home;
mod fork;
mod live;
mod mode;
mod model;
mod offers;
mod profile_home;
mod sessions;
mod settings;
mod setup;
mod standards;
mod store;
mod titles;

use std::sync::Arc;

use shadows_agent::choices::{Offered, SessionChoices};
use shadows_agent::events::{AccountLimits, SlashCommand};
use shadows_agent::policy;
use tokio::sync::broadcast;

pub use assistant::AylulConfig;
pub use model::{
    AylulUsageSnapshot, ContextBreakdown, HarnessInfo, InvocationVersions, InvocationView,
    RememberedSettings,
};
use sessions::Sessions;
pub use sessions::{LeaseError, OpenError, OpenSession, SessionsConfig};
pub(crate) use setup::InvocationContext;

pub(crate) use model::InvocationUsage;
pub(crate) use model::NewInvocation;
use settings::SettingRefused;

use crate::db::{Storage, StorageError};
use crate::error::CoreError;
use crate::extensions::Extensions;
use crate::projects::Projects;
use crate::threads::ThreadId;

/// Harness: what storage holds, and the sessions each open thread has.
pub struct Harness {
    store: store::HarnessStore,
    operations: crate::operations::Operations,
    sessions: Arc<Sessions>,
    projects: Projects,
}

impl Harness {
    /// The configured AYLUL profile, without exposing its environment.
    pub fn aylul_profile(&self) -> Result<&str, CoreError> {
        self.sessions
            .config
            .aylul
            .as_ref()
            .map(|_| "aylul")
            .ok_or(CoreError::AylulNotConfigured)
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn with_owners(
        adapter: Arc<shadows_agent::claude::ClaudeAdapter>,
        storage: Arc<Storage>,
        config: SessionsConfig,
        owners: crate::app::AylulOwners,
        instructions: crate::Instructions,
        grants: crate::grants::Grants,
        operations: crate::operations::Operations,
        projects: Projects,
    ) -> Arc<Self> {
        let sessions = Sessions::with_owners(
            adapter,
            storage.clone(),
            config,
            owners,
            instructions,
            grants,
        );
        Arc::new(Self {
            store: store::HarnessStore::with_storage(storage),
            operations,
            sessions,
            projects,
        })
    }

    /// Changes the durable harness, then closes the adapter if its kind changed.
    pub async fn set_thread_harness(
        &self,
        command_id: String,
        thread: &ThreadId,
        harness: &str,
    ) -> Result<crate::threads::PlanningThread, CoreError> {
        Projects::known_harness(harness)?;
        let mut conn = self
            .store
            .storage
            .reader()
            .begin()
            .await
            .map_err(StorageError::from)?;
        let context = crate::threads::Threads::turn_context_in(&mut conn, thread).await?;
        drop(conn);
        self.projects
            .harness_fits(&context.project_id, harness)
            .await?;
        let before = context.harness;
        let command = crate::db::CommandContext::user(
            command_id,
            "thread.harness",
            serde_json::json!({"thread_id": thread, "harness": harness}),
        );
        let updated = self
            .store
            .set_thread_harness(&command, thread, harness)
            .await?;
        if updated.harness != before
            && let Err(error) = self.close_session(thread).await
        {
            tracing::error!(%error, thread_id = %thread, "thread.harness_change_close_failed");
        }
        Ok(updated)
    }

    /// Deletes an agent and closes its live adapters. The one way an agent is
    /// deleted: its adapters are live state this service owns. The tombstone, its
    /// variables and its extension switches go in one write, before any close.
    pub async fn delete_agent(&self, command_id: String, slug: &str) -> Result<(), CoreError> {
        let agents = self.sessions.owners.agents.clone();
        let owned = slug.to_owned();
        self.store
            .storage
            .write_txn_as(move |conn| {
                Box::pin(async move {
                    agents.delete_in(conn, command_id, &owned).await?;
                    Ok::<_, CoreError>(Extensions::remove_agent_in(conn, &owned).await?)
                })
            })
            .await?;
        self.sessions.close_agent(slug).await
    }

    /// Every harness Shadows knows, runnable or not, with the model and effort
    /// last used on it and the account limits it last reported.
    pub async fn list(&self) -> Result<Vec<HarnessInfo>, CoreError> {
        let mut list = Vec::new();
        for kind in policy::KNOWN {
            let available = self.sessions.harness_available(kind).await?;
            let remembered = self
                .store
                .remembered_settings(kind)
                .await?
                .map(|(model, effort)| RememberedSettings { model, effort });
            list.push(HarnessInfo {
                kind: kind.to_string(),
                label: Self::label(kind).to_string(),
                available,
                reason: (!available).then(|| "Coming later".to_string()),
                remembered,
                limits: self.store.latest_limits(kind).await?,
            });
        }
        Ok(list)
    }

    /// Opens the thread's harness session and answers what it
    /// offers. Idempotent: an open session answers what it holds.
    pub async fn open_session(&self, thread: &ThreadId) -> Result<SessionChoices, CoreError> {
        let context = crate::threads::Threads::turn_context_in(
            &mut *self
                .store
                .storage
                .reader()
                .begin()
                .await
                .map_err(crate::db::StorageError::from)?,
            thread,
        )
        .await?;
        self.check_available(&context).await?;
        self.sessions.open(thread).await?;
        self.apply_idle_mode(thread).await?;
        let offered = self.sessions.offered(thread).await.ok_or_else(|| {
            CoreError::HarnessStartFailed("the session closed as it opened".into())
        })?;
        self.choices(thread, &offered).await
    }

    /// Sets the thread's session to `model`, opening it first if
    /// it is not open, and answers its choices as `open_session` does. The
    /// harness is checked, then the thread's busyness, before the session is
    /// touched: a running turn's session is not changed. When the
    /// session moves to the model it is set to that model's remembered effort,
    /// if still offered. The change itself writes nothing durable (an
    /// opening it causes issues the thread's grant, as any opening does), and
    /// nothing is remembered: a turn started is.
    pub async fn change_model(
        &self,
        thread: &ThreadId,
        model: &str,
    ) -> Result<SessionChoices, CoreError> {
        let (harness, opened) = self.open_idle(thread).await?;
        let effort = self.store.remembered(&harness).await?.efforts.remove(model);
        let changed = self
            .sessions
            .change_model(thread, &opened, model, effort.as_deref());
        let offered = changed.await.map_err(|r| refusal("model", model, r))?;
        self.choices(thread, &offered).await
    }

    /// Sets the thread's session to `effort`, exactly as
    /// `change_model` sets a model: same checks in the same order, nothing
    /// durable, nothing remembered. An effort the session's model does not
    /// offer, or one the harness refuses, is SettingNotOffered.
    pub async fn change_effort(
        &self,
        thread: &ThreadId,
        effort: &str,
    ) -> Result<SessionChoices, CoreError> {
        let (_, opened) = self.open_idle(thread).await?;
        let changed = self.sessions.change_effort(thread, &opened, effort);
        let offered = changed.await.map_err(|r| refusal("effort", effort, r))?;
        self.choices(thread, &offered).await
    }

    /// The order before a person's change: the thread's harness is
    /// available, then no operation is open on it, then its session is opened.
    /// Answers the harness kind and the open session.
    async fn open_idle(&self, thread: &ThreadId) -> Result<(String, OpenSession), CoreError> {
        let context = crate::threads::Threads::turn_context_in(
            &mut *self
                .store
                .storage
                .reader()
                .begin()
                .await
                .map_err(crate::db::StorageError::from)?,
            thread,
        )
        .await?;
        self.check_idle(thread, &context).await?;
        let opened = self.sessions.open(thread).await?;
        Ok((
            preference_key(&context.harness, context.profile.as_deref()),
            opened,
        ))
    }

    async fn check_available(
        &self,
        context: &crate::threads::TurnContext,
    ) -> Result<(), CoreError> {
        self.sessions.check_context(context).await?;
        if !self.sessions.harness_available(&context.harness).await? {
            return Err(CoreError::HarnessUnavailable(context.harness.clone()));
        }
        Ok(())
    }

    pub(crate) async fn check_idle(
        &self,
        thread: &ThreadId,
        context: &crate::threads::TurnContext,
    ) -> Result<(), CoreError> {
        self.check_available(context).await?;
        if self.operations.thread_is_busy(thread).await? {
            return Err(CoreError::Refused {
                code: crate::ErrorCode::ThreadBusy,
                message: "the thread has a turn running".into(),
            });
        }
        Ok(())
    }

    /// The context breakdown of the thread's session, read on demand:
    /// the categories, or none with the reason. Nothing is written.
    pub async fn context(&self, thread: &ThreadId) -> Result<ContextBreakdown, CoreError> {
        crate::threads::Threads::turn_context_in(
            &mut *self
                .store
                .storage
                .reader()
                .begin()
                .await
                .map_err(crate::db::StorageError::from)?,
            thread,
        )
        .await?;
        Ok(match self.sessions.context(thread).await {
            Ok(categories) => ContextBreakdown {
                categories: Some(categories),
                reason: None,
            },
            Err(why) => ContextBreakdown {
                categories: None,
                reason: Some(why.reason().to_string()),
            },
        })
    }

    /// Closes the thread's open session, if it has one: its adapter's
    /// tree is ended and its grant revoked. Turns calls it after thread removal.
    pub(crate) async fn close_session(&self, thread: &ThreadId) -> std::io::Result<()> {
        self.sessions.terminate(thread).await
    }

    /// Every change to any thread's offer, as it happens. `Events` takes it
    /// for each subscriber, before the journal is read.
    pub(crate) fn watch_options(&self) -> broadcast::Receiver<(ThreadId, Offered)> {
        self.sessions.watch_options()
    }

    /// The thread's latest `/` list, if its open session sent one.
    pub(crate) fn commands_of(&self, thread: &ThreadId) -> Option<Vec<SlashCommand>> {
        self.sessions.commands.get(thread)
    }

    /// Every change to any thread's `/` list, as it happens. `Events` takes
    /// it for each subscriber, before the journal is read.
    pub(crate) fn watch_commands(&self) -> broadcast::Receiver<(ThreadId, Vec<SlashCommand>)> {
        self.sessions.commands.subscribe()
    }

    /// The account limits the thread's harness last reported, which a
    /// turn's watcher recorded before it published the usage that asks. `None`
    /// when unknown, or when the thread or the limits cannot be read.
    pub(crate) async fn limits_of(&self, thread: &ThreadId) -> Option<AccountLimits> {
        let context = crate::threads::Threads::turn_context_in(
            &mut *self.store.storage.reader().begin().await.ok()?,
            thread,
        )
        .await
        .ok()?;
        self.store
            .latest_limits(&context.harness)
            .await
            .ok()
            .flatten()
    }

    /// What a client is offered on `thread`: the session's choices after the
    /// policy of the thread's harness and its project's allowed modes.
    /// `Events` calls it for each offer it delivers.
    pub(crate) async fn choices(
        &self,
        thread: &ThreadId,
        offered: &Offered,
    ) -> Result<SessionChoices, CoreError> {
        let context = crate::threads::Threads::turn_context_in(
            &mut *self
                .store
                .storage
                .reader()
                .begin()
                .await
                .map_err(crate::db::StorageError::from)?,
            thread,
        )
        .await?;
        let allowed = self
            .projects
            .allowed_modes(&context.project_id, &context.harness)
            .await?;
        let mut choices = offered.for_client(&context.harness, &allowed);
        let slot = self.sessions.slot(thread).await;
        let live = slot.lock().await;
        if let Some(item) = live.as_ref() {
            choices.capabilities = item.opened.connection().prompt_capabilities();
        }
        // A busy session has not applied a deferred pick. An idle session's
        // reported mode can differ because its model moved it.
        if live.as_ref().is_some_and(|item| item.events.is_none())
            || self.operations.thread_is_busy(thread).await?
        {
            choices.current.mode = self.selected_mode(thread).await?;
        }
        Ok(choices)
    }
}

/// A refused change of `what` to `id`, as a caller meets it.
fn refusal(what: &str, id: &str, refused: SettingRefused) -> CoreError {
    let not_offered = |detail| CoreError::SettingNotOffered {
        what: what.into(),
        id: id.into(),
        detail,
    };
    match refused {
        SettingRefused::NotOffered => not_offered(None),
        SettingRefused::Harness(message) => not_offered(Some(message)),
        SettingRefused::Lease(LeaseError::Busy) => CoreError::Refused {
            code: crate::ErrorCode::ThreadBusy,
            message: "the thread has a turn running".into(),
        },
        SettingRefused::Lease(e @ LeaseError::Closed) => {
            CoreError::HarnessStartFailed(e.to_string())
        }
    }
}

fn preference_key(harness: &str, profile: Option<&str>) -> String {
    if harness == policy::AYLUL {
        format!("aylul:{}", profile.unwrap_or("aylul"))
    } else {
        harness.into()
    }
}
