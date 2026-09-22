//! One job: turn a Planner turn request into a registered, running operation.
//!
//! Spec §2.7's two-phase spawn, in order: the durable attempt is committed
//! (`Pending`) before anything is spawned, the child is registered in
//! `LiveHandles` before `Running` is committed (§8.3 — a committed transition
//! with nothing registered would leave `stop` with nothing to terminate), and
//! only then is the turn handed to the watcher in this module's parent, which
//! owns every question about how it ends.
//!
//! Failures here are named by the stage that produced them, because they are
//! not the same fact: `Prepare` means no process ever existed, `Spawn` means
//! the operating system refused to start one.

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use tokio::sync::broadcast;
use tracing::Instrument;

use super::{LiveHandles, LiveTurn, PlannerTurn, TurnWatch, watch_turn};
use crate::agent::claude::ClaudeHarness;
use crate::agent::{AgentHarness, AgentInvocation, StreamItem};
use crate::operation::{FailureStage, OperationId};
use crate::process::spawn;
use crate::runtime::Runtime;
use crate::storage::StorageError;
use crate::thread::ThreadId;

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

impl PlannerTurn {
    /// Returns the operation id as soon as Pending is committed. The turn
    /// keeps running in a background task; the caller subscribes to `bus`
    /// separately for the stream.
    pub async fn start(
        runtime: Arc<Runtime>,
        handles: Arc<LiveHandles>,
        harness: Arc<ClaudeHarness>,
        request: PlannerTurnRequest,
        bus: broadcast::Sender<(ThreadId, OperationId, StreamItem)>,
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
        // Spec §8.7's correlation fields, carried by every line this turn logs
        // — here, in the watcher, in `stop`, and in `process/` beneath them.
        let span =
            tracing::info_span!("planner.turn", operation_id = %op_id, thread_id = %thread_id);

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
        // The prompt's length, never its text (spec §8.7).
        tracing::info!(
            parent: &span,
            prompt_len = invocation.prompt.len(),
            resume = invocation.resume_session_id.is_some(),
            "agent.invocation.start"
        );
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

        let mut handle = match span.in_scope(|| spawn(spec)) {
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
                    terminated_by_stop: false,
                    span: span.clone(),
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
                let _ = span.in_scope(|| turn.handle.terminate_tree());
                let _ = turn.handle.wait().instrument(span).await;
            }
            return Err(error);
        }

        watch_turn(
            TurnWatch {
                op_id: op_id.clone(),
                runtime,
                handles,
                harness,
                thread_id,
                agent_role: invocation.role,
                turn_end_seen,
                span,
            },
            lines,
            bus,
        );

        Ok(op_id)
    }
}
