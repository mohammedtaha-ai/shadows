use std::{path::PathBuf, sync::Arc, time::Duration};

use shadows::{
    agent::{claude::ClaudeAdapter, events::HarnessEvent},
    command::{CommandContext, fingerprint},
    planner::{OpenError, OpenSession, Sessions, SessionsConfig},
    project::ProjectDirectory,
    storage::Storage,
    thread::ThreadId,
};

struct Fixture {
    _tmp: tempfile::TempDir,
    project_dir: PathBuf,
    storage: Storage,
    sessions: Arc<Sessions>,
    thread: ThreadId,
}

async fn fixture(config: SessionsConfig) -> Fixture {
    let tmp = tempfile::tempdir().unwrap();
    let project_dir = tmp.path().join("project");
    std::fs::create_dir(&project_dir).unwrap();
    let storage = Storage::open(&tmp.path().join("s.sqlite3")).await.unwrap();
    let params = serde_json::json!({ "slug": "demo" });
    let ctx = CommandContext {
        principal_kind: "User".into(),
        principal_id: "local".into(),
        command_id: "c1".into(),
        command_kind: "project.create".into(),
        command_schema_ver: 1,
        request_fingerprint: fingerprint("project.create", &params),
    };
    let project = storage
        .create_project(
            &ctx,
            "demo",
            "Demo",
            &ProjectDirectory::resolve(&project_dir).unwrap(),
        )
        .await
        .unwrap();
    let tctx = CommandContext {
        command_id: "c2".into(),
        command_kind: "thread.create".into(),
        request_fingerprint: fingerprint("thread.create", &params),
        ..ctx
    };
    let thread = storage
        .create_planning_thread(&tctx, &project.id, "T")
        .await
        .unwrap()
        .id;
    let adapter = Arc::new(ClaudeAdapter {
        node: PathBuf::from(env!("CARGO_BIN_EXE_fake_acp")),
        adapter: PathBuf::from("unused"),
        agent: PathBuf::from("unused"),
        adapter_version: "fake".into(),
        agent_version: "fake".into(),
    });
    let stored = Storage::open(&tmp.path().join("s.sqlite3")).await.unwrap();
    let sessions = Sessions::new(adapter, stored, config);
    Fixture {
        _tmp: tmp,
        project_dir,
        storage,
        sessions,
        thread,
    }
}

async fn prompt_text(fx: &Fixture, s: &OpenSession, text: &str) -> String {
    let mut events = fx.sessions.take_events(&fx.thread).await.unwrap();
    s.connection().prompt(&s.session_id, text).await.unwrap();
    let mut result = String::new();
    while let Ok(event) = events.try_recv() {
        if let HarnessEvent::Chunk { text, .. } = event {
            result.push_str(&text);
        }
    }
    fx.sessions.give_back_events(&fx.thread, events).await;
    result
}

#[tokio::test]
async fn opening_twice_reuses_one_adapter_and_starts_in_accept_edits() {
    let fx = fixture(SessionsConfig::default()).await;
    let a = fx.sessions.open(&fx.thread).await.unwrap();
    let b = fx.sessions.open(&fx.thread).await.unwrap();
    assert_eq!(a.session_id, b.session_id);
    assert_eq!(fx.sessions.live_count().await, 1);
    let report: serde_json::Value =
        serde_json::from_str(&prompt_text(&fx, &a, "report").await).unwrap();
    assert_eq!(report["mode"], "acceptEdits");
    assert_eq!(report["how"], "new");
    fx.sessions.close_all().await.unwrap();
}

#[tokio::test]
async fn a_thread_with_a_recorded_session_resumes_it() {
    let fx = fixture(SessionsConfig::default()).await;
    fx.storage
        .record_harness_session(&fx.thread, "fake-77")
        .await
        .unwrap();
    let s = fx.sessions.open(&fx.thread).await.unwrap();
    assert_eq!((s.session_id.as_str(), s.how), ("fake-77", "resume"));
    fx.sessions.close_all().await.unwrap();
}

#[tokio::test]
async fn an_idle_connection_is_closed_and_the_next_opening_resumes() {
    let fx = fixture(SessionsConfig {
        idle_after: Duration::from_millis(300),
        ..Default::default()
    })
    .await;
    fx.storage
        .record_harness_session(&fx.thread, "fake-5")
        .await
        .unwrap();
    fx.sessions.open(&fx.thread).await.unwrap();
    tokio::time::sleep(Duration::from_millis(1500)).await;
    assert_eq!(fx.sessions.live_count().await, 0);
    let again = fx.sessions.open(&fx.thread).await.unwrap();
    assert_eq!((again.session_id.as_str(), again.how), ("fake-5", "resume"));
    fx.sessions.close_all().await.unwrap();
}

#[tokio::test]
async fn idle_close_before_first_turn_discards_the_unrecorded_session() {
    let fx = fixture(SessionsConfig {
        idle_after: Duration::from_millis(300),
        ..Default::default()
    })
    .await;
    let first = fx.sessions.open(&fx.thread).await.unwrap();
    assert_eq!(first.how, "new");
    assert!(
        fx.storage
            .turn_context(&fx.thread)
            .await
            .unwrap()
            .harness_session_id
            .is_none()
    );
    tokio::time::timeout(Duration::from_secs(3), async {
        while fx.sessions.live_count().await != 0 {
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    })
    .await
    .expect("idle reaper should close the adapter");
    let again = fx.sessions.open(&fx.thread).await.unwrap();
    assert_eq!(again.how, "new");
    assert_eq!(prompt_text(&fx, &again, "hi").await, "hello from fake_acp");
    fx.sessions.close_all().await.unwrap();
}

#[tokio::test]
async fn a_dead_connection_is_replaced_on_the_next_opening() {
    let fx = fixture(SessionsConfig::default()).await;
    let s = fx.sessions.open(&fx.thread).await.unwrap();
    let _ = s.connection().prompt(&s.session_id, "exit").await;
    let again = fx.sessions.open(&fx.thread).await.unwrap();
    assert_eq!(prompt_text(&fx, &again, "hi").await, "hello from fake_acp");
    fx.sessions.close_all().await.unwrap();
}

#[tokio::test]
async fn a_project_without_its_directory_does_not_start_an_adapter() {
    let fx = fixture(SessionsConfig::default()).await;
    std::fs::remove_dir_all(&fx.project_dir).unwrap();
    assert!(matches!(
        fx.sessions.open(&fx.thread).await,
        Err(OpenError::Start(_))
    ));
    assert_eq!(fx.sessions.live_count().await, 0);
}

#[tokio::test]
async fn reaper_keeps_a_connection_while_its_turn_holds_events() {
    let fx = fixture(SessionsConfig {
        idle_after: Duration::from_millis(200),
        ..Default::default()
    })
    .await;
    fx.sessions.open(&fx.thread).await.unwrap();
    let events = fx.sessions.take_events(&fx.thread).await.unwrap();
    tokio::time::sleep(Duration::from_millis(600)).await;
    assert_eq!(fx.sessions.live_count().await, 1);
    fx.sessions.give_back_events(&fx.thread, events).await;
    fx.sessions.close_all().await.unwrap();
}

#[tokio::test]
async fn terminate_removes_the_connection() {
    let fx = fixture(SessionsConfig::default()).await;
    fx.sessions.open(&fx.thread).await.unwrap();
    fx.sessions.terminate(&fx.thread).await.unwrap();
    assert_eq!(fx.sessions.live_count().await, 0);
}
