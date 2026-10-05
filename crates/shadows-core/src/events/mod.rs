//! One job: what clients watch live (spec §2.10, §14.4) — the `Events` service.
//!
//! `Events` owns subscription and delivery, and emits nothing of its own: a
//! writer appends its durable event inside its own transaction
//! (`db::append_event`), `Turns` sends a turn's live items on the bus, and
//! `Plans` sends `plan_show`'s signal. `subscribe` takes every live source
//! before the journal is read, and `Subscription::next` replays the journal
//! after the client's cursor, answers `CaughtUp`, then goes live.
//!
//! `model` holds the event shapes, `store` reads the journal back, and
//! `subscription` is the replay-then-live loop.

mod model;
mod store;
mod subscription;

use std::sync::Arc;

use tokio::sync::broadcast;

#[cfg(feature = "test-support")]
pub use model::Causation;
pub use model::{Actor, DurableEvent, EventCursor, UiSignal};
pub use store::StoredEvent;
pub use subscription::{Delivery, Subscription};

use crate::app::Bus;
use crate::db::Storage;
use crate::error::CoreError;
use crate::harness::Harness;
use crate::projects::ProjectId;
use crate::threads::ThreadId;

/// Events: every subscriber's stream, over the journal and the live sources.
pub struct Events {
    storage: Arc<Storage>,
    harness: Arc<Harness>,
    bus: Bus,
    ui: broadcast::Sender<UiSignal>,
}

impl Events {
    pub(crate) fn new(
        storage: Arc<Storage>,
        harness: Arc<Harness>,
        bus: Bus,
        ui: broadcast::Sender<UiSignal>,
    ) -> Self {
        Self {
            storage,
            harness,
            bus,
            ui,
        }
    }

    /// Takes the committed-sequence watch, the bus, the options watch and the
    /// UI receiver before anything is read, then replay, CaughtUp, live (§2.4,
    /// §6.18). Nothing is read here: a commit or a live item that lands before
    /// the first `next` is already pending on a receiver, so it cannot fall
    /// between the replay and the live phase.
    pub fn subscribe(&self, thread: ThreadId, after: i64) -> Subscription {
        Subscription::new(
            subscription::Scope::Thread(thread),
            after,
            self.storage.clone(),
            self.harness.clone(),
            subscription::Live {
                committed: self.storage.watch_committed(),
                bus: self.bus.subscribe(),
                options: self.harness.watch_options(),
                commands: self.harness.watch_commands(),
                signals: self.ui.subscribe(),
            },
        )
    }

    /// Plan notifications for a live project (§16.8), using the same journal
    /// and committed watch as the thread stream. Unknown or removed is NotFound.
    pub async fn subscribe_project(
        &self,
        project: ProjectId,
        after: i64,
    ) -> Result<Subscription, CoreError> {
        self.storage.get_project(&project).await?;
        Ok(Subscription::new(
            subscription::Scope::Project(project),
            after,
            self.storage.clone(),
            self.harness.clone(),
            subscription::Live {
                committed: self.storage.watch_committed(),
                bus: self.bus.subscribe(),
                options: self.harness.watch_options(),
                commands: self.harness.watch_commands(),
                signals: self.ui.subscribe(),
            },
        ))
    }
}
