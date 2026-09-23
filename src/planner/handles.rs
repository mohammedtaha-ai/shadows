//! One job: the registry of live turns this runtime owns.
//!
//! Spec §8.3: termination goes through the containment handle that owns the
//! tree, never through a PID read from the database, because the OS reuses
//! PIDs. This registry is where those handles live, and nowhere else.
//!
//! It is also the interlock's single arbitration point (§8.4): a turn's
//! terminal transition may only be written by whoever holds its registration,
//! and a side that cannot confirm what it is claiming puts the registration
//! back rather than keeping it. The rule itself — which side that is — is read
//! in `mod.rs`, where both sides of it live; this file only holds the facts
//! the rule reads.
//!
//! **Closing.** Spec §8.5's shutdown stops every turn this runtime owns, which
//! is only a finite list if no turn can join it afterwards. `close` sets the
//! flag and snapshots the list under the same lock `register` takes, so a turn
//! is either in the snapshot or refused at registration — there is no third
//! place for it to be.

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use tokio::sync::Mutex;

use crate::operation::OperationId;
use crate::process::ProcessHandle;

/// One live turn's registration: the containment handle that owns its tree,
/// and the two facts §8.4 needs to answer its two separate questions.
///
/// **They are separate questions and the code must not fuse them.** Whether
/// there is a tree to terminate is `ProcessHandle::has_exited` and nothing
/// else — §8.4 case 4's precondition is that the process *exited*, not that it
/// said it was finishing. Whether this turn already produced its own ending is
/// `turn_end_seen`. A harness can emit its result and then stay alive for a
/// long time, which is case 3 ("terminate and reap the registered tree"), and
/// reading `turn_end_seen` as "nothing to kill" makes such a harness
/// unstoppable while `stop` reports success.
pub(crate) struct LiveTurn {
    pub(crate) handle: ProcessHandle,
    /// Set by the reader the moment it classifies this turn's `TurnEnd`, which
    /// is strictly before the process exits and before the reader competes for
    /// the map. It decides who writes the outcome, never whether to terminate.
    pub(crate) turn_end_seen: Arc<AtomicBool>,
    /// Set by `stop` when it terminated this tree but left the outcome to the
    /// reader (§8.4 case 3 over a turn that had already ended). It travels with
    /// the registration rather than in an `Arc`, because only whoever holds the
    /// registration reads it. Without it the reader would see the exit status
    /// of a process *we* killed and record a `Run` failure for a turn that
    /// reported success.
    pub(crate) terminated_by_stop: bool,
    /// The turn's `planner.turn` span, so `stop`'s lines carry its ids too.
    pub(crate) span: tracing::Span,
}

/// What the lock guards: the live turns, and whether new ones may still join.
/// One lock for both, because `close` and `register` must each see the other's
/// write whole — that is the whole of the §8.5 guarantee this file provides.
#[derive(Default)]
pub(crate) struct Registry {
    pub(crate) turns: HashMap<OperationId, LiveTurn>,
    closed: bool,
}

/// Live handles for operations this runtime owns.
#[derive(Default)]
pub struct LiveHandles(pub(crate) Mutex<Registry>);

impl LiveHandles {
    /// Registers a freshly spawned turn, unless shutdown has begun — then the
    /// turn is handed back, still owned by the caller, which must terminate it
    /// itself (§8.4 case 2: register for termination only, never `Running`).
    pub(crate) async fn register(
        &self,
        op_id: OperationId,
        turn: LiveTurn,
    ) -> Result<(), Box<LiveTurn>> {
        let mut registry = self.0.lock().await;
        if registry.closed {
            return Err(Box::new(turn));
        }
        registry.turns.insert(op_id, turn);
        Ok(())
    }

    /// Takes a registration out. Whoever gets `Some` owns the turn's outcome.
    pub(crate) async fn claim(&self, op_id: &OperationId) -> Option<LiveTurn> {
        self.0.lock().await.turns.remove(op_id)
    }

    /// Refuses every later registration and answers the turns registered now.
    /// Spec §8.5: from here on the list of this runtime's live turns can only
    /// shrink.
    pub(crate) async fn close(&self) -> Vec<OperationId> {
        let mut registry = self.0.lock().await;
        registry.closed = true;
        registry.turns.keys().cloned().collect()
    }

    /// Whether shutdown has begun. A caller about to start a turn asks this
    /// first so that a refused turn leaves nothing durable behind; the answer
    /// can go stale at once, which is why `register` asks again under the lock.
    pub async fn is_closed(&self) -> bool {
        self.0.lock().await.closed
    }

    /// Test-only visibility into whether an operation still holds a live
    /// handle. `pub(crate)` does not reach an integration test, which is a
    /// separate crate, so this follows the same feature-gated pattern as
    /// `storage::test_support`: compiled in for `cargo test` only, never for
    /// an ordinary build.
    #[cfg(feature = "test-support")]
    pub async fn contains(&self, op_id: &OperationId) -> bool {
        self.0.lock().await.turns.contains_key(op_id)
    }

    /// Test-only. The registered tree's leader pid, so a test can assert that
    /// a cancelled turn's process is actually gone rather than trusting that
    /// `stop` said so.
    #[cfg(feature = "test-support")]
    pub async fn pid(&self, op_id: &OperationId) -> Option<u32> {
        self.0
            .lock()
            .await
            .turns
            .get(op_id)
            .and_then(|turn| turn.handle.id())
    }

    /// Test-only. Arms the registered handle so its next `terminate_tree`
    /// fails, which is the only way to reach spec §8.4 case 6's live-handle
    /// branch — see `ProcessHandle::force_termination_failure`. Returns
    /// whether a registration was there to arm, so a test cannot pass by
    /// arming nothing.
    #[cfg(feature = "test-support")]
    pub async fn force_termination_failure(&self, op_id: &OperationId) -> bool {
        match self.0.lock().await.turns.get_mut(op_id) {
            Some(turn) => {
                turn.handle.force_termination_failure();
                true
            }
            None => false,
        }
    }
}
