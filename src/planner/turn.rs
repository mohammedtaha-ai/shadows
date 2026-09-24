//! One job: the recorded ending of a live Planner turn (spec §2.3, §8.4, §12.3).
//!
//! Starting one is the neighbouring file's job. `spawn.rs` turns a request
//! into a registered, running operation and hands it here; everything after
//! that — the updates, the ending, the cancellation interlock — is this file.
//! `handles.rs` is the registry of live turns, `sessions.rs` owns the adapter
//! the prompt runs on, and `shutdown.rs` stops every turn when the runtime
//! stops (§8.5).
//!
//! `Cancelled` means Shadows confirmed the managed execution is no longer
//! running AND persisted the terminal transition (spec §2.3) — never only
//! that a user asked for it. There are two confirmations. The harness answering
//! the prompt `cancelled` after `session/cancel` is one: the turn stopped, the
//! adapter lives on for the next turn, and the watcher writes `Cancelled`. If
//! it does not answer within `cancel_wait`, `stop` terminates the adapter's
//! tree, confirms it is gone, and only then writes `Cancelled` itself. If
//! termination cannot be confirmed, nothing terminal is written; the operation
//! is left for a later runtime's recovery (`Storage::reconcile_orphans`, which
//! writes `Interrupted`, never `Cancelled`).
//!
//! **Why the watcher and `stop` live in one file.** They are the two sides of
//! one arbitration. §8.4 requires that exactly one of them writes the terminal
//! transition, and the rule that decides which — claiming the registration in
//! `LiveHandles`, plus the two facts `LiveTurn` carries — is only reviewable if
//! both sides are read together.
//!
//! | Milestone 0 (a process per turn) | Milestone 1 (a prompt on a live connection) |
//! |---|---|
//! | spawn `claude --print` | the thread's adapter is open (§12.2), then `session/prompt` is sent |
//! | handle registered, then `Running` | the prompt is registered in `LiveHandles`, then `Running` |
//! | `stream_event` text deltas | `session/update` chunks: transient, rendered, never stored |
//! | `assistant` / `user` lines → entries | one entry per agent message, and one per tool call under its last title (`entries.rs`) |
//! | `result` line + exit status | the `session/prompt` response: `end_turn` is success; any other stop reason, or a JSON-RPC error, fails the turn naming it |
//! | session recorded at turn-end | unchanged: recorded when the first turn ends, never earlier |
//! | the adapter exits mid-turn | `Failed { stage: Run }`, "the harness exited during the turn" |

use super::{
    LiveHandles, OpenSession, Sessions,
    entries::{Collector, Durable},
};
use crate::{
    agent::{
        acp::{AcpError, TurnEnd},
        events::{HarnessEvent, TurnObservation, limits_from},
    },
    events::Actor,
    operation::{FailureStage, OperationId},
    runtime::Runtime,
    storage::StorageError,
    thread::{NewThreadEntry, ThreadId},
};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use tokio::{
    sync::{broadcast, mpsc},
    time::{Instant, sleep},
};
use tracing::Instrument;

pub struct PlannerTurn;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StopOutcome {
    Cancelled,
    ResolvedByTurn,
    NotLive,
    TerminationFailed,
}

pub(crate) struct TurnWatch {
    pub op_id: OperationId,
    pub runtime: Arc<Runtime>,
    pub handles: Arc<LiveHandles>,
    pub sessions: Arc<Sessions>,
    pub opened: OpenSession,
    pub thread_id: ThreadId,
    /// The thread's harness, whose account limits a usage report updates.
    pub harness: String,
    pub prompt: String,
    pub turn_end_seen: Arc<AtomicBool>,
    pub cancel_requested: Arc<AtomicBool>,
    pub span: tracing::Span,
}

async fn persist(w: &TurnWatch, entries: Vec<Durable>) {
    for entry in entries {
        let (kind, author, body) = match entry {
            Durable::Message(body) => (
                "AgentMessage",
                Actor {
                    kind: "Agent".into(),
                    id: "Planner".into(),
                },
                body,
            ),
            Durable::Tool(title) => (
                "AgentMessage",
                Actor {
                    kind: "Agent".into(),
                    id: "Planner".into(),
                },
                format!("[tool: {title}]"),
            ),
            Durable::PermissionRefused(body) => ("PermissionRefused", Actor::system(), body),
        };
        if let Err(error) = w
            .runtime
            .storage
            .append_thread_entry(
                &w.thread_id,
                NewThreadEntry {
                    kind,
                    author,
                    body: &body,
                    refs: &[],
                    operation_id: Some(&w.op_id),
                },
            )
            .await
        {
            tracing::error!(%error, "planner.append_thread_entry_failed");
        }
    }
}

async fn accept(
    w: &TurnWatch,
    collector: &mut Collector,
    bus: &broadcast::Sender<(ThreadId, OperationId, HarnessEvent)>,
    first: &mut bool,
    last_usage: &mut Option<HarnessEvent>,
    e: HarnessEvent,
) {
    if *first && matches!(e, HarnessEvent::Chunk { .. }) {
        *first = false;
        tracing::info!("planner.first_output");
    }
    if let HarnessEvent::Usage { rate_limit, .. } = &e {
        // Recorded before the event is published, so the `usage` frame it
        // becomes carries these limits as the latest (§12.8).
        let limits = rate_limit
            .as_ref()
            .and_then(|r| limits_from(r, &now_rfc3339()));
        if let Some(limits) = limits
            && let Err(error) = w.runtime.storage.record_limits(&w.harness, &limits).await
        {
            tracing::error!(%error, "planner.record_limits_failed");
        }
        *last_usage = Some(e.clone());
    }
    let durable = collector.push(&e);
    persist(w, durable).await;
    let _ = bus.send((w.thread_id.clone(), w.op_id.clone(), e));
}

pub(crate) fn watch_turn(
    w: TurnWatch,
    mut rx: mpsc::UnboundedReceiver<HarnessEvent>,
    bus: broadcast::Sender<(ThreadId, OperationId, HarnessEvent)>,
) {
    let span = w.span.clone();
    tokio::spawn(async move {
        let mut collector = Collector::new();
        let mut first = true;
        let mut last_usage = None;
        tracing::info!(session_id = %w.opened.session_id, how = w.opened.how, "agent.invocation.start");
        // Notifications between turns belong to neither prompt.
        while let Ok(stale) = rx.try_recv() { tracing::trace!(?stale, "planner.stale_event"); }
        let connection = w.opened.connection().clone();
        let session_id = w.opened.session_id.clone();
        let prompt = w.prompt.clone();
        let answer = {
            let prompt_future = connection.prompt(&session_id, &prompt);
            tokio::pin!(prompt_future);
            loop {
                tokio::select! {
                    result = &mut prompt_future => break result,
                    event = rx.recv() => match event {
                        Some(event) => accept(&w, &mut collector, &bus, &mut first, &mut last_usage, event).await,
                        // The adapter dropped its side: nothing more will
                        // arrive, and polling a closed channel would spin.
                        None => break prompt_future.await,
                    },
                }
            }
        };
        while let Ok(event) = rx.try_recv() { accept(&w, &mut collector, &bus, &mut first, &mut last_usage, event).await; }
        persist(&w, collector.finish()).await;
        w.sessions.give_back_events(&w.thread_id, &w.opened, rx).await;
        w.turn_end_seen.store(true, Ordering::SeqCst);
        let (subtype, stop_reason) = match &answer {
            Ok(TurnEnd::Ended) => ("success", Some("end_turn".to_string())),
            Ok(TurnEnd::Cancelled) => ("error", Some("cancelled".to_string())),
            Ok(TurnEnd::Refused(reason)) => ("error", Some(reason.clone())),
            Err(_) => ("error", None),
        };
        tracing::info!(subtype, stop_reason = stop_reason.as_deref().unwrap_or(""), "planner.turn_end");
        let _ = bus.send((w.thread_id.clone(), w.op_id.clone(), HarnessEvent::TurnEnd { subtype, stop_reason }));
        // A new session, or a fork's, is recorded when its first turn ends
        // (§12.3, §12.9); a resumed one is already the thread's.
        if answer.is_ok()
            && w.opened.how != "resume"
            && let Err(error) = w.runtime.storage.record_harness_session(&w.thread_id, &w.opened.session_id).await
        {
            tracing::error!(%error, "planner.record_session_failed");
        }
        if matches!(answer, Ok(TurnEnd::Ended)) {
            w.sessions.mark_answered(&w.thread_id, &w.opened).await;
        }
        w.sessions.touch(&w.thread_id).await;
        if w.handles.claim(&w.op_id).await.is_none() { return; }
        let result = match answer {
            // Only a completed turn records an observation: a failed or
            // cancelled one leaves it NULL, which a client shows as
            // unavailable (§12.7 — nothing unreported is estimated).
            Ok(TurnEnd::Ended) => {
                let seen = TurnObservation {
                    native_session_id: Some(w.opened.session_id.clone()),
                    ..TurnObservation::from_usage(last_usage.as_ref())
                };
                w.runtime.storage.mark_operation_completed(&w.op_id, serde_json::json!({"stop_reason":"end_turn"}), &seen).await
            }
            Ok(TurnEnd::Cancelled) if w.cancel_requested.load(Ordering::SeqCst) => w.runtime.storage.mark_operation_cancelled(&w.op_id).await,
            Ok(TurnEnd::Cancelled) => w.runtime.storage.mark_operation_failed(&w.op_id, FailureStage::Run, "the harness cancelled the turn on its own").await,
            Ok(TurnEnd::Refused(reason)) => w.runtime.storage.mark_operation_failed(&w.op_id, FailureStage::Run, &reason).await,
            Err(AcpError::Closed) => {
                if let Err(error) = w.sessions.terminate_adapter(&w.thread_id, &w.opened).await { tracing::error!(%error, "planner.dead_adapter_cleanup_failed"); }
                w.runtime.storage.mark_operation_failed(&w.op_id, FailureStage::Run, "the harness exited during the turn").await
            }
            Err(AcpError::Rpc(reason)) => w.runtime.storage.mark_operation_failed(&w.op_id, FailureStage::Run, &reason).await,
        };
        if let Err(error) = result { tracing::error!(%error, "planner.terminal_transition_failed"); }
    }.instrument(span));
}

impl PlannerTurn {
    pub async fn stop(
        runtime: Arc<Runtime>,
        handles: Arc<LiveHandles>,
        sessions: Arc<Sessions>,
        op_id: &OperationId,
        requester: Actor,
    ) -> Result<StopOutcome, StorageError> {
        runtime
            .storage
            .request_cancellation(op_id, requester)
            .await?;
        let Some((thread_id, session, flag, ended, span)) = ({
            let r = handles.0.lock().await;
            r.turns.get(op_id).map(|t| {
                (
                    t.thread_id.clone(),
                    t.session.clone(),
                    t.cancel_requested.clone(),
                    t.turn_end_seen.clone(),
                    t.span.clone(),
                )
            })
        }) else {
            tracing::info!(operation_id = %op_id, "planner.stop: not live, nothing to cancel");
            return Ok(StopOutcome::NotLive);
        };
        tracing::info!(parent: &span, "planner.stop");
        flag.store(true, Ordering::SeqCst);
        session.connection().cancel(&session.session_id);
        let deadline = Instant::now() + sessions.cancel_wait();
        while Instant::now() < deadline {
            if !handles.contains_internal(op_id).await {
                tracing::info!(parent: &span, "planner.stop: the turn named its own ending");
                return Ok(StopOutcome::ResolvedByTurn);
            }
            sleep(std::time::Duration::from_millis(10)).await;
        }
        // The prompt has answered and the watcher is recording its ending:
        // that ending stands, and the adapter it ran on is still sound.
        if ended.load(Ordering::SeqCst) {
            return Ok(StopOutcome::ResolvedByTurn);
        }
        let Some(turn) = handles.claim(op_id).await else {
            return Ok(StopOutcome::ResolvedByTurn);
        };
        if let Err(error) = sessions.terminate_adapter(&thread_id, &session).await {
            tracing::error!(parent: &turn.span, %error, "planner.stop_termination_failed");
            handles.restore(op_id.clone(), turn).await;
            return Ok(StopOutcome::TerminationFailed);
        }
        tracing::info!(parent: &span, "planner.stop: the harness did not confirm; tree reaped");
        runtime.storage.mark_operation_cancelled(op_id).await?;
        Ok(StopOutcome::Cancelled)
    }
}

fn now_rfc3339() -> String {
    time::OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_default()
}
