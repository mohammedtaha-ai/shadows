//! One job: one subscriber's durable replay, handoff and live phase (spec
//! §2.10).
//!
//! Every live source is taken BEFORE a single journal row is read (see
//! `Events::subscribe`):
//!
//! - the storage's committed-sequence signal (spec §2.4: raised after each
//!   commit that appended to the journal). A commit that lands while the
//!   replay is being read shows up as a pending change, so it cannot fall into
//!   the gap between the replay's last read and the live phase.
//! - the transient bus, for what is never stored: deltas and turn ends;
//!   and beside it the session's options and `plan_show`'s signals.
//!
//! Durable events reach the subscriber only by reading the journal after
//! `last_seq` — in the replay, and again on every signal change in the live
//! phase — so each one is delivered once, in sequence order, and
//! de-duplication by `seq` is structural rather than left to the client. The
//! bus's `Entry` items are therefore not delivered: the same entry arrives as
//! its durable `ThreadEntryAppended` event. Items for another thread are
//! dropped.

use std::collections::VecDeque;
use std::sync::Arc;

use shadows_agent::choices::{Offered, SessionChoices};
use shadows_agent::events::{AccountLimits, HarnessEvent};
use tokio::sync::broadcast::error::RecvError;
use tokio::sync::{broadcast, watch};

use super::{EventCursor, StoredEvent, UiSignal};
use crate::db::Storage;
use crate::harness::Harness;
use crate::projects::ProjectId;
use crate::threads::ThreadId;
use crate::turns::OperationId;

/// How many journal rows one read returns; the replay reads until one is empty.
const BATCH: i64 = 500;

/// One frame of a thread's stream, in the order the subscriber must see it.
#[derive(Debug)]
pub enum Delivery {
    /// One journal event, once each, in `seq` order.
    Durable(StoredEvent),
    /// The replay is over; `seq` is the last one it delivered.
    CaughtUp { seq: i64 },
    /// Streamed text of a running turn. Transient.
    Delta { op: OperationId, text: String },
    /// The harness finished a turn. Transient.
    TurnEnd {
        op: OperationId,
        subtype: &'static str,
        stop_reason: Option<String>,
    },
    /// The session's context use, with the account limits its harness last
    /// reported (`None` when unknown). A `size` of 0 is no window. Transient.
    Usage {
        thread: ThreadId,
        used: u64,
        size: u64,
        limits: Option<AccountLimits>,
    },
    /// The session's choices changed, after the policy (§12.4). Transient.
    Options {
        thread: ThreadId,
        choices: SessionChoices,
    },
    /// The Planner showed a plan (§13.9). Its card was delivered first, as a
    /// `Durable` `PlanShown` event. Transient.
    PlanShow(UiSignal),
    /// This subscriber fell behind a live source; transient items were
    /// dropped, durable ones were not.
    Lagged,
    /// The journal could not be read. The subscription is over: the next
    /// `next` answers why.
    Fatal(String),
}

/// The live sources, taken before the replay is read.
pub(super) struct Live {
    pub(super) committed: watch::Receiver<i64>,
    pub(super) bus: broadcast::Receiver<(ThreadId, OperationId, HarnessEvent)>,
    pub(super) options: broadcast::Receiver<(ThreadId, Offered)>,
    pub(super) signals: broadcast::Receiver<UiSignal>,
}

pub(super) enum Scope {
    Thread(ThreadId),
    Project(ProjectId),
}

/// One subscriber's replay-then-live stream, filtered by its scope.
pub struct Subscription {
    scope: Scope,
    storage: Arc<Storage>,
    harness: Arc<Harness>,
    live: Live,
    /// The highest `seq` delivered, starting at the client's `after`.
    last_seq: i64,
    /// Journal rows read and not yet delivered, in `seq` order.
    read: VecDeque<StoredEvent>,
    /// The journal may hold rows after `last_seq`: true for the replay, and
    /// again each time the committed-sequence signal moves.
    reading: bool,
    caught_up: bool,
    /// Why the stream is over, once it is.
    over: Option<&'static str>,
}

impl Subscription {
    pub(super) fn new(
        scope: Scope,
        after: i64,
        storage: Arc<Storage>,
        harness: Arc<Harness>,
        live: Live,
    ) -> Self {
        Self {
            scope,
            storage,
            harness,
            live,
            last_seq: after,
            read: VecDeque::new(),
            reading: true,
            caught_up: false,
            over: None,
        }
    }

    /// The next frame, or `Err(why)` once over, `why` being the text
    /// `sse.closed` logs.
    ///
    /// The journal is drained before any live source is polled, and the live
    /// sources are polled `biased`, journal first: a publisher commits
    /// (raising the signal) before it sends the transient item that follows,
    /// so a turn's `TurnEnd` stays behind the durable entry it ends.
    pub async fn next(&mut self) -> Result<Delivery, &'static str> {
        loop {
            if let Some(why) = self.over {
                return Err(why);
            }
            if let Some(event) = self.read.pop_front() {
                self.last_seq = event.seq;
                return Ok(Delivery::Durable(event));
            }
            if self.reading {
                if let Some(fatal) = self.read_journal().await {
                    return Ok(fatal);
                }
                if !self.read.is_empty() {
                    continue;
                }
                self.reading = false;
                if !self.caught_up {
                    self.caught_up = true;
                    return Ok(Delivery::CaughtUp { seq: self.last_seq });
                }
                continue;
            }
            if let Some(delivery) = self.live().await {
                return delivery;
            }
        }
    }

    /// Reads the journal after `last_seq` into `read`. `Some(Fatal)` when it
    /// could not be read, which ends the subscription.
    async fn read_journal(&mut self) -> Option<Delivery> {
        let cursor = EventCursor(self.last_seq);
        let batch = match &self.scope {
            Scope::Thread(thread) => self.storage.read_events_after(cursor, thread, BATCH).await,
            Scope::Project(project) => {
                self.storage
                    .read_project_events_after(cursor, project, BATCH)
                    .await
            }
        };
        match batch {
            Ok(batch) => {
                self.read.extend(batch);
                None
            }
            Err(e) => {
                // Spec §3.2: the cause is logged here; the client is told only
                // what it needs to know — that the stream is over.
                tracing::error!(error = %e, "sse.replay_failed");
                self.over = Some("error: journal read failed");
                Some(Delivery::Fatal("the journal could not be read".into()))
            }
        }
    }

    /// Waits for one live source to wake, in today's order: committed, bus,
    /// options, signals. `None` when it woke for nothing to deliver: the
    /// journal moved (it is read next), or the item is another thread's.
    async fn live(&mut self) -> Option<Result<Delivery, &'static str>> {
        // Project notifications are durable only, on the same committed watch.
        if matches!(self.scope, Scope::Project(_)) {
            return match self.live.committed.changed().await {
                Ok(()) => {
                    self.reading = true;
                    None
                }
                Err(_) => Some(self.end("shutdown: storage closed")),
            };
        }
        // The select only decides which source woke; acting on it happens
        // after, so no borrowed `watch::Ref` is held across an await.
        let woke = tokio::select! {
            biased;
            changed = self.live.committed.changed() => Woke::Committed(changed.is_ok()),
            received = self.live.bus.recv() => Woke::Bus(received),
            offered = self.live.options.recv() => Woke::Options(offered),
            signal = self.live.signals.recv() => Woke::Signal(signal),
        };
        match woke {
            Woke::Committed(true) => {
                self.reading = true;
                None
            }
            Woke::Committed(false) => Some(self.end("shutdown: storage closed")),
            Woke::Bus(Ok((thread, op, item))) if matches!(&self.scope, Scope::Thread(own) if own == &thread) => {
                self.transient(thread, op, item).await.map(Ok)
            }
            Woke::Options(Ok((thread, offered))) if matches!(&self.scope, Scope::Thread(own) if own == &thread) =>
            {
                // `None` when the thread's policy cannot be read; the next
                // opening answers.
                let choices = self.harness.choices(&thread, &offered).await.ok()?;
                Some(Ok(Delivery::Options { thread, choices }))
            }
            Woke::Signal(Ok(signal)) if matches!(&self.scope, Scope::Thread(own) if own == &signal.thread_id) =>
            {
                // The card's durable event was committed before the signal
                // was sent, so the journal has already delivered it.
                Some(Ok(Delivery::PlanShow(signal)))
            }
            Woke::Bus(Ok(_)) | Woke::Options(Ok(_)) | Woke::Signal(Ok(_)) => None,
            // Spec §8.4 case 7: a subscriber falling behind or disconnecting
            // never cancels work. It resubscribes with its last seq.
            Woke::Bus(Err(RecvError::Lagged(_)))
            | Woke::Options(Err(RecvError::Lagged(_)))
            | Woke::Signal(Err(RecvError::Lagged(_))) => Some(Ok(Delivery::Lagged)),
            Woke::Bus(Err(RecvError::Closed)) => Some(self.end("shutdown: bus closed")),
            Woke::Options(Err(RecvError::Closed)) => Some(self.end("shutdown: options closed")),
            Woke::Signal(Err(RecvError::Closed)) => Some(self.end("shutdown: signals closed")),
        }
    }

    /// A bus item of this thread as a frame, or `None` for one this stream
    /// does not deliver: entries arrive durable, options through the options
    /// watch.
    async fn transient(
        &self,
        thread: ThreadId,
        op: OperationId,
        item: HarnessEvent,
    ) -> Option<Delivery> {
        Some(match item {
            HarnessEvent::Usage { used, size, .. } => Delivery::Usage {
                limits: self.harness.limits_of(&thread).await,
                thread,
                used,
                size,
            },
            HarnessEvent::Chunk { text, .. } => Delivery::Delta { op, text },
            HarnessEvent::TurnEnd {
                subtype,
                stop_reason,
            } => Delivery::TurnEnd {
                op,
                subtype,
                stop_reason,
            },
            _ => return None,
        })
    }

    fn end(&mut self, why: &'static str) -> Result<Delivery, &'static str> {
        self.over = Some(why);
        Err(why)
    }
}

/// Which live source woke, and what it held.
enum Woke {
    Committed(bool),
    Bus(Result<(ThreadId, OperationId, HarnessEvent), RecvError>),
    Options(Result<(ThreadId, Offered), RecvError>),
    Signal(Result<UiSignal, RecvError>),
}
