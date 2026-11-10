//! The real Claude adapter on MiniMax, for this crate's live tests. CI skips
//! them; run one with `API_KEY_MINIMAX` set and `-- --ignored --test-threads=1`.

#![allow(dead_code)]

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use serde_json::Value;
use shadows_agent::claude::ClaudeAdapter;
use shadows_agent::profile::ClaudeProfile;
use shadows_core::{Operation, SendTurn};

use super::app::App;

const ADAPTER_VERSION: &str = "0.101.0";
const CLAUDE_VERSION: &str = "2.1.319";
const MINIMAX_URL: &str = "https://api.minimax.io/anthropic";
const MODEL: &str = "MiniMax-M3";
/// What the person picks: the adapter's own alias, which the profile maps to `MODEL`.
pub const PICKED: &str = "default";

/// The real adapter on MiniMax, started through a launcher that sets the
/// profile's environment; the key stays in `dir`, which the test deletes.
pub struct LiveAdapter {
    pub adapter: Arc<ClaudeAdapter>,
    _dir: tempfile::TempDir,
}

pub fn live_adapter() -> LiveAdapter {
    let key = std::env::var("API_KEY_MINIMAX").expect("API_KEY_MINIMAX is required");
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path().join("home");
    std::fs::create_dir_all(&home).unwrap();
    std::fs::write(
        home.join(".claude.json"),
        r#"{"hasCompletedOnboarding":true}"#,
    )
    .unwrap();
    let profile = ClaudeProfile::new(
        "planner-live".into(),
        home,
        [
            ("ANTHROPIC_BASE_URL", MINIMAX_URL),
            ("ANTHROPIC_MODEL", MODEL),
            ("ANTHROPIC_DEFAULT_HAIKU_MODEL", MODEL),
            ("ANTHROPIC_DEFAULT_SONNET_MODEL", MODEL),
            ("ANTHROPIC_DEFAULT_OPUS_MODEL", MODEL),
            ("CLAUDE_CODE_MAX_OUTPUT_TOKENS", "2048"),
            ("CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC", "1"),
        ]
        .map(|(name, value)| (name.to_string(), value.to_string()))
        .to_vec(),
    );
    let env: serde_json::Map<String, Value> = profile
        .spawn_env_with(&[], Some(&key))
        .unwrap()
        .into_iter()
        .map(|(name, value)| (name, Value::from(value)))
        .collect();
    std::fs::write(dir.path().join("env.json"), Value::Object(env).to_string()).unwrap();
    let pinned = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../harness/claude/node_modules/@agentclientprotocol")
        .join("claude-agent-acp/dist/index.js");
    let pinned = std::fs::canonicalize(pinned).unwrap();
    let launcher = dir.path().join("launch.mjs");
    std::fs::write(
        &launcher,
        format!(
            "import {{ readFileSync }} from 'node:fs';\n\
             import {{ pathToFileURL }} from 'node:url';\n\
             const env = JSON.parse(readFileSync(new URL('./env.json', import.meta.url)));\n\
             Object.assign(process.env, env);\n\
             await import(pathToFileURL({}).href);\n",
            Value::from(pinned.to_string_lossy().into_owned())
        ),
    )
    .unwrap();
    let adapter = ClaudeAdapter::new(
        r"C:\Program Files\nodejs\node.exe".into(),
        launcher,
        r"C:\Users\Mohammed\.local\bin\claude.exe".into(),
        ADAPTER_VERSION.into(),
        CLAUDE_VERSION.into(),
    );
    LiveAdapter {
        adapter: Arc::new(adapter),
        _dir: dir,
    }
}

/// Sends `prompt` on the app's Planner thread through the real adapter and
/// waits up to three minutes for the turn to end; answers the operation.
pub async fn planner_turn(app: &App, command: &str, prompt: &str) -> Operation {
    let turn = SendTurn {
        command_id: command.into(),
        prompt: prompt.into(),
        attachments: vec![],
        model: PICKED.into(),
        mode: "acceptEdits".into(),
        effort: None,
        focus: None,
        plan: None,
        client_tab: None,
    };
    let op = app
        .core
        .turns()
        .send_with_default_effort(app.thread.clone(), turn)
        .await
        .unwrap();
    let mut last = None;
    for _ in 0..1800 {
        let now = shadows_core::Turns::get_operation_for_test(&app.storage, &op)
            .await
            .unwrap();
        if !matches!(now.status_kind.as_str(), "Pending" | "Running") {
            return now;
        }
        last = Some(now);
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    panic!("the turn did not end in three minutes: {last:?}");
}
