use std::{path::PathBuf, time::Duration};

use shadows::agent::{
    acp::{AcpError, Connection, SessionStart, TurnEnd},
    claude::ClaudeAdapter,
    events::HarnessEvent,
};
use shadows::process::{self, ProcessHandle, ProcessSpec};
use tokio::sync::mpsc;

fn tmp() -> PathBuf {
    std::env::temp_dir()
}

async fn open_fake() -> (
    ProcessHandle,
    Connection,
    mpsc::UnboundedReceiver<HarnessEvent>,
) {
    let executable = PathBuf::from(env!("CARGO_BIN_EXE_fake_acp"));
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
    let connection = Connection::open(&mut handle, tx).await.unwrap();
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
    let opened = c.start_session(&tmp(), SessionStart::New).await.unwrap();
    assert!(opened.session_id.starts_with("fake-"));
    assert!(opened.options.to_string().contains("fake-large"));
    assert_eq!(
        c.prompt(&opened.session_id, "hi").await.unwrap(),
        TurnEnd::Ended
    );
    let chunks = drain(&mut ev);
    assert!(
        matches!(&chunks[0], HarnessEvent::Chunk { message_id: Some(m), text } if m == "m1" && text == "hello ")
    );
}

#[tokio::test]
async fn a_cancelled_prompt_answers_cancelled() {
    let (_h, c, _ev) = open_fake().await;
    let s = c
        .start_session(&tmp(), SessionStart::New)
        .await
        .unwrap()
        .session_id;
    let c2 = c.clone();
    let s2 = s.clone();
    let turn = tokio::spawn(async move { c2.prompt(&s2, "hang").await });
    tokio::time::sleep(Duration::from_millis(200)).await;
    c.cancel(&s);
    assert_eq!(turn.await.unwrap().unwrap(), TurnEnd::Cancelled);
}

#[tokio::test]
async fn a_permission_request_is_refused_and_reported() {
    let (_h, c, mut ev) = open_fake().await;
    let s = c
        .start_session(&tmp(), SessionStart::New)
        .await
        .unwrap()
        .session_id;
    c.prompt(&s, "ask-permission").await.unwrap();
    let got = drain(&mut ev);
    assert!(got.iter().any(
        |e| matches!(e, HarnessEvent::PermissionRefused { title } if title == "Run echo probe")
    ));
    assert!(got.iter().any(
        |e| matches!(e, HarnessEvent::Chunk { text, .. } if text == "permission: reject_once")
    ));
}

#[tokio::test]
async fn setting_the_model_answers_the_new_effort_list() {
    let (_h, c, _ev) = open_fake().await;
    let s = c
        .start_session(&tmp(), SessionStart::New)
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
        .start_session(&tmp(), SessionStart::New)
        .await
        .unwrap()
        .session_id;
    let result = c.prompt(&s, "exit").await;
    assert!(matches!(result, Err(AcpError::Closed)), "{result:?}");
}

#[tokio::test]
async fn usage_carries_context_model_and_rate_limit() {
    let (_h, c, mut ev) = open_fake().await;
    let s = c
        .start_session(&tmp(), SessionStart::New)
        .await
        .unwrap()
        .session_id;
    c.prompt(&s, "usage").await.unwrap();
    let usages: Vec<_> = drain(&mut ev)
        .into_iter()
        .filter_map(|e| match e {
            HarnessEvent::Usage {
                used,
                size,
                model,
                rate_limit,
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
        .start_session(&tmp(), SessionStart::New)
        .await
        .unwrap()
        .session_id;
    c.prompt(&s, "two-messages").await.unwrap();
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
        .start_session(&tmp(), SessionStart::New)
        .await
        .unwrap()
        .session_id;
    let fork = c
        .start_session(&tmp(), SessionStart::Fork(src.clone()))
        .await
        .unwrap();
    assert_eq!(fork.session_id, format!("fork-of-{src}"));
    assert_eq!(
        c.prompt(&fork.session_id, "hi").await.unwrap(),
        TurnEnd::Ended
    );
}

#[tokio::test]
async fn a_model_the_account_cannot_use_is_refused_with_the_harness_message() {
    let (_h, c, _ev) = open_fake().await;
    let s = c
        .start_session(&tmp(), SessionStart::New)
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
        .start_session(&tmp(), SessionStart::New)
        .await
        .unwrap()
        .session_id;
    assert_eq!(
        c.prompt(&s, "refuse").await.unwrap(),
        TurnEnd::Refused("max_tokens".into())
    );
}

#[test]
fn claude_adapter_launches_node_with_the_explicit_agent() {
    let adapter = ClaudeAdapter {
        node: PathBuf::from("node.exe"),
        adapter: PathBuf::from("adapter.js"),
        agent: PathBuf::from("claude.exe"),
        adapter_version: "1".into(),
        agent_version: "2".into(),
    };
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
        .start_session(&tmp(), SessionStart::New)
        .await
        .unwrap()
        .session_id;
    c.set_option(&s, "model", "fake-small").await.unwrap();
    assert_eq!(c.prompt(&s, "report").await.unwrap(), TurnEnd::Ended);
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
    c.start_session(&tmp(), SessionStart::Resume(s.clone()))
        .await
        .unwrap();
    c.prompt(&s, "report").await.unwrap();
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
