//! One job: run the two-phase Planner turn spawn and its cancellation
//! interlock (spec §2.3, §2.7, §8.3).
//!
//! `Cancelled` means Shadows confirmed the managed execution is no longer
//! running AND persisted the terminal transition (spec §2.3) — never only
//! that a user asked for it. `PlannerTurn::stop` enforces that by construction:
//! it requests (TX #1, `Storage::request_cancellation`), terminates through the
//! containment handle that owns the tree, confirms the tree is gone, and only
//! then writes Cancelled (TX #2, `Storage::mark_operation_cancelled`). If
//! termination cannot be confirmed, Cancelled is never written; the operation
//! is left non-terminal for a later runtime's recovery to resolve honestly
//! (`Storage::reconcile_orphans`, which writes `Interrupted`, never
//! `Cancelled`).
//!
//! There is no `mark_operation_interrupted` here on purpose: interruption is
//! not a transition anyone requests, it is what recovery concludes about work
//! a previous runtime left behind, so it belongs solely to `reconcile_orphans`.
//!
//! **Why the stream reader and `stop` live in one file.** They are not two
//! jobs that happen to be adjacent: they are the two sides of one arbitration.
//! §8.4 requires that exactly one of them writes the terminal transition, and
//! the rule that decides which one — registration in `LiveHandles`, plus the
//! one fact that tells "still running" from "already ended on its own" — is
//! only reviewable if both sides are read together. Splitting them would leave
//! two files that must be read as one, which CLAUDE.md names as the split that
//! does not count.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use tokio::sync::{Mutex, broadcast};

use crate::agent::claude::ClaudeHarness;
use crate::agent::{AgentHarness, AgentInvocation, StreamItem};
use crate::events::Actor;
use crate::operation::{FailureStage, OperationId};
use crate::process::{ProcessHandle, spawn};
use crate::runtime::Runtime;
use crate::storage::StorageError;
use crate::thread::{NewThreadEntry, ThreadId};

/// One live turn's registration: the containment handle that owns its tree,
/// and the single fact `stop` needs in order not to invert spec §8.4 case 4.
pub(crate) struct LiveTurn {
    handle: ProcessHandle,
    /// Set by the reader the moment it classifies this turn's `TurnEnd`, which
    /// is strictly before the process exits and before the reader competes for
    /// the map. It is what lets `stop` tell "this turn is still running and I
    /// am about to end it" from "this turn already produced its own ending and
    /// I would only be discarding it".
    turn_end_seen: Arc<AtomicBool>,
}

impl LiveTurn {
    /// Spec §8.4 case 4's distinguishing fact. Either signal alone is
    /// incomplete: a `TurnEnd` is observed before the process has actually
    /// gone, and a process can die without ever emitting one.
    fn ended_on_its_own(&mut self) -> bool {
        self.turn_end_seen.load(Ordering::SeqCst) || self.handle.has_exited()
    }
}

/// Live handles for operations this runtime owns. Spec §8.3: termination goes
/// through the containment handle that owns the tree, never through a PID read
/// from the database, because the OS reuses PIDs.
///
/// The map is also the interlock's single arbitration point (§8.4): a turn's
/// terminal transition may only be written by whoever holds its registration,
/// and a side that cannot confirm what it is claiming puts the registration
/// back rather than keeping it.
///
/// The inner map is `pub(crate)` so `cli::serve` can enumerate this runtime's
/// live operations at shutdown.
#[derive(Default)]
pub struct LiveHandles(pub(crate) Mutex<HashMap<OperationId, LiveTurn>>);

impl LiveHandles {
    /// Test-only visibility into whether an operation still holds a live
    /// handle. `pub(crate)` does not reach an integration test, which is a
    /// separate crate, so this follows the same feature-gated pattern as
    /// `storage::test_support`: compiled in for `cargo test` only, never for
    /// an ordinary build.
    #[cfg(feature = "test-support")]
    pub async fn contains(&self, op_id: &OperationId) -> bool {
        self.0.lock().await.contains_key(op_id)
    }

    /// Test-only. Arms the registered handle so its next `terminate_tree`
    /// fails, which is the only way to reach spec §8.4 case 6's live-handle
    /// branch — see `ProcessHandle::force_termination_failure`. Returns
    /// whether a registration was there to arm, so a test cannot pass by
    /// arming nothing.
    #[cfg(feature = "test-support")]
    pub async fn force_termination_failure(&self, op_id: &OperationId) -> bool {
        match self.0.lock().await.get_mut(op_id) {
            Some(turn) => {
                turn.handle.force_termination_failure();
                true
            }
            None => false,
        }
    }
}

/// What a caller asks for, bundled rather than passed positionally.
/// `PlannerTurn::start` otherwise takes eight parameters, which is both a
/// clippy lint (`too_many_arguments`, refused here rather than suppressed —
/// see CLAUDE.md) and the exact shape of mistake this project's
/// `NewThreadEntry` precedent exists to close: same-typed neighbours
/// (`thread_id`, `prompt`, `resume_session_id` are all string-ish) that the
/// compiler cannot tell apart at a positional call site.
#[derive(Debug, Clone)]
pub struct PlannerTurnRequest {
    pub thread_id: ThreadId,
    pub prompt: String,
    pub cwd: PathBuf,
    /// Present on a resumed turn. Continuity belongs to the harness, not to us.
    pub resume_session_id: Option<String>,
}

pub struct PlannerTurn;

impl PlannerTurn {
    /// Returns the operation id as soon as Pending is committed. The turn
    /// keeps running in a background task; the caller subscribes to `bus`
    /// separately for the stream.
    pub async fn start(
        runtime: Arc<Runtime>,
        handles: Arc<LiveHandles>,
        harness: Arc<ClaudeHarness>,
        request: PlannerTurnRequest,
        bus: broadcast::Sender<(OperationId, StreamItem)>,
    ) -> Result<OperationId, StorageError> {
        let PlannerTurnRequest {
            thread_id,
            prompt,
            cwd,
            resume_session_id,
        } = request;

        // TX #1: the durable attempt exists before anything spawns.
        let op_id = runtime
            .storage
            .create_pending_operation(&thread_id, &runtime.instance_id)
            .await?;

        let invocation = AgentInvocation {
            operation_id: op_id.clone(),
            role: "Planner".into(),
            model: "sonnet".into(),
            prompt,
            cwd,
            resume_session_id,
            session_id: uuid::Uuid::new_v4().to_string(),
        };

        // Prepare: resolve, build the environment, ready the workspace. A
        // failure here is not a spawn failure — no process ever existed.
        let spec = harness.to_process_spec(&invocation);
        if !spec.executable.exists() && spec.executable.components().count() > 1 {
            runtime
                .storage
                .mark_operation_failed(
                    &op_id,
                    FailureStage::Prepare,
                    &format!(
                        "harness executable not found: {}",
                        spec.executable.display()
                    ),
                )
                .await?;
            return Ok(op_id);
        }

        let mut handle = match spawn(spec) {
            Ok(h) => h,
            Err(e) => {
                runtime
                    .storage
                    .mark_operation_failed(&op_id, FailureStage::Spawn, &e.to_string())
                    .await?;
                return Ok(op_id);
            }
        };

        // Spec §8.3: register the handle, THEN commit Running. A committed
        // transition without a registered handle is a state we must not
        // produce — `stop` would have nothing to terminate.
        let lines = handle.take_stdout_lines();
        let turn_end_seen = Arc::new(AtomicBool::new(false));
        {
            let mut map = handles.0.lock().await;
            map.insert(
                op_id.clone(),
                LiveTurn {
                    handle,
                    turn_end_seen: turn_end_seen.clone(),
                },
            );
        }

        // TX #2. The child is already running, and the only thing that can
        // reach it is the registration made two lines above, so returning this
        // error without undoing that would leave a live harness attached to an
        // operation id the caller never received: unreachable, unterminable,
        // and still Pending. The registration is withdrawn and the tree killed
        // before the error propagates.
        if let Err(error) = runtime
            .storage
            .mark_operation_started(&op_id, &runtime.instance_id)
            .await
        {
            let orphan = handles.0.lock().await.remove(&op_id);
            if let Some(mut turn) = orphan {
                let _ = turn.handle.terminate_tree();
                let _ = turn.handle.wait().await;
            }
            return Err(error);
        }

        let reader_op = op_id.clone();
        let reader_runtime = runtime.clone();
        let reader_handles = handles.clone();
        let reader_harness = harness.clone();
        let reader_thread = thread_id.clone();
        let agent_role = invocation.role.clone();
        tokio::spawn(async move {
            let mut turn_end: Option<serde_json::Value> = None;
            if let Some(mut lines) = lines {
                loop {
                    let line = match lines.next_line().await {
                        Ok(Some(line)) => line,
                        Ok(None) => break,
                        Err(error) => {
                            // A read failure is not an ordinary end of stream,
                            // and the old `while let Ok(Some(_))` could not
                            // tell them apart. It still breaks — there is
                            // nothing left to read — but the turn's ending is
                            // now decided by the exit status below, not by the
                            // fact that reading stopped.
                            tracing::error!(
                                operation_id = %reader_op,
                                %error,
                                "planner.stream_read_failed: the harness stream ended in an error"
                            );
                            break;
                        }
                    };
                    let item = reader_harness.classify(&line);
                    match &item {
                        // Durable: write before forwarding. The UI may drop a
                        // frame; the record may not.
                        StreamItem::Entry { text, .. } => {
                            if let Err(error) = reader_runtime
                                .storage
                                .append_thread_entry(
                                    &reader_thread,
                                    NewThreadEntry {
                                        kind: "AgentMessage",
                                        // The author is the agent whose turn
                                        // this is, which is the same actor for
                                        // every line of it. The stream's own
                                        // per-line `role` ("assistant", or
                                        // "user" on a tool-result echo) is a
                                        // label on the transport, not a second
                                        // author, and its `uuid` is a
                                        // harness-side line identity, not an
                                        // actor at all: putting either in
                                        // `Actor.id` would make every message
                                        // look like it had a different author.
                                        // The uuid is dropped rather than
                                        // stored because `thread_entry` has
                                        // nowhere to put a harness-side
                                        // identity — see that table's OPEN
                                        // block in the schema spec, whose
                                        // trigger is the first feature that
                                        // must match a stored entry back to a
                                        // streamed line.
                                        author: Actor {
                                            kind: "Agent".into(),
                                            id: agent_role.clone(),
                                        },
                                        body: text,
                                        refs: &[],
                                    },
                                )
                                .await
                            {
                                // Not a shutdown path: this is the durable
                                // write of the conversation itself. Swallowing
                                // this would lose a message with nothing
                                // anywhere saying so, so it is surfaced loudly
                                // even though the stream keeps running — one
                                // dropped entry must not also abort the turn
                                // the user is watching.
                                tracing::error!(
                                    operation_id = %reader_op,
                                    thread_id = %reader_thread,
                                    %error,
                                    "planner.append_thread_entry_failed: a durable turn entry was lost"
                                );
                            }
                        }
                        StreamItem::TurnEnd {
                            subtype,
                            stop_reason,
                        } => {
                            turn_end = Some(serde_json::json!({
                                "subtype": subtype, "stop_reason": stop_reason
                            }));
                            // Published before the item reaches the bus, so
                            // that anyone who has seen the turn end knows the
                            // interlock has seen it too.
                            turn_end_seen.store(true, Ordering::SeqCst);
                        }
                        _ => {}
                    }
                    let _ = bus.send((reader_op.clone(), item));
                }
            }

            // The stream ended — either the process exited on its own, or
            // `stop` killed it and the kill also closed stdout, which looks
            // identical from here. Which side writes the terminal state is
            // decided by the registration, not by racing two storage CASes:
            // whoever's `remove` returns `Some` owns the outcome. `None` means
            // `stop` took ownership and will write `Cancelled`, so this task
            // writes nothing (§8.4 case 5).
            let claimed = reader_handles.0.lock().await.remove(&reader_op);
            let Some(mut turn) = claimed else {
                return;
            };

            // Owning the registration means owning the handle, so the real
            // exit is available and §8.4 case 4's "persist Completed or Failed
            // from the real exit" can be obeyed instead of assuming success.
            // `wait` blocks until the leader is gone: a harness that closes
            // stdout and keeps running holds this task here, which is honest —
            // the turn genuinely has not ended — rather than recording an
            // ending that did not happen.
            let status = turn.handle.wait().await;
            let ended_cleanly = matches!(&status, Ok(code) if code.success());
            let ending = match &status {
                Ok(code) => format!("{code}"),
                Err(error) => format!("exit status unreadable: {error}"),
            };

            let written = match turn_end {
                Some(outcome) if ended_cleanly => {
                    reader_runtime
                        .storage
                        .mark_operation_completed(&reader_op, outcome)
                        .await
                }
                Some(_) => {
                    reader_runtime
                        .storage
                        .mark_operation_failed(
                            &reader_op,
                            FailureStage::Run,
                            &format!("the harness emitted its turn-end result, then {ending}"),
                        )
                        .await
                }
                None => {
                    reader_runtime
                        .storage
                        .mark_operation_failed(
                            &reader_op,
                            FailureStage::Run,
                            &format!("the harness ended without a turn-end result: {ending}"),
                        )
                        .await
                }
            };
            if let Err(error) = written {
                // This task is the sole writer of a naturally-ending turn's
                // terminal state. Discarding this would leave the operation
                // Running forever with nothing anywhere saying why.
                tracing::error!(
                    operation_id = %reader_op,
                    %error,
                    "planner.terminal_transition_failed: a finished turn was left non-terminal"
                );
            }
        });

        Ok(op_id)
    }

    /// Spec §2.3: request, terminate, confirm, then write Cancelled. If
    /// termination cannot be confirmed — no live handle, or the tree refuses
    /// to terminate — Cancelled is not written and the operation is left for
    /// recovery (spec §8.4 case 6).
    pub async fn stop(
        runtime: Arc<Runtime>,
        handles: Arc<LiveHandles>,
        op_id: &OperationId,
    ) -> Result<(), StorageError> {
        runtime
            .storage
            .request_cancellation(op_id, Actor::user("local"))
            .await?;

        let mut map = handles.0.lock().await;
        let Some(mut turn) = map.remove(op_id) else {
            // No live handle: either it already exited (the reader task has
            // or will record its real outcome), or this runtime never
            // registered one for it. Either way termination cannot be
            // confirmed here, so Cancelled must not be written.
            return Ok(());
        };

        // §8.4 case 4. The turn produced its own ending in the window between
        // the request and this lock. Terminating now would succeed against an
        // already-dead tree and write Cancelled over an outcome the reader has
        // already built, which is Shadows claiming to have stopped something
        // that had already stopped. The registration goes back so the reader
        // still owns the outcome and persists the real exit.
        if turn.ended_on_its_own() {
            map.insert(op_id.clone(), turn);
            return Ok(());
        }

        // §8.4 case 6. Ownership of the outcome is claimed only once
        // termination is actually under way. Keeping the registration here
        // would drop the one handle that can still reach the tree — the
        // process would keep running, unterminable, and no one would ever
        // resolve the operation.
        if turn.handle.terminate_tree().is_err() {
            map.insert(op_id.clone(), turn);
            return Ok(());
        }
        drop(map);

        // `wait` returning is the confirmation that the leader (and, through
        // job/group containment, the tree it owns) has been reaped. The exit
        // status itself is discarded deliberately: we just killed this
        // process ourselves, so its status reports the kill, not an outcome
        // of the turn, and Cancelled records that the operation was stopped,
        // not what its exit code was.
        let _ = turn.handle.wait().await;
        runtime.storage.mark_operation_cancelled(op_id).await
    }
}
