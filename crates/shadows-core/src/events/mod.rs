//! One job: what clients watch live — the `Events` service.
//!
//! `Events` owns subscription and delivery, and emits nothing of its own: a
//! writer appends its durable event inside its own transaction
//! (`db::append_event`), `Turns` sends a turn's live items on the bus, and
//! `Turns` sends `plan_show`'s signal. `subscribe` takes every live source
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

use core_plans::UiSignal;
pub use model::EventCursor;
pub use store::StoredEvent;
pub use subscription::{Delivery, Subscription};

use crate::error::CoreError;
use crate::turns::Turns;
use core_db::Storage;
use core_harness::Harness;
use core_projects::{ProjectId, Projects};
use core_threads::ThreadId;

/// Events: every subscriber's stream, over the journal and the live sources.
pub struct Events {
    store: store::EventsStore,
    storage: Arc<Storage>,
    harness: Arc<Harness>,
    projects: Projects,
    turns: Turns,
    ui: broadcast::Sender<UiSignal>,
}

impl Events {
    pub(crate) fn new(
        storage: Arc<Storage>,
        harness: Arc<Harness>,
        projects: Projects,
        turns: Turns,
        ui: broadcast::Sender<UiSignal>,
    ) -> Self {
        Self {
            store: store::EventsStore::with_storage(storage.clone()),
            storage,
            harness,
            projects,
            turns,
            ui,
        }
    }

    /// A project notification tail projected by its owners in one read snapshot.
    #[cfg(feature = "test-support")]
    pub async fn read_project_events_after(
        &self,
        cursor: EventCursor,
        project: &ProjectId,
        limit: i64,
    ) -> Result<Vec<StoredEvent>, crate::StorageError> {
        self.store
            .read_project_events_after(cursor, project, limit)
            .await
    }

    /// Takes the committed-sequence watch, the bus, the options watch and the
    /// UI receiver before anything is read, then replay, CaughtUp, live.
    /// A commit or a live item that lands before
    /// the first `next` is already pending on a receiver, so it cannot fall
    /// between the replay and the live phase.
    pub fn subscribe(&self, thread: ThreadId, after: i64) -> Subscription {
        Subscription::new(
            subscription::Scope::Thread(thread),
            after,
            self.store.clone(),
            self.harness.clone(),
            subscription::Live {
                committed: self.storage.watch_committed(),
                bus: self.turns.subscribe_live(),
                options: self.harness.watch_options(),
                commands: self.harness.watch_commands(),
                signals: self.ui.subscribe(),
            },
        )
    }

    /// Plan notifications for a live project, using the same journal
    /// and committed watch as the thread stream. Unknown or removed is NotFound.
    pub async fn subscribe_project(
        &self,
        project: ProjectId,
        after: i64,
    ) -> Result<Subscription, CoreError> {
        self.projects.get(&project).await?;
        Ok(Subscription::new(
            subscription::Scope::Project(project),
            after,
            self.store.clone(),
            self.harness.clone(),
            subscription::Live {
                committed: self.storage.watch_committed(),
                bus: self.turns.subscribe_live(),
                options: self.harness.watch_options(),
                commands: self.harness.watch_commands(),
                signals: self.ui.subscribe(),
            },
        ))
    }
}
