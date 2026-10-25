use shadows_core::{StopKind, testing::Storage};

use shadows_core::testing::acp;
use shadows_core::testing::turn;

/// One daemon's worth of state over the database at `db`: a runtime, its
/// registry and its adapters, as `shadows serve` assembles them.
struct Daemon {
    runtime: std::sync::Arc<shadows_core::testing::Runtime>,
    handles: std::sync::Arc<shadows_core::testing::LiveHandles>,
    sessions: std::sync::Arc<shadows_core::testing::Harness>,
}

impl Daemon {
    async fn start(db: &std::path::Path) -> Self {
        let storage = std::sync::Arc::new(Storage::open(db).await.unwrap());
        let (runtime, _report) = shadows_core::AppCore::runtime_for_tests(storage.clone())
            .await
            .unwrap();
        Daemon {
            runtime: std::sync::Arc::new(runtime),
            handles: Default::default(),
            sessions: acp::fake_sessions(storage),
        }
    }

    /// Runs one turn to its end and answers the text of its last entry.
    async fn turn(&self, thread: &shadows_core::ThreadId, prompt: &str) -> String {
        let bus = tokio::sync::broadcast::channel(16).0;
        let op = turn::start_direct(
            &self.runtime,
            &self.handles,
            &self.sessions,
            &bus,
            thread,
            prompt,
        )
        .await
        .unwrap();
        for _ in 0..200 {
            let loaded =
                shadows_core::Turns::get_operation_for_test(&self.runtime.storage_for_test(), &op)
                    .await
                    .unwrap();
            if loaded.finished_at.is_some() {
                assert_eq!(loaded.status_kind, "Completed", "{loaded:?}");
                let entries = shadows_core::Threads::list_thread_entries_for_test(
                    &self.runtime.storage_for_test(),
                    thread,
                )
                .await
                .unwrap();
                return entries.last().unwrap().body.clone();
            }
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
        panic!("the turn never ended");
    }

    async fn stop(self) {
        let kind = shadows_core::testing::PlannerTurn::shut_down(
            self.runtime.clone(),
            self.runtime.storage_for_test(),
            shadows_core::AppCore::operations_for_tests(self.runtime.storage_for_test()),
            self.handles,
            self.sessions.clone(),
            std::time::Duration::from_secs(10),
            std::future::pending(),
        )
        .await
        .unwrap();
        assert_eq!(kind, StopKind::Graceful);
        assert_eq!(
            self.sessions.live_count().await,
            0,
            "an adapter outlived the daemon"
        );
    }
}

/// The daemon's adapters do not survive it, and the thread's
/// recorded session does. The next daemon, holding no live adapter, opens the
/// thread by resuming that session — which is how a conversation continues
/// across a restart.
#[tokio::test]
async fn after_a_restart_the_next_turn_resumes_the_recorded_session() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("s.sqlite3");
    let first = Daemon::start(&db).await;
    let ctx = |id: &str, kind: &str| shadows_core::testing::CommandContext {
        agent: None,
        principal_kind: "User".into(),
        principal_id: "local".into(),
        command_id: id.into(),
        command_kind: kind.into(),
        command_schema_ver: 1,
        request_fingerprint: shadows_core::testing::CommandContext::fingerprint(
            kind,
            &serde_json::json!({ "id": id }),
        ),
    };
    let dir = shadows_core::testing::ProjectDirectory::resolve(tmp.path()).unwrap();
    let storage = &first.runtime.storage_for_test();
    let project = shadows_core::testing::create_project(
        storage,
        &ctx("c1", "project.create"),
        "demo",
        "Demo",
        &dir,
        &shadows_agent::policy::default_modes(),
    )
    .await
    .unwrap();
    let thread = shadows_core::Threads::create_planning_thread_for_test(
        storage,
        &ctx("c2", "thread.create"),
        &project.id,
        "T",
        "claude-code",
        None,
    )
    .await
    .unwrap()
    .id;

    first.turn(&thread, "hi").await;
    let session = shadows_core::Threads::turn_context_for_test(storage, &thread)
        .await
        .unwrap()
        .harness_session_id
        .expect("the first turn records its session");
    first.stop().await;

    let second = Daemon::start(&db).await;
    let report: serde_json::Value =
        serde_json::from_str(&second.turn(&thread, "report").await).unwrap();
    assert_eq!(
        (report["how"].as_str(), report["session"].as_str()),
        (Some("resume"), Some(session.as_str()))
    );
    second.stop().await;
}
