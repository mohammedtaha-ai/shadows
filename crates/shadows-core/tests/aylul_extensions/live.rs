//! One authorized live ACP test of product-generated extension homes.
use super::*;
use shadows_agent::{
    acp::{Connection, SessionSetup, SessionStart, TurnEnd},
    claude::ClaudeAdapter,
    events::HarnessEvent,
    profile::ClaudeProfile,
};
use std::time::Duration;

async fn commands(home: &std::path::Path) -> Vec<String> {
    let key = std::env::var("API_KEY_MINIMAX").expect("authorized MiniMax key required");
    std::fs::write(
        home.join(".claude.json"),
        r#"{"hasCompletedOnboarding":true}"#,
    )
    .unwrap();
    let adapter = ClaudeAdapter::new(
        r"C:\Program Files\nodejs\node.exe".into(),
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../harness/claude/node_modules/@agentclientprotocol")
            .join("claude-agent-acp/dist/index.js"),
        r"C:\Users\Mohammed\.local\bin\claude.exe".into(),
        "0.81.1".into(),
        "2.1.303".into(),
    );
    // The local fixture's fake model ids are replaced only in the live harness fixture.
    // Keep the product-generated extension settings and warm plugin cache intact.
    let settings_path = home.join("settings.json");
    let mut settings: Value =
        serde_json::from_slice(&std::fs::read(&settings_path).unwrap()).unwrap();
    settings["model"] = "MiniMax-M3".into();
    settings["availableModels"] = json!(["MiniMax-M3"]);
    std::fs::write(&settings_path, serde_json::to_vec(&settings).unwrap()).unwrap();
    let model = "MiniMax-M3";
    let profile = ClaudeProfile::new(
        "extension-live".into(),
        home.into(),
        vec![
            (
                "ANTHROPIC_BASE_URL".into(),
                "https://api.minimax.io/anthropic".into(),
            ),
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
        let connection = Connection::open(&mut process, move |event| {
            if let HarnessEvent::Commands(v) = event {
                let _ = tx.send(v);
            }
        })
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
        "ACP 0.81.1 / Claude 2.1.303: A skill=true plugin=true; B neither; A disabled neither"
    );

    w.core
        .shut_down(std::time::Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
}
