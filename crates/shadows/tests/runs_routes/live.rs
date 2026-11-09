//! A task run end to end over HTTP on the real Claude adapter, against
//! MiniMax through a Claude profile, with the executor reaching Shadows'
//! `/mcp` on a real port. CI skips it; run it with `API_KEY_MINIMAX` set and
//! `-- --ignored --test-threads=1`.
use super::*;
use shadows_agent::claude::ClaudeAdapter;
use shadows_agent::profile::ClaudeProfile;
use std::sync::Arc;
use std::time::Duration;

const ADAPTER_VERSION: &str = "0.101.0";
const CLAUDE_VERSION: &str = "2.1.319";
const MINIMAX_URL: &str = "https://api.minimax.io/anthropic";
const MODEL: &str = "MiniMax-M3";
/// What the person picks: the adapter's own alias, which the profile maps to `MODEL`.
const PICKED: &str = "default";

/// The real adapter on MiniMax, started through a launcher that sets the
/// profile's environment; the key stays in `dir`, which the test deletes.
struct LiveAdapter {
    adapter: Arc<ClaudeAdapter>,
    _dir: tempfile::TempDir,
}

fn live_adapter() -> LiveAdapter {
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
        "executor-routes-live".into(),
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

/// An approved v1 whose one task says what to do. Only `task_context` carries
/// it to the executor: the first prompt is the production one, naming T1 alone.
async fn approved_task(app: &App) -> WorkflowId {
    let v1 = plan::draft(app).await.workflow_id;
    let mut t1 = plan::task(1, "Add t1");
    t1.goal = "Do exactly these steps, nothing else. \
               1. Use the Write tool to create src/t1.rs containing `pub fn t1() {}`. \
               2. Call the task_commit tool with the message `Add t1`. \
               3. Reply DONE."
        .into();
    plan::edit(app, &v1, 0, &[shadows_core::PlanOp::TaskAdd { task: t1 }]).await;
    shadows_core::AppCore::plans_for_tests(app.storage.clone())
        .approve_plan_for_test(&app::ctx("approve-1", "PlanApprove"), &v1, 1)
        .await
        .unwrap();
    v1
}

/// Start over HTTP with the production first prompt; the executor reads its
/// task through `task_context` naming no plan version, writes, commits through
/// `task_commit` over the real `/mcp`, and the commit holds only the task's
/// path: the person's change outside it stays. The Diff route reads it.
#[tokio::test]
#[ignore = "live: needs API_KEY_MINIMAX"]
async fn live_a_task_started_over_http_commits_only_its_writes() {
    let live = live_adapter();
    let tmp = tempfile::tempdir().unwrap();
    let l = listening::listening_on(tmp.path(), live.adapter.clone()).await;
    let app = &l.app;
    let root = folder(app).await;
    repository(&root);
    let v1 = approved_task(app).await;
    // The person's own change outside the task's writes.
    std::fs::write(root.join("notes.txt"), "x\n").unwrap();
    let path = format!("{}/1/attempts", tasks(&v1));
    let (status, attempt) = post(
        app,
        &path,
        json!({ "command_id": "live-1", "model": PICKED }),
    )
    .await;
    assert_eq!(status, 201, "{attempt}");
    assert_eq!(attempt["state"], "running", "{attempt}");
    let id = attempt["id"].as_str().unwrap().to_string();
    let mut last = Value::Null;
    for _ in 0..1800 {
        let (_, t1) = call(app, "GET", &format!("{}/1", tasks(&v1)), None).await;
        last = t1["latest"].clone();
        if last["state"] != "running" {
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    println!("live attempt: {last}");
    assert_eq!(last["state"], "committed", "{last}");
    let commit = last["commit"].as_str().unwrap();
    let names = git(&root, &["show", "--name-only", "--format=", commit]);
    assert_eq!(names.trim(), "src/t1.rs", "{names}");
    // Only the task, read through task_context, named this content.
    let written = git(&root, &["show", &format!("{commit}:src/t1.rs")]);
    assert!(written.contains("pub fn t1"), "{written}");
    let message = git(&root, &["log", "-1", "--format=%B", commit]);
    assert!(
        message.contains(&format!("Shadows-Attempt: {id}")),
        "{message}"
    );
    let status = git(&root, &["status", "--porcelain", "-uall"]);
    println!("git status: {status:?}");
    assert!(status.contains("notes.txt"), "{status}");
    assert!(!status.contains("src/t1.rs"), "{status}");
    let (code, diff) = call(app, "GET", &format!("/api/attempts/{id}/diff"), None).await;
    assert_eq!(code, 200);
    assert_eq!(diff[0]["path"], "src/t1.rs", "{diff}");
    assert_eq!(diff[0]["added"], 1, "{diff}");
    app.core
        .shut_down(Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
}
