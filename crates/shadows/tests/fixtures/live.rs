//! The real Claude adapter, on MiniMax or on the person's signed-in Claude
//! Code, for this crate's live tests. CI skips them; run one with
//! `API_KEY_MINIMAX` set (or signed in) and `-- --ignored --test-threads=1`.

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
    let env = profile.spawn_env_with(&[], Some(&key)).unwrap();
    launched(dir, env)
}

/// The model picked on the subscription.
pub const SUBSCRIPTION_MODEL: &str = "sonnet";

/// The real adapter on the person's signed-in Claude Code, with no provider
/// override: `auto`'s safety check runs on Anthropic's server, which MiniMax
/// does not pass through, so there `auto` refuses every `Bash` call. The
/// launcher hands the adapter this process's environment, without any
/// `ANTHROPIC_` name or `CLAUDE_CONFIG_DIR`, so Claude Code reads its own
/// sign-in from the user's home.
pub fn subscription_adapter() -> LiveAdapter {
    let env = std::env::vars()
        .filter(|(name, _)| {
            !name.starts_with("ANTHROPIC_")
                && !name.starts_with("API_KEY_")
                && name != "CLAUDE_CONFIG_DIR"
        })
        .collect();
    launched(tempfile::tempdir().unwrap(), env)
}

/// The pinned adapter run by a launcher in `dir` that first sets `env`.
fn launched(dir: tempfile::TempDir, env: Vec<(String, String)>) -> LiveAdapter {
    let env: serde_json::Map<String, Value> = env
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
    // The person's Windows install unless the machine names its own, as
    // `planner_permissions` reads them.
    let installed = |var: &str, default: &str| {
        std::env::var_os(var).map_or_else(|| PathBuf::from(default), PathBuf::from)
    };
    let adapter = ClaudeAdapter::new(
        installed("SHADOWS_NODE_PATH", r"C:\Program Files\nodejs\node.exe"),
        launcher,
        installed(
            "SHADOWS_HARNESS_PATH",
            r"C:\Users\Mohammed\.local\bin\claude.exe",
        ),
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
    planner_turn_on(app, command, prompt, PICKED).await
}

/// `planner_turn` asking for `model`.
pub async fn planner_turn_on(app: &App, command: &str, prompt: &str, model: &str) -> Operation {
    let turn = SendTurn {
        command_id: command.into(),
        prompt: prompt.into(),
        attachments: vec![],
        model: model.into(),
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
