//! Shared apparatus: starting a Planner turn without HTTP, the way the turn
//! route does (spec §12.7) — the operation is committed by
//! `Storage::start_turn`, then `PlannerTurn::start` runs it.

#![allow(dead_code)]

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use shadows_agent::TurnSettings;
use shadows_agent::events::HarnessEvent;
use shadows_core::OperationId;
use shadows_core::StartError;
use shadows_core::command::{CommandContext, fingerprint};
use shadows_core::harness::Sessions;
use shadows_core::runtime::Runtime;
use shadows_core::testing::NewTurn;
use shadows_core::testing::{LiveHandles, PlannerTurn, PlannerTurnRequest};
use shadows_core::threads::ThreadId;

/// The fake's own settings, which every turn can run with.
pub fn default_turn_settings() -> TurnSettings {
    TurnSettings {
        model: "fake-large".into(),
        mode: "acceptEdits".into(),
        effort: Some("high".into()),
    }
}

static NEXT: AtomicUsize = AtomicUsize::new(1);

pub fn turn_command(id: &str, thread: &ThreadId, prompt: &str, s: &TurnSettings) -> CommandContext {
    let params = serde_json::json!({
        "thread_id": thread, "prompt": prompt, "model": s.model, "mode": s.mode, "effort": s.effort,
    });
    CommandContext {
        principal_kind: "User".into(),
        principal_id: "local".into(),
        command_id: id.into(),
        command_kind: "turn.start".into(),
        command_schema_ver: 1,
        request_fingerprint: fingerprint("turn.start", &params),
    }
}

pub fn new_turn<'a>(
    thread: &'a ThreadId,
    runtime: &'a Runtime,
    prompt: &'a str,
    settings: &'a TurnSettings,
) -> NewTurn<'a> {
    NewTurn {
        thread_id: thread,
        runtime: &runtime.instance_id,
        prompt,
        role: "Planner",
        harness_kind: "claude-code",
        harness_path: "fake-adapter/dist/index.js",
        harness_version: "fake-adapter-1",
        agent_path: "fake_acp",
        agent_version: "fake-claude-1",
        settings,
        prompt_version: Some(shadows_core::harness::prompt_version()),
        instructions_version: None,
        focus: None,
    }
}

/// Opens the thread's session, commits the turn, and starts it with the
/// fake's default settings. A stopped runtime answers `RuntimeStopping`.
pub async fn start_direct(
    runtime: &Arc<Runtime>,
    handles: &Arc<LiveHandles>,
    sessions: &Arc<Sessions>,
    bus: &tokio::sync::broadcast::Sender<(ThreadId, OperationId, HarnessEvent)>,
    thread: &ThreadId,
    prompt: &str,
) -> Result<OperationId, StartError> {
    let opened = sessions.open(thread).await.unwrap();
    let events = sessions.lease_events(thread, &opened).await.unwrap();
    let settings = default_turn_settings();
    let id = format!("direct-{}", NEXT.fetch_add(1, Ordering::SeqCst));
    let command = turn_command(&id, thread, prompt, &settings);
    let started = match runtime
        .storage
        .start_turn(&command, new_turn(thread, runtime, prompt, &settings))
        .await
    {
        Ok(started) => started,
        Err(e) => {
            sessions.give_back_events(thread, &opened, events).await;
            if handles.is_closed().await {
                return Err(StartError::RuntimeStopping);
            }
            return Err(e.into());
        }
    };
    PlannerTurn::start(
        runtime.clone(),
        handles.clone(),
        sessions.clone(),
        opened,
        PlannerTurnRequest {
            thread_id: thread.clone(),
            harness: "claude-code".into(),
            operation_id: started.operation_id,
            prompt: prompt.into(),
            settings,
            focus: None,
            client_tab: None,
            events,
        },
        bus.clone(),
    )
    .await
}
