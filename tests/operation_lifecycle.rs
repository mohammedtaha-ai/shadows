use shadows::operation::FailureStage;
use shadows::runtime::RuntimeInstanceId;
use shadows::storage::{Storage, StorageError};

async fn fixture() -> (tempfile::TempDir, Storage, RuntimeInstanceId, String) {
    let tmp = tempfile::tempdir().unwrap();
    let storage = Storage::open(&tmp.path().join("s.sqlite3")).await.unwrap();
    let runtime = storage.register_runtime_instance("test").await.unwrap();
    let params = serde_json::json!({ "slug": "demo" });
    let ctx = shadows::command::CommandContext {
        principal_kind: "User".into(),
        principal_id: "local".into(),
        command_id: "c1".into(),
        command_kind: "project.create".into(),
        command_schema_ver: 1,
        request_fingerprint: shadows::command::fingerprint("project.create", &params),
    };
    let project = storage.create_project(&ctx, "demo", "Demo").await.unwrap();
    let tctx = shadows::command::CommandContext {
        command_id: "c2".into(),
        command_kind: "thread.create".into(),
        request_fingerprint: shadows::command::fingerprint("thread.create", &params),
        ..ctx
    };
    let thread = storage
        .create_planning_thread(&tctx, &project.id, "T")
        .await
        .unwrap();
    (tmp, storage, runtime, thread.id)
}

/// Spec §2.7. TX #1 persists Pending before anything spawns. A Pending
/// Operation is a durable attempt that has not yet claimed a running process.
#[tokio::test]
async fn phase_one_persists_pending_before_anything_spawns() {
    let (_t, storage, runtime, thread) = fixture().await;
    let op = storage
        .create_pending_operation(&thread, &runtime)
        .await
        .unwrap();

    let loaded = storage.get_operation(&op).await.unwrap();
    assert_eq!(loaded.status_kind, "Pending");
    assert!(
        loaded.started_at.is_none(),
        "Pending must not claim a start time"
    );
    assert!(loaded.finished_at.is_none());
}

/// Spec §2.7 and §8.3. The Pending -> Running transition is an exact CAS. It
/// must not fire against an operation owned by a different runtime, and it must
/// not fire twice.
#[tokio::test]
async fn the_running_transition_is_an_exact_compare_and_swap() {
    let (_t, storage, runtime, thread) = fixture().await;
    let op = storage
        .create_pending_operation(&thread, &runtime)
        .await
        .unwrap();

    storage.mark_operation_started(&op, &runtime).await.unwrap();
    assert_eq!(
        storage.get_operation(&op).await.unwrap().status_kind,
        "Running"
    );

    let again = storage.mark_operation_started(&op, &runtime).await;
    assert!(
        matches!(again, Err(StorageError::TransitionConflict { .. })),
        "a second Running transition must be refused"
    );
    assert_eq!(
        storage.get_operation(&op).await.unwrap().status_kind,
        "Running",
        "the refused second transition must leave the stored row unchanged"
    );

    let other_runtime = storage.register_runtime_instance("other").await.unwrap();
    let op2 = storage
        .create_pending_operation(&thread, &runtime)
        .await
        .unwrap();
    let wrong_owner = storage.mark_operation_started(&op2, &other_runtime).await;
    assert!(
        matches!(wrong_owner, Err(StorageError::TransitionConflict { .. })),
        "a runtime must not start an operation it does not own"
    );
    assert_eq!(
        storage.get_operation(&op2).await.unwrap().status_kind,
        "Pending",
        "the refused wrong-owner transition must leave the stored row unchanged"
    );
}

/// Spec §8.3. Prepare failure is not spawn failure: the process never existed.
/// Keeping them distinct is what makes a diagnosis possible later.
#[tokio::test]
async fn prepare_failure_and_spawn_failure_are_distinguishable() {
    let (_t, storage, runtime, thread) = fixture().await;

    let prepared = storage
        .create_pending_operation(&thread, &runtime)
        .await
        .unwrap();
    storage
        .mark_operation_failed(
            &prepared,
            FailureStage::Prepare,
            "harness executable not found",
        )
        .await
        .unwrap();
    let a = storage.get_operation(&prepared).await.unwrap();
    assert_eq!(a.status_kind, "Failed");
    assert_eq!(a.failure_stage.as_deref(), Some("Prepare"));
    assert!(a.started_at.is_none(), "nothing ever started");

    let spawned = storage
        .create_pending_operation(&thread, &runtime)
        .await
        .unwrap();
    storage
        .mark_operation_failed(
            &spawned,
            FailureStage::Spawn,
            "os refused to start the process",
        )
        .await
        .unwrap();
    let b = storage.get_operation(&spawned).await.unwrap();
    assert_eq!(b.failure_stage.as_deref(), Some("Spawn"));
}

/// Spec §8.6: terminal states never transition again. A retry creates a new
/// Operation; it does not revive an old one.
#[tokio::test]
async fn a_terminal_operation_never_transitions_again() {
    let (_t, storage, runtime, thread) = fixture().await;
    let op = storage
        .create_pending_operation(&thread, &runtime)
        .await
        .unwrap();
    storage.mark_operation_started(&op, &runtime).await.unwrap();
    storage
        .mark_operation_completed(&op, serde_json::json!({ "ok": true }))
        .await
        .unwrap();

    let err = storage
        .mark_operation_failed(&op, FailureStage::Spawn, "too late")
        .await;
    assert!(matches!(err, Err(StorageError::TransitionConflict { .. })));
    assert_eq!(
        storage.get_operation(&op).await.unwrap().status_kind,
        "Completed",
        "the refused post-terminal transition must leave the stored row unchanged"
    );
}

/// Every transition appends its durable event in the same transaction.
/// Cross-cutting rule 5. Ordered by the explicit durable sequence, never by
/// `rowid` or insertion order (CLAUDE.md).
#[tokio::test]
async fn every_transition_appends_its_event_atomically() {
    let (_t, storage, runtime, thread) = fixture().await;
    let before: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM durable_event")
        .fetch_one(storage.reader())
        .await
        .unwrap();

    let op = storage
        .create_pending_operation(&thread, &runtime)
        .await
        .unwrap();
    storage.mark_operation_started(&op, &runtime).await.unwrap();
    storage
        .mark_operation_completed(&op, serde_json::json!({}))
        .await
        .unwrap();

    let kinds: Vec<String> =
        sqlx::query_scalar("SELECT kind FROM durable_event WHERE operation_id = ? ORDER BY seq")
            .bind(op.as_str())
            .fetch_all(storage.reader())
            .await
            .unwrap();
    assert_eq!(
        kinds,
        vec!["OperationCreated", "OperationStarted", "OperationCompleted"]
    );
    let _ = before;
}
