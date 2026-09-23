//! One job: decide and persist how a live Planner turn ends (spec §2.3, §8.4).
//!
//! Starting one is the neighbouring file's job. `spawn.rs` turns a request
//! into a registered, running operation and hands it here; everything after
//! that — the stream, the ending, the cancellation interlock — is this file.
//! The two other neighbours hold what the interlock reads and who else calls
//! it: `handles.rs` is the registry of live turns, and `shutdown.rs` stops
//! every one of them when the runtime stops (§8.5).
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
//! two facts `LiveTurn` carries — is only reviewable if both sides are read
//! together. Splitting them would leave two files that must be read as one,
//! which CLAUDE.md names as the split that does not count.
//!
//! **Terminating and naming the ending are separate decisions.** `stop`
//! terminates anything still alive (§8.4 case 3) and only then asks who names
//! the ending. Fusing the two — declining to kill a live tree because its turn
//! already reported a result — leaves a harness that hangs after its result
//! unstoppable, while `stop` returns success.

mod handles;
mod shutdown;
mod spawn;

pub use handles::LiveHandles;
pub(crate) use handles::LiveTurn;
pub use shutdown::shut_down;
pub use spawn::{PlannerTurnRequest, StartError};

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use tokio::sync::broadcast;
use tracing::Instrument;

use crate::agent::claude::ClaudeHarness;
use crate::agent::{AgentHarness, StreamItem};
use crate::events::Actor;
use crate::operation::{FailureStage, OperationId};
use crate::process::StdoutLines;
use crate::runtime::Runtime;
use crate::storage::StorageError;
use crate::thread::{NewThreadEntry, ThreadId};

/// The turn-end subtype the harness evidence measured as a successful turn
/// (`docs/evidence/harness/SERVE_STREAM_SPIKE.md`). Stated as the one value
/// that means success rather than as a list of failing values, because the
/// evidence measured this one and did not enumerate the others: a blacklist
/// here would be invented, and would silently record the next unmeasured
/// failure subtype as a completed turn.
const TURN_END_SUCCESS: &str = "success";

pub struct PlannerTurn;

/// What `PlannerTurn::stop` did, because its callers need to tell these
/// apart: the Stop route answers differently when the tree could not be
/// terminated, and shutdown (§8.5) must not count a live tree as stopped.
/// `Ok(())` for all five was how shutdown came to record `Graceful` over a
/// turn whose termination had failed.
///
/// Only `Cancelled` means `stop` itself wrote the terminal state. In the three
/// middle cases the reader writes it (or already has), which is why a caller
/// that needs the operation terminal must still wait for the durable record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StopOutcome {
    /// §8.4 case 3: the tree was terminated and reaped, then `Cancelled` was
    /// written.
    Cancelled,
    /// §8.4 case 3 over a turn that had already produced its ending: the tree
    /// was terminated, and the reader reaps it and records that ending.
    TerminatedAfterTurnEnd,
    /// §8.4 case 4: the process had already exited on its own. Nothing was
    /// terminated; the reader records the real exit.
    AlreadyExited,
    /// This runtime holds no live tree for the operation: the reader already
    /// claimed it, or it was never registered here. Nothing was terminated.
    NotLive,
    /// §8.4 case 6: termination failed. The tree may still be running, the
    /// registration is kept, and the operation stays non-terminal.
    TerminationFailed,
}

/// Everything the watcher needs to decide and persist how one live turn ends,
/// bundled because it is one thing — the turn — and not six parameters.
pub(crate) struct TurnWatch {
    pub(crate) op_id: OperationId,
    pub(crate) runtime: Arc<Runtime>,
    pub(crate) handles: Arc<LiveHandles>,
    pub(crate) harness: Arc<ClaudeHarness>,
    pub(crate) thread_id: ThreadId,
    pub(crate) agent_role: String,
    /// The session this turn started, when it started one. Recorded on the
    /// thread at the turn-end, the harness's statement that the turn — and
    /// so its session — exists in the harness's store. Earlier would be
    /// wrong: a spawn failure, crash, or Stop before then can leave no
    /// session, and a recorded id `--resume` rejects would fail every later
    /// turn on the thread. The price is that a first turn stopped mid-stream
    /// is forgotten by the model; the thread's entries still show it.
    pub(crate) new_session: Option<String>,
    /// The same flag the registration holds. The watcher sets it the moment it
    /// classifies this turn's `TurnEnd`; `stop` reads it to decide who names
    /// the ending. Shared rather than re-read from the map because the watcher
    /// must publish the fact without taking the map's lock on every line.
    pub(crate) turn_end_seen: Arc<AtomicBool>,
    pub(crate) span: tracing::Span,
}

/// Reads one turn's stream to its end, then names and persists that ending.
/// Returns as soon as the background task is spawned.
///
/// This is one half of §8.4's arbitration and `PlannerTurn::stop` is the
/// other, which is why they share a file: the rule that decides which of them
/// writes the terminal transition is only reviewable with both in view.
pub(crate) fn watch_turn(
    watch: TurnWatch,
    lines: Option<StdoutLines>,
    bus: broadcast::Sender<(ThreadId, OperationId, StreamItem)>,
) {
    let TurnWatch {
        op_id: reader_op,
        runtime: reader_runtime,
        handles: reader_handles,
        harness: reader_harness,
        thread_id: reader_thread,
        agent_role,
        mut new_session,
        turn_end_seen,
        span,
    } = watch;
    tokio::spawn(async move {
        let mut turn_end: Option<serde_json::Value> = None;
        let mut first_output = true;
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
                if std::mem::take(&mut first_output) {
                    tracing::info!(bytes = line.len(), "planner.first_output");
                }
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
                        tracing::info!(%subtype, "planner.turn_end");
                        if let Some(session) = new_session.take()
                            && let Err(error) = reader_runtime
                                .storage
                                .record_harness_session(&reader_thread, &session)
                                .await
                        {
                            // The turn itself is unaffected; the thread's
                            // next turn starts a new session instead.
                            tracing::error!(%error, "planner.record_session_failed");
                        }
                    }
                    _ => {}
                }
                let _ = bus.send((reader_thread.clone(), reader_op.clone(), item));
            }
        }

        // The stream ended — either the process exited on its own, or
        // `stop` killed it and the kill also closed stdout, which looks
        // identical from here. Which side writes the terminal state is
        // decided by the registration, not by racing two storage CASes:
        // whoever's `remove` returns `Some` owns the outcome. `None` means
        // `stop` took ownership and will write `Cancelled`, so this task
        // writes nothing (§8.4 case 5).
        let claimed = reader_handles.claim(&reader_op).await;
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
        let ending = match &status {
            Ok(code) => format!("{code}"),
            Err(error) => format!("exit status unreadable: {error}"),
        };
        // The exit status says something about the turn only when the turn
        // is what ended it. If `stop` terminated this tree after the turn
        // had already reported its result (§8.4 case 3 over a finished
        // turn), the status reports our own kill, and reading it as the
        // turn's verdict would record a failure we caused.
        let exit_is_the_turns =
            turn.terminated_by_stop || matches!(&status, Ok(code) if code.success());

        let written = match turn_end {
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
            Some(outcome) => {
                // Two verdicts have to agree before this is a success: the
                // harness's own and the process's. The evidence report
                // (`docs/evidence/harness/SERVE_STREAM_SPIKE.md`) measured
                // exactly one subtype, `success`, and says the turn end
                // carries "a structured verdict and a process status, and
                // can cross-check them" — so the rule is that `success` is
                // the one subtype measured to mean success, not a guessed
                // blacklist of the failing ones.
                let verdict = outcome
                    .get("subtype")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or_default()
                    .to_string();
                if verdict != TURN_END_SUCCESS {
                    reader_runtime
                        .storage
                        .mark_operation_failed(
                            &reader_op,
                            FailureStage::Run,
                            &format!("the harness reported a failing turn end: {verdict}"),
                        )
                        .await
                } else if !exit_is_the_turns {
                    reader_runtime
                        .storage
                        .mark_operation_failed(
                            &reader_op,
                            FailureStage::Run,
                            &format!("the harness reported success, then {ending}"),
                        )
                        .await
                } else {
                    reader_runtime
                        .storage
                        .mark_operation_completed(&reader_op, outcome)
                        .await
                }
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
    }.instrument(span));
}

impl PlannerTurn {
    /// Spec §2.3: request, terminate, confirm, then write Cancelled. If
    /// termination cannot be confirmed — no live handle, or the tree refuses
    /// to terminate — Cancelled is not written and the operation is left for
    /// recovery (spec §8.4 case 6).
    ///
    /// Terminating and naming the ending are two decisions, taken in that
    /// order: anything still alive is terminated (§8.4 case 3), and only then
    /// does the question of who writes the terminal state arise. The branches
    /// below name the case each one serves, and each answers its own
    /// [`StopOutcome`].
    ///
    /// `requester` is whoever asked: the user through the Stop route, or the
    /// runtime itself at shutdown (§8.5). It is recorded on the request.
    pub async fn stop(
        runtime: Arc<Runtime>,
        handles: Arc<LiveHandles>,
        op_id: &OperationId,
        requester: Actor,
    ) -> Result<StopOutcome, StorageError> {
        runtime
            .storage
            .request_cancellation(op_id, requester)
            .await?;

        let mut registry = handles.0.lock().await;
        let map = &mut registry.turns;
        let Some(mut turn) = map.remove(op_id) else {
            // No live handle: either it already exited (the reader task has
            // or will record its real outcome), or this runtime never
            // registered one for it. Either way termination cannot be
            // confirmed here, so Cancelled must not be written.
            tracing::info!(operation_id = %op_id, "planner.stop: no live handle, nothing to terminate");
            return Ok(StopOutcome::NotLive);
        };
        let span = turn.span.clone();
        tracing::info!(parent: &span, "planner.stop");

        // §8.4 case 4. The process exited between the request and this lock,
        // so there is no tree to terminate and nothing Shadows can claim to
        // have stopped. The registration goes back: the reader owns the
        // outcome and persists the real exit. Note what this branch is NOT
        // asking — whether the stream said the turn was finishing. A harness
        // that reported its result and is still alive is a live tree, and
        // case 3 below is what it gets.
        if turn.handle.has_exited() {
            tracing::info!(parent: &span, "planner.stop: already exited; the reader names the ending");
            map.insert(op_id.clone(), turn);
            return Ok(StopOutcome::AlreadyExited);
        }

        // §8.4 case 6. Ownership of the outcome is claimed only once
        // termination is actually under way. Keeping the registration here
        // would drop the one handle that can still reach the tree — the
        // process would keep running, unterminable, and no one would ever
        // resolve the operation.
        if span.in_scope(|| turn.handle.terminate_tree()).is_err() {
            map.insert(op_id.clone(), turn);
            return Ok(StopOutcome::TerminationFailed);
        }

        // §8.4 case 3 has now been served: the registered tree is terminated.
        // What remains is who gets to name the ending, and that is the one
        // question `turn_end_seen` answers. The turn produced its true ending
        // before cancellation took termination ownership, so that ending wins
        // — Shadows does not relabel a finished turn as one it stopped. The
        // registration goes back carrying the fact that the exit about to be
        // observed is our kill, so the reader records the outcome it already
        // holds instead of reading that kill as a failing exit.
        if turn.turn_end_seen.load(Ordering::SeqCst) {
            turn.terminated_by_stop = true;
            map.insert(op_id.clone(), turn);
            return Ok(StopOutcome::TerminatedAfterTurnEnd);
        }
        drop(registry);

        // `wait` returning is the confirmation that the leader (and, through
        // job/group containment, the tree it owns) has been reaped. The exit
        // status itself is discarded deliberately: we just killed this
        // process ourselves, so its status reports the kill, not an outcome
        // of the turn, and Cancelled records that the operation was stopped,
        // not what its exit code was.
        let _ = turn.handle.wait().instrument(span.clone()).await;
        tracing::info!(parent: &span, "planner.stop: tree reaped");
        runtime.storage.mark_operation_cancelled(op_id).await?;
        Ok(StopOutcome::Cancelled)
    }
}
