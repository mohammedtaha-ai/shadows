//! One authorized live ACP test of product-generated extension homes.
use super::*;
use shadows_agent::{
    acp::{Connection, SessionSetup, SessionStart, TurnEnd},
    claude::ClaudeAdapter,
    events::HarnessEvent,
    profile::ClaudeProfile,
};
use shadows_core::{SendTurn, ThreadEntry, ThreadEntryKind, ThreadId};
use std::{path::Path, sync::Arc, time::Duration};

const ADAPTER_VERSION: &str = "0.101.0";
const CLAUDE_VERSION: &str = "2.1.317";
const MINIMAX_URL: &str = "https://api.minimax.io/anthropic";
const MODEL: &str = "MiniMax-M3";

fn complete_onboarding(home: &Path) {
    std::fs::create_dir_all(home).unwrap();
    std::fs::write(
        home.join(".claude.json"),
        r#"{"hasCompletedOnboarding":true}"#,
    )
    .unwrap();
}

fn live_adapter() -> ClaudeAdapter {
    ClaudeAdapter::new(
        r"C:\Program Files\nodejs\node.exe".into(),
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../harness/claude/node_modules/@agentclientprotocol")
            .join("claude-agent-acp/dist/index.js"),
        r"C:\Users\Mohammed\.local\bin\claude.exe".into(),
        ADAPTER_VERSION.into(),
        CLAUDE_VERSION.into(),
    )
}

async fn commands(home: &Path) -> Vec<String> {
    let key = std::env::var("API_KEY_MINIMAX").expect("authorized MiniMax key required");
    complete_onboarding(home);
    let adapter = live_adapter();
    // The local fixture's fake model ids are replaced only in the live harness fixture.
    // Keep the product-generated extension settings and warm plugin cache intact.
    let settings_path = home.join("settings.json");
    let mut settings: Value =
        serde_json::from_slice(&std::fs::read(&settings_path).unwrap()).unwrap();
    settings["model"] = MODEL.into();
    settings["availableModels"] = json!([MODEL]);
    std::fs::write(&settings_path, serde_json::to_vec(&settings).unwrap()).unwrap();
    let model = MODEL;
    let profile = ClaudeProfile::new(
        "extension-live".into(),
        home.into(),
        vec![
            ("ANTHROPIC_BASE_URL".into(), MINIMAX_URL.into()),
            ("ANTHROPIC_MODEL".into(), model.into()),
            ("ANTHROPIC_DEFAULT_HAIKU_MODEL".into(), model.into()),
            ("ANTHROPIC_DEFAULT_SONNET_MODEL".into(), model.into()),
            ("ANTHROPIC_DEFAULT_OPUS_MODEL".into(), model.into()),
            ("CLAUDE_CODE_MAX_OUTPUT_TOKENS".into(), "1024".into()),
            (
                "CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC".into(),
                "1".into(),
            ),
        ],
    );
    let mut process = shadows_process::spawn(
        adapter
            .process_spec_with(home, Some(&profile), &[], Some(&key))
            .unwrap(),
    )
    .unwrap();
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    let result = tokio::time::timeout(Duration::from_secs(120), async {
        let connection = Connection::open(
            &mut process,
            shadows_agent::acp::PermissionPolicy::Refuse,
            move |event| {
                if let HarnessEvent::Commands(v) = event {
                    let _ = tx.send(v);
                }
            },
        )
        .await
        .map_err(|_| "initialize failed")?;
        let session = connection
            .start_session(
                home,
                SessionStart::New,
                &SessionSetup {
                    setting_sources: Some(vec!["user".into()]),
                    ..Default::default()
                },
            )
            .await
            .map_err(|_| "opening failed")?;
        connection
            .prompt(&session.session_id, "say hi", &[])
            .await
            .map(|r| r.end)
            .map_err(|_| "prompt failed")
    })
    .await;
    process.terminate_tree().unwrap();
    assert!(
        matches!(result, Ok(Ok(TurnEnd::Ended))),
        "live ACP turn did not complete"
    );
    let mut names = Vec::new();
    while let Ok(v) = rx.try_recv() {
        names = v.into_iter().map(|c| c.name).collect();
    }
    names
}

async fn mode_turn(w: &World, thread: &ThreadId, opening: &str, mode: &str) -> ThreadEntry {
    let opened = w.sessions.open(thread).await.unwrap();
    assert_eq!(opened.how, opening);
    let choices = w.core.harness().open_session(thread).await.unwrap();
    assert_eq!(choices.current.mode, mode);
    let operation = w
        .core
        .turns()
        .send(
            thread.clone(),
            SendTurn {
                command_id: uuid::Uuid::new_v4().to_string(),
                prompt: "Reply with the single word OK. Do not use tools.".into(),
                model: choices.current.model,
                effort: choices.current.effort,
                mode: choices.current.mode,
                attachments: Vec::new(),
                focus: None,
                plan: None,
                client_tab: None,
            },
        )
        .await
        .unwrap();
    tokio::time::timeout(Duration::from_secs(90), async {
        loop {
            let op = shadows_core::Turns::get_operation_for_test(&w.storage, &operation)
                .await
                .unwrap();
            if op.finished_at.is_some() {
                assert_eq!(op.status_kind, "Completed", "{opening} turn");
                break;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    })
    .await
    .expect("live mode turn did not finish");
    let reply = w
        .core
        .threads()
        .entries(thread)
        .await
        .unwrap()
        .pop()
        .unwrap();
    assert_eq!(reply.kind, ThreadEntryKind::AgentMessage);
    assert!(!reply.body.trim().is_empty());
    reply
}

async fn live_modes() {
    let key = std::env::var("API_KEY_MINIMAX").expect("authorized MiniMax key required");
    let w = world_with_provider(Arc::new(live_adapter()), MINIMAX_URL, &[MODEL], &key).await;
    complete_onboarding(&w._tmp.path().join("home/profiles/aylul"));
    mode_turn(&w, &w.thread, "new", "acceptEdits").await;
    w.core
        .harness()
        .change_mode("auto".into(), &w.thread, "auto")
        .await
        .unwrap();
    w.sessions.terminate(&w.thread).await.unwrap();
    let reply = mode_turn(&w, &w.thread, "resume", "auto").await;
    let fork = w
        .core
        .harness()
        .fork_thread("fork".into(), &w.thread, &reply.id)
        .await
        .unwrap();
    mode_turn(&w, &fork.id, "fork", "auto").await;
    w.core
        .shut_down(Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
    println!("Live modes before first prompt: new=acceptEdits, resume=auto, fork=auto");
}

#[tokio::test]
#[ignore = "one authorized live MiniMax acceptance after local checks"]
async fn live_agent_extension_isolation() {
    let w = world().await;
    w.core
        .agents()
        .create_agent("b".into(), agent("Other"))
        .await
        .unwrap();
    let mut items = Vec::new();
    for (kind, name) in [
        (ExtensionKind::Skill, "live-skill"),
        (ExtensionKind::Plugin, "live-plugin"),
    ] {
        let src = source(&w, kind, name);
        let item = w
            .core
            .extensions()
            .add_extension(name.into(), kind, &src)
            .await
            .unwrap();
        w.core
            .extensions()
            .set_agent_extension(format!("on-{name}"), "aylul", kind, &item.id, true)
            .await
            .unwrap();
        items.push((kind, item.id));
    }
    let root = w._tmp.path().join("home/profiles");
    w.core.harness().open_session(&w.thread).await.unwrap();
    w.sessions.terminate(&w.thread).await.unwrap();
    let a = commands(&root.join("aylul")).await;
    assert!(a.iter().any(|c| c == "live-skill"));
    assert!(a.iter().any(|c| c == "live-plugin:hello"));
    let b = commands(&root.join("other")).await;
    assert!(!b.iter().any(|c| c.starts_with("live-")));
    for (kind, id) in items {
        w.core
            .extensions()
            .set_agent_extension(format!("off-{id}"), "aylul", kind, &id, false)
            .await
            .unwrap();
    }
    // The real adapter's cache remains in A; projection must explicitly disable it.
    w.core.harness().open_session(&w.thread).await.unwrap();
    w.sessions.terminate(&w.thread).await.unwrap();
    let disabled = commands(&root.join("aylul")).await;
    assert!(!disabled.iter().any(|c| c.starts_with("live-")));
    println!(
        "ACP {ADAPTER_VERSION} / Claude {CLAUDE_VERSION}: A skill/plugin; B neither; A disabled"
    );

    w.core
        .shut_down(std::time::Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
    live_modes().await;
}
