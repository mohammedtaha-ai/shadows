//! One job: the latest `/` list each open session sent.
//!
//! Every `available_commands_update` is the complete list, so each one
//! replaces the thread's entry here and is published to whoever watches (the
//! SSE `commands` frame). Memory only: a restarted daemon has none until the
//! session opens again and the adapter sends it again.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use tokio::sync::broadcast;

use core_threads::ThreadId;
use shadows_agent::events::{HarnessEvent, SlashCommand};

pub(super) struct Commands {
    latest: Mutex<HashMap<ThreadId, Vec<SlashCommand>>>,
    changed: broadcast::Sender<(ThreadId, Vec<SlashCommand>)>,
}

impl Commands {
    pub(super) fn new() -> Self {
        Self {
            latest: Mutex::new(HashMap::new()),
            changed: broadcast::channel(256).0,
        }
    }

    fn record(&self, thread: &ThreadId, list: Vec<SlashCommand>) {
        self.latest
            .lock()
            .expect("commands lock")
            .insert(thread.clone(), list.clone());
        // Nobody watching is not a failure: the next stream sends it.
        let _ = self.changed.send((thread.clone(), list));
    }

    pub(super) fn get(&self, thread: &ThreadId) -> Option<Vec<SlashCommand>> {
        self.latest
            .lock()
            .expect("commands lock")
            .get(thread)
            .cloned()
    }

    /// Publishes nothing: a client keeps its last list.
    pub(super) fn forget(&self, thread: &ThreadId) {
        self.latest.lock().expect("commands lock").remove(thread);
    }

    pub(super) fn subscribe(&self) -> broadcast::Receiver<(ThreadId, Vec<SlashCommand>)> {
        self.changed.subscribe()
    }
}

/// Takes the `/` list off the connection, in its own dispatch, and hands
/// every other event on to `next`. It arrives between turns too, when nobody
/// reads the thread's events.
impl Commands {
    pub(super) fn keep_commands(
        commands: Arc<Commands>,
        thread: ThreadId,
        next: impl Fn(HarnessEvent) + Clone + Send + Sync + 'static,
    ) -> impl Fn(HarnessEvent) + Clone + Send + Sync + 'static {
        move |event| match event {
            HarnessEvent::Commands(list) => commands.record(&thread, list),
            other => next(other),
        }
    }
}

#[cfg(feature = "test-support")]
impl crate::Harness {
    /// Delivers a harness command report through the opening event dispatcher.
    pub fn report_commands_for_test(&self, thread: &ThreadId, list: Vec<SlashCommand>) {
        Commands::keep_commands(self.sessions.commands.clone(), thread.clone(), |_| {})(
            HarnessEvent::Commands(list),
        );
    }
}
