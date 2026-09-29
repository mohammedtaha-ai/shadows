//! One job: the latest choices each open session offers (spec §12.4).
//!
//! Every `session/set_config_option` answer and every `config_option_update`
//! notification is the complete set, so each one replaces the thread's entry
//! here and is published to whoever watches (the SSE `options` frame).

use std::collections::HashMap;
use std::sync::Mutex;

use serde_json::Value;
use tokio::sync::{broadcast, mpsc};

use crate::threads::ThreadId;
use shadows_agent::choices::{Offered, parse};
use shadows_agent::events::HarnessEvent;

pub(super) struct Offers {
    latest: Mutex<HashMap<ThreadId, Offered>>,
    changed: broadcast::Sender<(ThreadId, Offered)>,
}

impl Offers {
    pub(super) fn new() -> Self {
        Self {
            latest: Mutex::new(HashMap::new()),
            changed: broadcast::channel(256).0,
        }
    }

    /// Parses `options`, keeps it as the thread's latest, and publishes it.
    pub(super) fn record(&self, thread: &ThreadId, options: &Value) -> Result<Offered, String> {
        let offered = parse(options)?;
        self.latest
            .lock()
            .expect("offers lock")
            .insert(thread.clone(), offered.clone());
        // Nobody watching is not a failure: the next opening answers them.
        let _ = self.changed.send((thread.clone(), offered.clone()));
        Ok(offered)
    }

    pub(super) fn get(&self, thread: &ThreadId) -> Option<Offered> {
        self.latest
            .lock()
            .expect("offers lock")
            .get(thread)
            .cloned()
    }

    pub(super) fn forget(&self, thread: &ThreadId) {
        self.latest.lock().expect("offers lock").remove(thread);
    }

    pub(super) fn subscribe(&self) -> broadcast::Receiver<(ThreadId, Offered)> {
        self.changed.subscribe()
    }
}

/// Forwards a connection's events to `to`, recording every options update
/// on the way: the harness sends them between turns too, when nobody reads
/// the thread's event channel. It runs in the connection's own dispatch, not
/// as a task of its own: a relay task let a turn's last update, received
/// before the prompt's answer, reach `to` only after the turn had ended.
/// `to` closes when the connection drops this.
pub(super) fn intercept(
    offers: std::sync::Arc<Offers>,
    thread: ThreadId,
    to: mpsc::UnboundedSender<HarnessEvent>,
) -> impl Fn(HarnessEvent) + Clone + Send + Sync + 'static {
    move |event| {
        if let HarnessEvent::Options(options) = &event
            && let Err(error) = offers.record(&thread, options)
        {
            tracing::warn!(thread_id = %thread, %error, "sessions.options_unreadable");
        }
        let _ = to.send(event);
    }
}
