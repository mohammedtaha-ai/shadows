//! AYLUL threads open Claude Code on the profile (home as cwd, profile env, no
//! Planner prompt, tools or grant) and live only in the AYLUL project.
//! `fake_acp`'s `report` prompt answers with its cwd, env and session options.

use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use serde_json::Value;
use shadows_agent::policy;
use shadows_agent::profile::ClaudeProfile;
use shadows_core::testing::{LiveHandles, Runtime, Sessions, SessionsConfig, Storage, acp, turn};
use shadows_core::{AppCore, AylulConfig, CoreParts, OpenError, ThreadEntryKind, ThreadId};

/// `PATH` is a variable every process has, so the key resolves without
/// `set_var`; `key_env` names a variable nothing sets for the missing case.
fn config(home: &Path, key_env: &str) -> AylulConfig {
    AylulConfig {
        profile: ClaudeProfile {
            name: "aylul".into(),
            home: home.to_path_buf(),
            key_env: key_env.into(),
            env: vec![("ANTHROPIC_MODEL".into(), "MiniMax-M3".into())],
        },
        project_slug: "aylul".into(),
    }
}

struct World {
    _tmp: tempfile::TempDir,
    home: std::path::PathBuf,
    storage: Arc<Storage>,
    runtime: Arc<Runtime>,
    sessions: Arc<Sessions>,
    core: Arc<AppCore>,
    handles: Arc<LiveHandles>,
    thread: ThreadId,
}

async fn world(key_env: &str) -> World {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    std::fs::create_dir(&home).unwrap();
    let storage = Arc::new(Storage::open(&tmp.path().join("s.sqlite3")).await.unwrap());
    let runtime = Arc::new(Runtime::start(storage.clone()).await.unwrap().0);
    let sessions = Sessions::new(
        acp::fake_adapter(),
        storage.clone(),
        SessionsConfig {
            aylul: Some(config(&home, key_env)),
            ..acp::test_config()
        },
    );
    let handles = Arc::new(LiveHandles::default());
    let core = AppCore::assemble(CoreParts {
        storage: storage.clone(),
        runtime: runtime.clone(),
        sessions: sessions.clone(),
        handles: handles.clone(),
        bus: tokio::sync::broadcast::channel(16).0,
        ui: tokio::sync::broadcast::channel(16).0,
        mcp_url: acp::MCP_URL.into(),
    });
    let project = core
        .projects()
        .create("c1".into(), "aylul", "AYLUL", home.to_str().unwrap())
        .await
        .unwrap();
    let thread = core
        .threads()
        .create("c2".into(), &project.id, "talk", Some(policy::AYLUL))
        .await
        .unwrap()
        .id;
    World {
        _tmp: tmp,
        home,
        storage,
        runtime,
        sessions,
        core,
        handles,
        thread,
    }
}

#[tokio::test]
async fn aylul_open_spawns_with_profile_env_and_home_cwd() {
    let w = world("PATH").await;
    let bus = tokio::sync::broadcast::channel(16).0;
    let op = turn::start_direct(
        &w.runtime,
        &w.handles,
        &w.sessions,
        &bus,
        &w.thread,
        "report",
    )
    .await
    .unwrap();
    for _ in 0..200 {
        if w.storage
            .get_operation(&op)
            .await
            .unwrap()
            .finished_at
            .is_some()
        {
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    let entries = w.storage.list_thread_entries(&w.thread).await.unwrap();
    let reply = entries
        .iter()
        .rev()
        .find(|e| e.kind == ThreadEntryKind::AgentMessage)
        .expect("an agent reply");
    let report: Value = serde_json::from_str(&reply.body).unwrap();
    let home = w.home.to_str().unwrap();
    assert_eq!(report["config_dir"], home);
    assert_eq!(
        std::fs::canonicalize(report["cwd"].as_str().unwrap()).unwrap(),
        std::fs::canonicalize(&w.home).unwrap()
    );
    assert_eq!(report["has_token"], true);
    // No Planner prompt, tools or grant: only the user settings source.
    assert_eq!(
        report["options"]["settingSources"],
        serde_json::json!(["user"])
    );
    assert!(report["options"].get("tools").is_none());
    assert!(report["append"].is_null());
    assert!(report["mcp"].is_null());
    // The turn carries only the person's text: no stage line, no context block.
    assert_eq!(report["blocks"].as_array().map(Vec::len), Some(1));
}

#[tokio::test]
async fn aylul_open_without_key_refuses_before_spawn() {
    let w = world("AYLUL_TEST_KEY_NEVER_SET_7f3").await;
    let error = w.sessions.open(&w.thread).await.err().expect("refused");
    match error {
        OpenError::Start(text) => assert!(text.contains("AYLUL_TEST_KEY_NEVER_SET_7f3"), "{text}"),
        other => panic!("expected Start, got {other:?}"),
    }
}

#[tokio::test]
async fn aylul_model_choice_does_not_move_planner_memory() {
    let w = world("PATH").await;
    let settings = turn::default_turn_settings();
    let mut new = turn::new_turn(&w.thread, &w.runtime, "hi", &settings);
    new.harness_kind = policy::AYLUL;
    let command = turn::turn_command("t1", &w.thread, "hi", &settings);
    w.storage.start_turn(&command, new).await.unwrap();
    assert_eq!(
        w.storage.remembered_settings("claude-code").await.unwrap(),
        None
    );
    assert!(
        w.storage
            .remembered_settings("aylul")
            .await
            .unwrap()
            .is_some()
    );
}

#[tokio::test]
async fn aylul_harness_only_in_the_aylul_project() {
    let w = world("PATH").await;
    let other = tempfile::tempdir().unwrap();
    let normal = w
        .core
        .projects()
        .create(
            "c3".into(),
            "normal",
            "Normal",
            other.path().to_str().unwrap(),
        )
        .await
        .unwrap();
    let threads = w.core.threads();
    assert!(
        threads
            .create("c4".into(), &normal.id, "x", Some(policy::AYLUL))
            .await
            .is_err()
    );
    assert!(
        threads
            .create("c5".into(), &normal.id, "x", None)
            .await
            .is_ok()
    );
    let aylul = w.storage.turn_context(&w.thread).await.unwrap().project_id;
    assert!(
        threads
            .create("c6".into(), &aylul, "x", Some(policy::CLAUDE_CODE))
            .await
            .is_err()
    );
}
