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
use shadows_core::{CoreError, SendTurn};

/// `PATH` is a variable every process has, so the key resolves without
/// `set_var`; `key_env` names a variable nothing sets for the missing case.
fn config(home: &Path, key_env: &str) -> AylulConfig {
    AylulConfig {
        profile: ClaudeProfile {
            name: "aylul".into(),
            home: home.to_path_buf(),
            key_env: key_env.into(),
            env: vec![("ANTHROPIC_MODEL".into(), "MiniMax-M3".into())],
            env_file: None,
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
        secrets: shadows_core::testing::secrets(storage.clone(), None),
        storage: storage.clone(),
        runtime: runtime.clone(),
        sessions: sessions.clone(),
        handles: handles.clone(),
        bus: tokio::sync::broadcast::channel(16).0,
        ui: tokio::sync::broadcast::channel(16).0,
        mcp_url: acp::MCP_URL.into(),
    });
    // Under another slug than the configured one: the project is AYLUL's
    // because `ensure_assistant` marked it, not because of its slug.
    let project = core
        .projects()
        .ensure_assistant("assistant", "AYLUL", &home)
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
        // Holds the configured slug but was never marked: an ordinary project.
        .create(
            "c3".into(),
            "aylul",
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
    let plain = threads
        .create("c5".into(), &normal.id, "x", None)
        .await
        .unwrap();
    let aylul = w.storage.turn_context(&w.thread).await.unwrap().project_id;
    assert!(
        threads
            .create("c6".into(), &aylul, "x", Some(policy::CLAUDE_CODE))
            .await
            .is_err()
    );
    // A harness change follows the same rule, both ways.
    assert!(
        threads
            .set_harness("c7".into(), &plain.id, policy::AYLUL)
            .await
            .is_err()
    );
    assert!(
        threads
            .set_harness("c8".into(), &w.thread, policy::CLAUDE_CODE)
            .await
            .is_err()
    );
}

fn send(prompt: &str) -> SendTurn {
    SendTurn {
        command_id: "send-1".into(),
        prompt: prompt.into(),
        model: "fake-large".into(),
        mode: "acceptEdits".into(),
        effort: Some("high".into()),
        focus: None,
        plan: None,
        client_tab: None,
    }
}

#[tokio::test]
async fn aylul_turn_starts_when_configured() {
    let w = world("PATH").await;
    let op = w
        .core
        .turns()
        .send(w.thread.clone(), send("report"))
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
        .find(|e| e.kind == ThreadEntryKind::AgentMessage)
        .expect("the adapter got a prompt and answered");
    let report: Value = serde_json::from_str(&reply.body).unwrap();
    assert_eq!(
        report["options"]["settingSources"],
        serde_json::json!(["user"])
    );
}

#[tokio::test]
async fn aylul_is_unavailable_without_a_profile() {
    // The thread exists from an earlier run with a profile; this daemon has none.
    let with = world("PATH").await;
    let w = World {
        core: AppCore::assemble(CoreParts {
            secrets: shadows_core::testing::secrets(with.storage.clone(), None),
            storage: with.storage.clone(),
            runtime: with.runtime.clone(),
            sessions: Sessions::new(
                acp::fake_adapter(),
                with.storage.clone(),
                acp::test_config(),
            ),
            handles: with.handles.clone(),
            bus: tokio::sync::broadcast::channel(16).0,
            ui: tokio::sync::broadcast::channel(16).0,
            mcp_url: acp::MCP_URL.into(),
        }),
        ..with
    };
    let error = w
        .core
        .turns()
        .send(w.thread.clone(), send("hi"))
        .await
        .unwrap_err();
    assert!(
        matches!(error, CoreError::HarnessUnavailable(_)),
        "{error:?}"
    );
    let aylul = aylul_entry(&w).await;
    assert_eq!(aylul["available"], false);
    assert_eq!(aylul["reason"], "Coming later");
}

#[tokio::test]
async fn harness_list_names_aylul() {
    let w = world("PATH").await;
    let aylul = aylul_entry(&w).await;
    assert_eq!(aylul["label"], "AYLUL");
    assert_eq!(aylul["available"], true);
}

async fn aylul_entry(w: &World) -> Value {
    let list = serde_json::to_value(w.core.harness().list().await.unwrap()).unwrap();
    list.as_array()
        .unwrap()
        .iter()
        .find(|h| h["kind"] == "aylul")
        .unwrap()
        .clone()
}
