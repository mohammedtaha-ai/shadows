use shadows_core::OperationId;
use shadows_core::runtime::RuntimeInstanceId;
use shadows_core::storage::{StopKind, Storage};

async fn seed_operation(
    storage: &Storage,
    op_id: &str,
    runtime_id: &RuntimeInstanceId,
    status: &str,
) {
    let started = if status == "Pending" {
        None
    } else {
        Some("2026-09-21T00:00:00Z")
    };
    storage
        .write_txn(|conn| {
            let (op_id, runtime_id, status) =
                (op_id.to_string(), runtime_id.as_str().to_string(), status.to_string());
            Box::pin(async move {
                sqlx::query(
                    "INSERT INTO operation (id, kind, status_kind, runtime_instance_id, created_at, started_at)
                     VALUES (?, 'PlannerTurn', ?, ?, '2026-09-21T00:00:00Z', ?)",
                )
                .bind(&op_id).bind(&status).bind(&runtime_id).bind(started)
                .execute(&mut *conn).await?;
                Ok(())
            })
        })
        .await
        .unwrap();
}

/// Spec §8.6. A runtime that was lost leaves its operations non-terminal; the
/// next runtime resolves them to Interrupted by exact CAS, and never claims
/// they succeeded, failed, or were cancelled.
#[tokio::test]
async fn a_lost_runtimes_operations_become_interrupted() {
    let tmp = tempfile::tempdir().unwrap();
    let storage = Storage::open(&tmp.path().join("s.sqlite3")).await.unwrap();

    let old = storage
        .register_runtime_instance("0.1.0-old")
        .await
        .unwrap();
    seed_operation(&storage, "op-pending", &old, "Pending").await;
    seed_operation(&storage, "op-running", &old, "Running").await;
    // `old` is never stopped: it was lost.

    let new = storage
        .register_runtime_instance("0.1.0-new")
        .await
        .unwrap();
    let report = storage.reconcile_orphans(&new).await.unwrap();

    assert_eq!(report.interrupted.len(), 2);
    assert!(report.anomalies.is_empty());

    let rows: Vec<(String, String, Option<String>)> =
        sqlx::query_as("SELECT id, status_kind, interrupt_reason FROM operation ORDER BY id")
            .fetch_all(storage.reader())
            .await
            .unwrap();
    assert_eq!(rows[0].1, "Interrupted");
    assert_eq!(
        rows[0].2.as_deref(),
        Some("PreviousRuntimeEndedBeforeStart")
    );
    assert_eq!(rows[1].1, "Interrupted");
    assert_eq!(rows[1].2.as_deref(), Some("PreviousRuntimeEndedDuringRun"));
}

/// Spec §8.5 and §8.6. An Escalated shutdown records `stopped_at` on purpose
/// and leaves work non-terminal. Recovery selects by ownership, not by
/// `stopped_at`, so that work must still be reconciled. This is the bug the
/// review caught; the test is what stops it coming back.
#[tokio::test]
async fn an_escalated_shutdowns_operations_are_not_stranded() {
    let tmp = tempfile::tempdir().unwrap();
    let storage = Storage::open(&tmp.path().join("s.sqlite3")).await.unwrap();

    let old = storage
        .register_runtime_instance("0.1.0-old")
        .await
        .unwrap();
    seed_operation(&storage, "op-abandoned", &old, "Running").await;
    storage
        .stop_runtime_instance(&old, StopKind::Escalated)
        .await
        .unwrap();

    let new = storage
        .register_runtime_instance("0.1.0-new")
        .await
        .unwrap();
    let report = storage.reconcile_orphans(&new).await.unwrap();

    let interrupted: Vec<&str> = report.interrupted.iter().map(|i| i.as_str()).collect();
    assert_eq!(interrupted, vec!["op-abandoned"]);
    let status: String =
        sqlx::query_scalar("SELECT status_kind FROM operation WHERE id='op-abandoned'")
            .fetch_one(storage.reader())
            .await
            .unwrap();
    assert_eq!(status, "Interrupted");
}

/// Spec §8.5: `Graceful` is a claim only written when true. §8.6: finding one
/// that owns a non-terminal Operation is a defect, reconciled and reported,
/// never stranded and never passed over silently.
///
/// Storage refuses to write that row (the next test), so the defect is seeded
/// in raw SQL — which is the point: recovery must still report a row it could
/// only meet through a bug, an older build, or a hand-edited file.
#[tokio::test]
async fn a_graceful_runtime_owning_unfinished_work_is_reported_as_an_anomaly() {
    let tmp = tempfile::tempdir().unwrap();
    let storage = Storage::open(&tmp.path().join("s.sqlite3")).await.unwrap();

    let old = storage
        .register_runtime_instance("0.1.0-old")
        .await
        .unwrap();
    seed_operation(&storage, "op-leaked", &old, "Running").await;
    let old_id = old.as_str().to_string();
    storage
        .write_txn(move |conn| {
            Box::pin(async move {
                sqlx::query(
                    "UPDATE runtime_instance SET stopped_at = '2026-09-21T00:00:01Z',
                            stop_kind = 'Graceful' WHERE id = ?",
                )
                .bind(&old_id)
                .execute(&mut *conn)
                .await?;
                Ok(())
            })
        })
        .await
        .unwrap();

    let new = storage
        .register_runtime_instance("0.1.0-new")
        .await
        .unwrap();
    let report = storage.reconcile_orphans(&new).await.unwrap();

    assert_eq!(
        report.interrupted,
        vec![OperationId::from_literal("op-leaked")]
    );
    let anomalies: Vec<&str> = report.anomalies.iter().map(|i| i.as_str()).collect();
    assert_eq!(anomalies, vec!["op-leaked"]);
}

/// Spec §8.5, enforced where the claim is written: storage refuses `Graceful`
/// for a runtime that owns a non-terminal operation, writes nothing, and still
/// accepts `Escalated`, which claims nothing. Another runtime's unfinished work
/// is not this runtime's to answer for, and does not block its `Graceful`.
#[tokio::test]
async fn graceful_is_refused_while_the_runtime_owns_unfinished_work() {
    let tmp = tempfile::tempdir().unwrap();
    let storage = Storage::open(&tmp.path().join("s.sqlite3")).await.unwrap();

    let busy = storage.register_runtime_instance("0.1.0").await.unwrap();
    let idle = storage.register_runtime_instance("0.1.0").await.unwrap();
    seed_operation(&storage, "op-running", &busy, "Running").await;

    let refused = storage
        .stop_runtime_instance(&busy, StopKind::Graceful)
        .await;
    assert!(
        matches!(
            refused,
            Err(shadows_core::storage::StorageError::TransitionConflict { .. })
        ),
        "Graceful over a Running operation must be refused, got {refused:?}"
    );
    let stopped: Option<String> =
        sqlx::query_scalar("SELECT stopped_at FROM runtime_instance WHERE id = ?")
            .bind(busy.as_str())
            .fetch_one(storage.reader())
            .await
            .unwrap();
    assert_eq!(stopped, None, "a refused stop writes nothing");

    storage
        .stop_runtime_instance(&busy, StopKind::Escalated)
        .await
        .expect("Escalated claims nothing and is never refused for unfinished work");
    storage
        .stop_runtime_instance(&idle, StopKind::Graceful)
        .await
        .expect("`idle` owns no unfinished work; `busy`'s Running row is not its own");
}

/// The current runtime's own live work is never reconciled out from under it.
#[tokio::test]
async fn the_current_runtimes_own_operations_are_left_alone() {
    let tmp = tempfile::tempdir().unwrap();
    let storage = Storage::open(&tmp.path().join("s.sqlite3")).await.unwrap();

    let me = storage.register_runtime_instance("0.1.0").await.unwrap();
    seed_operation(&storage, "op-mine", &me, "Running").await;

    let report = storage.reconcile_orphans(&me).await.unwrap();
    assert!(report.interrupted.is_empty());
    let status: String = sqlx::query_scalar("SELECT status_kind FROM operation WHERE id='op-mine'")
        .fetch_one(storage.reader())
        .await
        .unwrap();
    assert_eq!(status, "Running");
}

#[path = "fixtures/acp.rs"]
mod acp;
#[path = "fixtures/turn.rs"]
mod turn;

/// One daemon's worth of state over the database at `db`: a runtime, its
/// registry and its adapters, as `shadows serve` assembles them.
struct Daemon {
    runtime: std::sync::Arc<shadows_core::runtime::Runtime>,
    handles: std::sync::Arc<shadows_core::testing::LiveHandles>,
    sessions: std::sync::Arc<shadows_core::planner::Sessions>,
}

impl Daemon {
    async fn start(db: &std::path::Path) -> Self {
        let storage = std::sync::Arc::new(Storage::open(db).await.unwrap());
        let (runtime, _report) = shadows_core::runtime::Runtime::start(storage)
            .await
            .unwrap();
        Daemon {
            runtime: std::sync::Arc::new(runtime),
            handles: Default::default(),
            sessions: acp::fake_sessions(db).await,
        }
    }

    /// Runs one turn to its end and answers the text of its last entry.
    async fn turn(&self, thread: &shadows_core::thread::ThreadId, prompt: &str) -> String {
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
            let loaded = self.runtime.storage.get_operation(&op).await.unwrap();
            if loaded.finished_at.is_some() {
                assert_eq!(loaded.status_kind, "Completed", "{loaded:?}");
                let entries = self
                    .runtime
                    .storage
                    .list_thread_entries(thread)
                    .await
                    .unwrap();
                return entries.last().unwrap().body.clone();
            }
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
        panic!("the turn never ended");
    }

    async fn stop(self) {
        let kind = shadows_core::testing::shut_down(
            self.runtime,
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

/// Spec §12.2: the daemon's adapters do not survive it, and the thread's
/// recorded session does. The next daemon, holding no live adapter, opens the
/// thread by resuming that session — which is how a conversation continues
/// across a restart.
#[tokio::test]
async fn after_a_restart_the_next_turn_resumes_the_recorded_session() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("s.sqlite3");
    let first = Daemon::start(&db).await;
    let ctx = |id: &str, kind: &str| shadows_core::command::CommandContext {
        principal_kind: "User".into(),
        principal_id: "local".into(),
        command_id: id.into(),
        command_kind: kind.into(),
        command_schema_ver: 1,
        request_fingerprint: shadows_core::command::fingerprint(
            kind,
            &serde_json::json!({ "id": id }),
        ),
    };
    let dir = shadows_core::project::ProjectDirectory::resolve(tmp.path()).unwrap();
    let storage = &first.runtime.storage;
    let project = storage
        .create_project(
            &ctx("c1", "project.create"),
            "demo",
            "Demo",
            &dir,
            &shadows_agent::policy::default_modes(),
        )
        .await
        .unwrap();
    let thread = storage
        .create_planning_thread(&ctx("c2", "thread.create"), &project.id, "T", "claude-code")
        .await
        .unwrap()
        .id;

    first.turn(&thread, "hi").await;
    let session = storage
        .turn_context(&thread)
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
