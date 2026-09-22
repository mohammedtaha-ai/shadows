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

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use tokio::sync::{Mutex, broadcast};

use crate::agent::claude::ClaudeHarness;
use crate::agent::{AgentHarness, AgentInvocation, StreamItem};
use crate::events::Actor;
use crate::operation::{FailureStage, OperationId};
use crate::process::{ProcessHandle, spawn};
use crate::runtime::Runtime;
use crate::storage::StorageError;
use crate::thread::{NewThreadEntry, ThreadId};

/// Live handles for operations this runtime owns. Spec §8.3: termination goes
/// through the containment handle that owns the tree, never through a PID read
/// from the database, because the OS reuses PIDs.
///
/// The inner map is `pub(crate)` so `cli::serve` can enumerate this runtime's
/// live operations at shutdown.
#[derive(Default)]
pub struct LiveHandles(pub(crate) Mutex<HashMap<OperationId, ProcessHandle>>);

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
            operation_id: op_id.as_str().to_string(),
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
        {
            let mut map = handles.0.lock().await;
            map.insert(op_id.clone(), handle);
        }

        // TX #2.
        runtime
            .storage
            .mark_operation_started(&op_id, &runtime.instance_id)
            .await?;

        let reader_op = op_id.clone();
        let reader_runtime = runtime.clone();
        let reader_handles = handles.clone();
        let reader_harness = harness.clone();
        let reader_thread = thread_id.clone();
        tokio::spawn(async move {
            let mut outcome = serde_json::json!({ "stop_reason": null });
            if let Some(mut lines) = lines {
                while let Ok(Some(line)) = lines.next_line().await {
                    let item = reader_harness.classify(&line);
                    match &item {
                        // Durable: write before forwarding. The UI may drop a
                        // frame; the record may not.
                        StreamItem::Entry { uuid, role, text } => {
                            if let Err(error) = reader_runtime
                                .storage
                                .append_thread_entry(
                                    &reader_thread,
                                    NewThreadEntry {
                                        kind: "AgentMessage",
                                        author: Actor {
                                            kind: role.clone(),
                                            id: uuid.clone(),
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
                            outcome = serde_json::json!({
                                "subtype": subtype, "stop_reason": stop_reason
                            });
                        }
                        _ => {}
                    }
                    let _ = bus.send((reader_op.clone(), item));
                }
            }

            // The stream ended — either the process exited on its own, or
            // `stop` killed it and the kill also closed stdout, which looks
            // identical from here. Whether THIS task or `stop` gets to decide
            // the terminal state is not settled by racing two independent
            // storage CASes (measured: that race is real and non-deterministic
            // — `stop`'s confirmation loop and this task's EOF do not resolve
            // in a fixed order), so the live-handle map is the single
            // arbitration point instead: removing the entry is what grants
            // ownership of the outcome, and whoever's `remove` returns `Some`
            // is the one allowed to write a terminal transition.
            //
            // If `stop` already removed it, this `remove` returns `None`: a
            // cancellation is in flight or already confirmed, so this task
            // must not also claim Completed (spec §8.4 case 4/5 — Shadows
            // does not claim two different endings for the same operation).
            // The dropped `ProcessHandle` on the `Some` arm is the intent,
            // not an oversight: dropping it releases its containment wrapper,
            // which is correct once the process is already gone.
            let owns_outcome = reader_handles.0.lock().await.remove(&reader_op).is_some();
            if owns_outcome {
                let _ = reader_runtime
                    .storage
                    .mark_operation_completed(&reader_op, outcome)
                    .await;
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
        let Some(mut handle) = map.remove(op_id) else {
            // No live handle: either it already exited (the reader task has
            // or will remove it and record Completed/Failed), or this runtime
            // never registered one for it. Either way termination cannot be
            // confirmed here, so Cancelled must not be written.
            return Ok(());
        };
        drop(map);

        if handle.terminate_tree().is_err() {
            return Ok(()); // Unconfirmed; left non-terminal on purpose.
        }
        // `wait` returning is the confirmation that the leader (and, through
        // job/group containment, the tree it owns) has been reaped. The exit
        // status itself is discarded deliberately: we just killed this
        // process ourselves, so its status reports the kill, not an outcome
        // of the turn, and Cancelled records that the operation was stopped,
        // not what its exit code was.
        let _ = handle.wait().await;
        runtime.storage.mark_operation_cancelled(op_id).await
    }
}
