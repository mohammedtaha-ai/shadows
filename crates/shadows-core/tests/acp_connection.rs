use std::{path::PathBuf, time::Duration};

use shadows_agent::{
    acp::{
        AcpError, Connection, PermissionError, PermissionPolicy, SessionSetup, SessionStart,
        TurnEnd,
    },
    claude::ClaudeAdapter,
    events::HarnessEvent,
};
use shadows_process::{self as process, ProcessHandle, ProcessSpec};
use tokio::sync::mpsc;

fn tmp() -> PathBuf {
    std::env::temp_dir()
}

async fn open_fake() -> (
    ProcessHandle,
    Connection,
    mpsc::UnboundedReceiver<HarnessEvent>,
) {
    open_with_policy(PermissionPolicy::Refuse).await
}

async fn open_with_policy(
    policy: PermissionPolicy,
) -> (
    ProcessHandle,
    Connection,
    mpsc::UnboundedReceiver<HarnessEvent>,
) {
    let executable = shadows_core::testing::fake_acp_path();
    let mut handle = process::spawn(ProcessSpec {
        executable,
        args: vec!["anything".into()],
        cwd: tmp(),
        env: vec![],
        capture_stdout: true,
        pipe_stdin: true,
    })
    .unwrap();
    let (tx, rx) = mpsc::unbounded_channel();
    let connection = Connection::open(&mut handle, policy, move |e| {
        let _ = tx.send(e);
    })
    .await
    .unwrap();
    (handle, connection, rx)
}

fn drain(rx: &mut mpsc::UnboundedReceiver<HarnessEvent>) -> Vec<HarnessEvent> {
    let mut out = Vec::new();
    while let Ok(event) = rx.try_recv() {
        out.push(event);
    }
    out
}

#[tokio::test]
async fn a_new_session_answers_its_options_and_a_prompt_streams_then_ends() {
    let (_h, c, mut ev) = open_fake().await;
    let opened = c
        .start_session(&tmp(), SessionStart::New, &SessionSetup::default())
        .await
        .unwrap();
    assert!(opened.session_id.starts_with("fake-"));
    assert!(opened.options.to_string().contains("fake-large"));
    assert_eq!(
        c.prompt(&opened.session_id, "hi", &[]).await.unwrap().end,
        TurnEnd::Ended
    );
    let events = drain(&mut ev);
    // The fake's `/` list is one of the events, wherever it falls.
    assert!(
        events
            .iter()
            .any(|e| matches!(e, HarnessEvent::Commands(list) if list.len() == 2))
    );
    let chunks: Vec<_> = events
        .iter()
        .filter(|e| matches!(e, HarnessEvent::Chunk { .. }))
        .collect();
    assert!(matches!(
        chunks[0],
        HarnessEvent::Chunk { message_id: Some(m), text } if m == "m1" && text == "hello "
    ));
}

#[tokio::test]
async fn a_cancelled_prompt_answers_cancelled() {
    let (_h, c, _ev) = open_fake().await;
    let s = c
        .start_session(&tmp(), SessionStart::New, &SessionSetup::default())
        .await
        .unwrap()
        .session_id;
    let c2 = c.clone();
    let s2 = s.clone();
    let turn = tokio::spawn(async move { c2.prompt(&s2, "hang", &[]).await });
    tokio::time::sleep(Duration::from_millis(200)).await;
    c.cancel(&s);
    assert_eq!(turn.await.unwrap().unwrap().end, TurnEnd::Cancelled);
}

#[tokio::test]
async fn a_dropped_prompt_frees_the_session() {
    let (_handle, connection, _events) = open_fake().await;
    let session = connection
        .start_session(&tmp(), SessionStart::New, &SessionSetup::default())
        .await
        .unwrap()
        .session_id;
    {
        let prompt = connection.prompt(&session, "hang", &[]);
        tokio::pin!(prompt);
        tokio::select! {
            biased;
            result = &mut prompt => panic!("prompt unexpectedly ended: {result:?}"),
            () = tokio::task::yield_now() => {}
        }
    }
    let result = tokio::time::timeout(
        Duration::from_secs(5),
        connection.prompt(&session, "hi", &[]),
    )
    .await
    .expect("the session accepts another prompt after its caller is dropped");
    assert_eq!(result.unwrap().end, TurnEnd::Ended);
}

#[tokio::test]
async fn a_permission_request_is_refused_and_reported() {
    let (_h, c, mut ev) = open_fake().await;
    let s = c
        .start_session(&tmp(), SessionStart::New, &SessionSetup::default())
        .await
        .unwrap()
        .session_id;
    c.prompt(&s, "ask-permission", &[]).await.unwrap();
    let got = drain(&mut ev);
    assert!(got.iter().any(
        |e| matches!(e, HarnessEvent::PermissionRefused { title, .. } if title == "Run echo probe")
    ));
    assert!(got.iter().any(
        |e| matches!(e, HarnessEvent::Chunk { text, .. } if text == "permission: reject_once")
    ));
}

#[tokio::test]
async fn setting_the_model_answers_the_new_effort_list() {
    let (_h, c, _ev) = open_fake().await;
    let s = c
        .start_session(&tmp(), SessionStart::New, &SessionSetup::default())
        .await
        .unwrap()
        .session_id;
    let all = c.set_option(&s, "model", "fake-small").await.unwrap();
    assert!(!all.to_string().contains("\"max\""));
}

#[tokio::test]
async fn a_process_that_exits_mid_prompt_is_closed() {
    let (_h, c, _ev) = open_fake().await;
    let s = c
        .start_session(&tmp(), SessionStart::New, &SessionSetup::default())
        .await
        .unwrap()
        .session_id;
    let result = c.prompt(&s, "exit", &[]).await;
    assert!(matches!(result, Err(AcpError::Closed)), "{result:?}");
}

#[tokio::test]
async fn usage_carries_context_model_and_rate_limit() {
    let (_h, c, mut ev) = open_fake().await;
    let s = c
        .start_session(&tmp(), SessionStart::New, &SessionSetup::default())
        .await
        .unwrap()
        .session_id;
    c.prompt(&s, "usage", &[]).await.unwrap();
    let usages: Vec<_> = drain(&mut ev)
        .into_iter()
        .filter_map(|e| match e {
            HarnessEvent::Usage {
                used,
                size,
                model,
                rate_limit,
                ..
            } => Some((used, size, model, rate_limit)),
            _ => None,
        })
        .collect();
    assert_eq!(usages.len(), 2);
    let last = usages.last().unwrap();
    assert_eq!(
        (last.0, last.1, last.2.as_deref()),
        (1234, 1_000_000, Some("fake-large-answering"))
    );
    assert!(last.3.as_ref().unwrap()["unifiedWindows"]["seven_day"].is_object());
}

#[tokio::test]
async fn a_tool_call_reports_its_real_title_and_final_status() {
    let (_h, c, mut ev) = open_fake().await;
    let s = c
        .start_session(&tmp(), SessionStart::New, &SessionSetup::default())
        .await
        .unwrap()
        .session_id;
    c.prompt(&s, "two-messages", &[]).await.unwrap();
    let tools: Vec<_> = drain(&mut ev)
        .into_iter()
        .filter_map(|e| match e {
            HarnessEvent::ToolCall { title, status, .. } => Some((title, status)),
            _ => None,
        })
        .collect();
    assert_eq!(
        tools,
        [
            (Some("Terminal".to_string()), Some("pending".to_string())),
            (Some("Read notes.md".to_string()), None),
            (None, Some("completed".to_string())),
        ]
    );
}

#[tokio::test]
async fn a_fork_is_resumed_before_it_is_answered() {
    let (_h, c, _ev) = open_fake().await;
    let src = c
        .start_session(&tmp(), SessionStart::New, &SessionSetup::default())
        .await
        .unwrap()
        .session_id;
    let fork = c
        .start_session(
            &tmp(),
            SessionStart::Fork(src.clone()),
            &SessionSetup::default(),
        )
        .await
        .unwrap();
    assert_eq!(fork.session_id, format!("fork-of-{src}"));
    assert_eq!(
        c.prompt(&fork.session_id, "hi", &[]).await.unwrap().end,
        TurnEnd::Ended
    );
}

#[tokio::test]
async fn a_model_the_account_cannot_use_is_refused_with_the_harness_message() {
    let (_h, c, _ev) = open_fake().await;
    let s = c
        .start_session(&tmp(), SessionStart::New, &SessionSetup::default())
        .await
        .unwrap()
        .session_id;
    let err = c.set_option(&s, "model", "fake-locked").await.unwrap_err();
    assert!(matches!(err, AcpError::Rpc(m) if m.contains("Usage credits are required")));
}

#[tokio::test]
async fn a_refused_prompt_names_the_acp_stop_reason() {
    let (_h, c, _ev) = open_fake().await;
    let s = c
        .start_session(&tmp(), SessionStart::New, &SessionSetup::default())
        .await
        .unwrap()
        .session_id;
    assert_eq!(
        c.prompt(&s, "refuse", &[]).await.unwrap().end,
        TurnEnd::Refused("max_tokens".into())
    );
}

#[test]
fn claude_adapter_launches_node_with_the_explicit_agent() {
    let adapter = ClaudeAdapter::new(
        PathBuf::from("node.exe"),
        PathBuf::from("adapter.js"),
        PathBuf::from("claude.exe"),
        "1".into(),
        "2".into(),
    );
    let cwd = tmp();
    let spec = adapter.process_spec(&cwd);
    assert_eq!(spec.executable, PathBuf::from("node.exe"));
    assert_eq!(spec.args, ["adapter.js"]);
    assert_eq!(
        spec.env.last(),
        Some(&("CLAUDE_CODE_EXECUTABLE".into(), "claude.exe".into())),
        "the rest is inherited by name: tests/harness_config.rs"
    );
    assert_eq!(spec.cwd, cwd);
    assert!(spec.capture_stdout && spec.pipe_stdin);
}

#[tokio::test]
async fn changing_model_adjusts_mode_and_resuming_starts_at_personal_defaults() {
    let (_h, c, mut ev) = open_fake().await;
    let s = c
        .start_session(&tmp(), SessionStart::New, &SessionSetup::default())
        .await
        .unwrap()
        .session_id;
    c.set_option(&s, "model", "fake-small").await.unwrap();
    assert_eq!(
        c.prompt(&s, "report", &[]).await.unwrap().end,
        TurnEnd::Ended
    );
    let changed = drain(&mut ev)
        .into_iter()
        .find_map(|e| match e {
            HarnessEvent::Chunk { text, .. } => {
                serde_json::from_str::<serde_json::Value>(&text).ok()
            }
            _ => None,
        })
        .unwrap();
    assert_eq!(changed["model"], "fake-small");
    assert_eq!(changed["mode"], "acceptEdits");
    assert!(matches!(
        c.set_option(&s, "mode", "auto").await,
        Err(AcpError::Rpc(_))
    ));
    c.start_session(
        &tmp(),
        SessionStart::Resume(s.clone()),
        &SessionSetup::default(),
    )
    .await
    .unwrap();
    c.prompt(&s, "report", &[]).await.unwrap();
    let resumed = drain(&mut ev)
        .into_iter()
        .find_map(|e| match e {
            HarnessEvent::Chunk { text, .. } => {
                serde_json::from_str::<serde_json::Value>(&text).ok()
            }
            _ => None,
        })
        .unwrap();
    assert_eq!(resumed["model"], "fake-large");
    assert_eq!(resumed["mode"], "auto");
    assert_eq!(resumed["how"], "resume");
}

#[tokio::test]
async fn prompt_returns_raw_turn_usage() {
    let (_h, c, _ev) = open_fake().await;
    let s = c
        .start_session(&tmp(), SessionStart::New, &SessionSetup::default())
        .await
        .unwrap()
        .session_id;
    let r = c.prompt(&s, "turn-usage", &[]).await.unwrap();
    assert_eq!(r.end, TurnEnd::Ended);
    let u = r.usage.unwrap();
    assert_eq!(
        (
            u.input_tokens,
            u.output_tokens,
            u.cached_read_tokens,
            u.cached_write_tokens,
            u.total_tokens
        ),
        (10, 4, 20, 30, 64)
    );
    assert!(c.prompt(&s, "hi", &[]).await.unwrap().usage.is_none());
}

async fn ask(
    c: &Connection,
    ev: &mut mpsc::UnboundedReceiver<HarnessEvent>,
) -> (
    String,
    String,
    tokio::task::JoinHandle<Result<shadows_agent::acp::PromptResult, AcpError>>,
) {
    let session = c
        .start_session(&tmp(), SessionStart::New, &SessionSetup::default())
        .await
        .unwrap()
        .session_id;
    let connection = c.clone();
    let id = session.clone();
    let turn = tokio::spawn(async move { connection.prompt(&id, "ask-permission", &[]).await });
    let request = tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            if let Some(HarnessEvent::PermissionAsked { request, .. }) = ev.recv().await {
                break request;
            }
        }
    })
    .await
    .unwrap();
    (session, request, turn)
}

#[tokio::test]
async fn a_refusing_connection_rejects_at_once() {
    let (_h, c, mut ev) = open_fake().await;
    let session = c
        .start_session(&tmp(), SessionStart::New, &SessionSetup::default())
        .await
        .unwrap()
        .session_id;
    c.prompt(&session, "ask-permission", &[]).await.unwrap();
    assert!(drain(&mut ev).iter().any(|e| matches!(
        e,
        HarnessEvent::PermissionRefused {
            request: None,
            reason: None,
            ..
        }
    )));
}

#[tokio::test]
async fn a_held_request_waits_for_its_answer() {
    let (_h, c, mut ev) = open_with_policy(PermissionPolicy::Ask).await;
    let (_, request, turn) = ask(&c, &mut ev).await;
    assert_eq!(c.held_permissions().len(), 1);
    let options = serde_json::to_value(&c.held_permissions()[0].options).unwrap();
    assert_eq!(options[0]["option_id"], "allow_once");
    assert!(options[0].get("optionId").is_none());
    c.answer_permission(&request, "allow_once").await.unwrap();
    assert!(c.held_permissions().is_empty());
    assert!(matches!(
        ev.recv().await,
        Some(HarnessEvent::PermissionAnswered { .. })
    ));
    turn.await.unwrap().unwrap();
    assert!(drain(&mut ev).iter().any(|e| matches!(e,
        HarnessEvent::Chunk { text, .. } if text == "permission: allow_once")));
}

#[tokio::test]
async fn an_option_not_offered_is_refused_and_the_request_stays() {
    let (_h, c, mut ev) = open_with_policy(PermissionPolicy::Ask).await;
    let (_, request, turn) = ask(&c, &mut ev).await;
    assert!(matches!(
        c.answer_permission(&request, "bogus").await,
        Err(PermissionError::OptionNotOffered)
    ));
    assert_eq!(c.held_permissions().len(), 1);
    c.refuse_held("closed").await;
    turn.await.unwrap().unwrap();
}

#[tokio::test]
async fn a_second_answer_finds_the_request_settled() {
    let (_h, c, mut ev) = open_with_policy(PermissionPolicy::Ask).await;
    let (_, request, turn) = ask(&c, &mut ev).await;
    c.answer_permission(&request, "reject_once").await.unwrap();
    assert!(matches!(
        c.answer_permission(&request, "reject_once").await,
        Err(PermissionError::Settled)
    ));
    assert!(matches!(
        c.answer_permission("unknown", "reject_once").await,
        Err(PermissionError::Unknown)
    ));
    turn.await.unwrap().unwrap();
}

#[tokio::test]
async fn cancel_refuses_held_requests_before_cancelling() {
    let (_h, c, mut ev) = open_with_policy(PermissionPolicy::Ask).await;
    let (session, request, turn) = ask(&c, &mut ev).await;
    c.cancel(&session);
    assert!(
        matches!(ev.recv().await, Some(HarnessEvent::PermissionRefused {
        request: Some(id), reason: Some(reason), ..
    }) if id == request && reason == "cancelled")
    );
    assert!(c.held_permissions().is_empty());
    turn.await.unwrap().unwrap();
}

#[tokio::test]
async fn a_prepared_answer_serializes_with_cancel_and_drop_keeps_it_waiting() {
    let (_h, c, mut ev) = open_with_policy(PermissionPolicy::Ask).await;
    let (session, request, turn) = ask(&c, &mut ev).await;
    let prepared = c
        .prepare_permission_answer(&request, "allow_once")
        .await
        .unwrap();
    drop(prepared);
    assert_eq!(c.held_permissions().len(), 1);
    let prepared = c
        .prepare_permission_answer(&request, "allow_once")
        .await
        .unwrap();
    c.cancel(&session);
    tokio::task::yield_now().await;
    assert_eq!(c.held_permissions().len(), 1);
    prepared.commit().unwrap();
    assert!(matches!(
        ev.recv().await,
        Some(HarnessEvent::PermissionAnswered { .. })
    ));
    turn.await.unwrap().unwrap();
}

#[tokio::test]
async fn refuse_held_ends_every_waiting_request() {
    let (_h, c, mut ev) = open_with_policy(PermissionPolicy::Ask).await;
    let (_, request, turn) = ask(&c, &mut ev).await;
    let (_, second, other_turn) = ask(&c, &mut ev).await;
    assert_eq!(c.held_permissions().len(), 2);
    c.refuse_held("stopped").await;
    let mut ids = Vec::new();
    for _ in 0..2 {
        if let Some(HarnessEvent::PermissionRefused {
            request: Some(id),
            reason: Some(reason),
            ..
        }) = ev.recv().await
        {
            assert_eq!(reason, "stopped");
            ids.push(id);
        }
    }
    assert!(ids.contains(&request));
    assert!(ids.contains(&second));
    assert!(c.held_permissions().is_empty());
    turn.await.unwrap().unwrap();
    other_turn.await.unwrap().unwrap();
}
