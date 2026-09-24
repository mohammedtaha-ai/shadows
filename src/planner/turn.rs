//! One job: decide and persist how a live Planner turn ends.
//!
//! The watcher and Stop compete for one registration. A harness `cancelled`
//! answer confirms Stop; otherwise Stop terminates and reaps the adapter tree
//! before recording `Cancelled`. If the watcher claimed the registration first,
//! its prompt result wins. If termination fails, Stop restores the registration
//! and leaves the operation nonterminal for recovery (§2.3, §8.4, §12.3).

use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use tokio::{sync::{broadcast, mpsc}, time::{sleep, Instant}};
use tracing::Instrument;
use crate::{agent::{acp::{AcpError, TurnEnd}, events::HarnessEvent}, events::Actor, operation::{FailureStage, OperationId}, runtime::Runtime, storage::StorageError, thread::{NewThreadEntry, ThreadId}};
use super::{entries::{Collector, Durable}, LiveHandles, OpenSession, Sessions};

pub struct PlannerTurn;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StopOutcome { Cancelled, ResolvedByTurn, NotLive, TerminationFailed }

pub(crate) struct TurnWatch {
    pub op_id: OperationId,
    pub runtime: Arc<Runtime>,
    pub handles: Arc<LiveHandles>,
    pub sessions: Arc<Sessions>,
    pub opened: OpenSession,
    pub thread_id: ThreadId,
    pub prompt: String,
    pub events: Option<mpsc::UnboundedReceiver<HarnessEvent>>,
    pub turn_end_seen: Arc<AtomicBool>,
    pub cancel_requested: Arc<AtomicBool>,
    pub span: tracing::Span,
}

async fn persist(w: &TurnWatch, entries: Vec<Durable>) {
    for entry in entries {
        let (kind, author, body) = match entry {
            Durable::Message(body) => ("AgentMessage", Actor { kind: "Agent".into(), id: "Planner".into() }, body),
            Durable::Tool(title) => ("AgentMessage", Actor { kind: "Agent".into(), id: "Planner".into() }, format!("[tool: {title}]")),
            Durable::PermissionRefused(body) => ("PermissionRefused", Actor::system(), body),
        };
        if let Err(error) = w.runtime.storage.append_thread_entry(&w.thread_id, NewThreadEntry { kind, author, body: &body, refs: &[] }).await {
            tracing::error!(%error, "planner.append_thread_entry_failed");
        }
    }
}

async fn accept(w: &TurnWatch, collector: &mut Collector, bus: &broadcast::Sender<(ThreadId, OperationId, HarnessEvent)>, e: HarnessEvent) {
    let durable = collector.push(&e);
    persist(w, durable).await;
    let _ = bus.send((w.thread_id.clone(), w.op_id.clone(), e));
}

pub(crate) fn watch_turn(mut w: TurnWatch, bus: broadcast::Sender<(ThreadId, OperationId, HarnessEvent)>) {
    let span = w.span.clone();
    tokio::spawn(async move {
        let mut collector = Collector::new();
        // Notifications between turns belong to neither prompt.
        let mut events = w.events.take();
        if let Some(rx) = events.as_mut() {
            while let Ok(stale) = rx.try_recv() { tracing::trace!(?stale, "planner.stale_event"); }
        }
        let connection = w.opened.connection().clone();
        let session_id = w.opened.session_id.clone();
        let prompt = w.prompt.clone();
        let answer = {
            let prompt_future = connection.prompt(&session_id, &prompt);
            tokio::pin!(prompt_future);
            loop {
                if let Some(rx) = events.as_mut() {
                    tokio::select! {
                        result = &mut prompt_future => break result,
                        event = rx.recv() => if let Some(event) = event { accept(&w, &mut collector, &bus, event).await; },
                    }
                } else { break prompt_future.await; }
            }
        };
        if let Some(mut rx) = events.take() {
            while let Ok(event) = rx.try_recv() { accept(&w, &mut collector, &bus, event).await; }
            persist(&w, collector.finish()).await;
            w.sessions.give_back_events(&w.thread_id, rx).await;
        } else { persist(&w, collector.finish()).await; }
        w.turn_end_seen.store(true, Ordering::SeqCst);
        if answer.is_ok() && w.opened.how == "new" {
            if let Err(error) = w.runtime.storage.record_harness_session(&w.thread_id, &w.opened.session_id).await {
                tracing::error!(%error, "planner.record_session_failed");
            }
        }
        w.sessions.touch(&w.thread_id).await;
        if w.handles.claim(&w.op_id).await.is_none() { return; }
        let result = match answer {
            Ok(TurnEnd::Ended) => w.runtime.storage.mark_operation_completed(&w.op_id, serde_json::json!({"stop_reason":"end_turn"})).await,
            Ok(TurnEnd::Cancelled) if w.cancel_requested.load(Ordering::SeqCst) => w.runtime.storage.mark_operation_cancelled(&w.op_id).await,
            Ok(TurnEnd::Cancelled) => w.runtime.storage.mark_operation_failed(&w.op_id, FailureStage::Run, "the harness cancelled the turn on its own").await,
            Ok(TurnEnd::Refused(reason)) => w.runtime.storage.mark_operation_failed(&w.op_id, FailureStage::Run, &reason).await,
            Err(AcpError::Closed) => {
                if let Err(error) = w.sessions.terminate(&w.thread_id).await { tracing::error!(%error, "planner.dead_adapter_cleanup_failed"); }
                w.runtime.storage.mark_operation_failed(&w.op_id, FailureStage::Run, "the harness exited during the turn").await
            }
            Err(AcpError::Rpc(reason)) => w.runtime.storage.mark_operation_failed(&w.op_id, FailureStage::Run, &reason).await,
        };
        if let Err(error) = result { tracing::error!(%error, "planner.terminal_transition_failed"); }
    }.instrument(span));
}

impl PlannerTurn {
    pub async fn stop(runtime: Arc<Runtime>, handles: Arc<LiveHandles>, sessions: Arc<Sessions>, op_id: &OperationId, requester: Actor) -> Result<StopOutcome, StorageError> {
        runtime.storage.request_cancellation(op_id, requester).await?;
        let Some((thread_id, flag)) = ({
            let r = handles.0.lock().await;
            r.turns.get(op_id).map(|t| (t.thread_id.clone(), t.cancel_requested.clone()))
        }) else { return Ok(StopOutcome::NotLive); };
        flag.store(true, Ordering::SeqCst);
        sessions.cancel(&thread_id).await;
        let deadline = Instant::now() + sessions.cancel_wait();
        while Instant::now() < deadline {
            if !handles.contains_internal(op_id).await { return Ok(StopOutcome::ResolvedByTurn); }
            sleep(std::time::Duration::from_millis(10)).await;
        }
        let Some(turn) = handles.claim(op_id).await else { return Ok(StopOutcome::ResolvedByTurn); };
        if let Err(error) = sessions.terminate(&thread_id).await {
            tracing::error!(parent: &turn.span, %error, "planner.stop_termination_failed");
            handles.restore(op_id.clone(), turn).await;
            return Ok(StopOutcome::TerminationFailed);
        }
        runtime.storage.mark_operation_cancelled(op_id).await?;
        Ok(StopOutcome::Cancelled)
    }
}
