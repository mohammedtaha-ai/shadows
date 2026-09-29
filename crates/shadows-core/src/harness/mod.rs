//! One job: the harnesses and each thread's open session (spec §12.2–§12.4,
//! §12.8, §14.4) — the `Harness` service.
//!
//! `change_model` keeps §12.7's order, and the order is the rule (§14.9): the
//! harness is checked, then the thread's busyness, before the session is
//! touched; the session is opened, then leased while its model changes, so a
//! turn that took it in between is refused.
//!
//! `sessions` holds the live adapter each open thread has, `settings` sets an
//! open session's options, `setup` is what a session opens with, `offers` the
//! latest choices each session offers, `context` the breakdown read on demand,
//! `model` the shapes a caller meets and `store` the queries. `Turns` drives a
//! turn through `Sessions`, the methods `contract.yaml` names.

mod context;
mod model;
mod offers;
mod sessions;
mod settings;
mod setup;
mod store;

use std::sync::Arc;

use shadows_agent::choices::{Offered, SessionChoices, for_client};
use shadows_agent::events::AccountLimits;
use shadows_agent::policy;
use tokio::sync::broadcast;

pub use model::{ContextBreakdown, HarnessInfo, RememberedSettings};
pub use sessions::{LeaseError, OpenError, OpenSession, Sessions, SessionsConfig};
pub use setup::prompt_version;
// What another write calls inside its own transaction (spec §14.6): a turn
// start remembers its model and effort.
pub(crate) use store::remember_settings;

use model::label;
use settings::ModelRefused;

use crate::db::{Storage, StorageError};
use crate::error::CoreError;
use crate::threads::ThreadId;

/// Harness: what storage holds, and the sessions each open thread has.
pub struct Harness {
    storage: Arc<Storage>,
    sessions: Arc<Sessions>,
}

impl Harness {
    pub(crate) fn new(storage: Arc<Storage>, sessions: Arc<Sessions>) -> Self {
        Self { storage, sessions }
    }

    /// Every harness Shadows knows, runnable or not, with the model and effort
    /// last used on it and the account limits it last reported.
    pub async fn list(&self) -> Result<Vec<HarnessInfo>, CoreError> {
        let mut list = Vec::new();
        for kind in policy::KNOWN {
            let available = policy::is_available(kind);
            let remembered = self
                .storage
                .remembered_settings(kind)
                .await?
                .map(|(model, effort)| RememberedSettings { model, effort });
            list.push(HarnessInfo {
                kind: kind.to_string(),
                label: label(kind).to_string(),
                available,
                reason: (!available).then(|| "Coming later".to_string()),
                remembered,
                limits: self.storage.latest_limits(kind).await?,
            });
        }
        Ok(list)
    }

    /// Opens the thread's harness session (spec §12.2) and answers what it
    /// offers. Idempotent: an open session answers what it holds.
    pub async fn open_session(&self, thread: &ThreadId) -> Result<SessionChoices, CoreError> {
        let context = self.storage.turn_context(thread).await?;
        if !policy::is_available(&context.harness) {
            return Err(CoreError::HarnessUnavailable(context.harness));
        }
        self.sessions.open(thread).await?;
        let offered = self.sessions.offered(thread).await.ok_or_else(|| {
            CoreError::HarnessStartFailed("the session closed as it opened".into())
        })?;
        self.choices(thread, &offered).await
    }

    /// Sets the thread's session to `model` (spec §12.7), opening it first if
    /// it is not open, and answers its choices as `open_session` does. The
    /// harness is checked, then the thread's busyness, before the session is
    /// touched (§14.9): a running turn's session is not changed. The change
    /// itself writes nothing durable (an opening it causes issues the thread's
    /// grant, as any opening does), and the remembered model does not move
    /// (§12.4).
    pub async fn change_model(
        &self,
        thread: &ThreadId,
        model: &str,
    ) -> Result<SessionChoices, CoreError> {
        let context = self.storage.turn_context(thread).await?;
        if !policy::is_available(&context.harness) {
            return Err(CoreError::HarnessUnavailable(context.harness));
        }
        if self.storage.thread_is_busy(thread).await? {
            return Err(StorageError::ThreadBusy.into());
        }
        let opened = self.sessions.open(thread).await?;
        let not_offered = |detail| CoreError::SettingNotOffered {
            what: "model".into(),
            id: model.into(),
            detail,
        };
        let offered =
            (self.sessions.change_model(thread, &opened, model).await).map_err(|refused| {
                match refused {
                    ModelRefused::NotOffered => not_offered(None),
                    ModelRefused::Harness(message) => not_offered(Some(message)),
                    ModelRefused::Lease(LeaseError::Busy) => StorageError::ThreadBusy.into(),
                    ModelRefused::Lease(e @ LeaseError::Closed) => {
                        CoreError::HarnessStartFailed(e.to_string())
                    }
                }
            })?;
        self.choices(thread, &offered).await
    }

    /// The context breakdown of the thread's session, read on demand (spec
    /// §12.8): the categories, or none with the reason. Nothing is written.
    pub async fn context(&self, thread: &ThreadId) -> Result<ContextBreakdown, CoreError> {
        self.storage.turn_context(thread).await?;
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

    /// Closes the thread's open session, if it has one (§12.2): its adapter's
    /// tree is ended and its grant revoked. `Threads` calls it when a thread's
    /// harness changes.
    pub(crate) async fn close_session(&self, thread: &ThreadId) -> std::io::Result<()> {
        self.sessions.terminate(thread).await
    }

    /// Every change to any thread's offer, as it happens. `Events` takes it
    /// for each subscriber, before the journal is read.
    pub(crate) fn watch_options(&self) -> broadcast::Receiver<(ThreadId, Offered)> {
        self.sessions.watch_options()
    }

    /// The account limits the thread's harness last reported (§12.8), which a
    /// turn's watcher recorded before it published the usage that asks. `None`
    /// when unknown, or when the thread or the limits cannot be read.
    pub(crate) async fn limits_of(&self, thread: &ThreadId) -> Option<AccountLimits> {
        let context = self.storage.turn_context(thread).await.ok()?;
        self.storage
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
        let context = self.storage.turn_context(thread).await?;
        let project = self.storage.get_project(&context.project_id).await?;
        let allowed = project
            .allowed_modes
            .get(&context.harness)
            .cloned()
            .unwrap_or_default();
        Ok(for_client(offered, &context.harness, &allowed))
    }
}
