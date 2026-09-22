use shadows::events::Actor;
use shadows::operation::FailureStage;
use shadows::runtime::RuntimeInstanceId;
use shadows::storage::{Storage, StorageError};
use shadows::thread::ThreadId;

async fn fixture() -> (tempfile::TempDir, Storage, RuntimeInstanceId, ThreadId) {
    let tmp = tempfile::tempdir().unwrap();
    let storage = Storage::open(&tmp.path().join("s.sqlite3")).await.unwrap();
    let runtime = storage.register_runtime_instance("test").await.unwrap();
    let dir = shadows::project::ProjectDirectory::resolve(tmp.path()).unwrap();
    let params = serde_json::json!({ "slug": "demo" });
    let ctx = shadows::command::CommandContext {
        principal_kind: "User".into(),
        principal_id: "local".into(),
        command_id: "c1".into(),
        command_kind: "project.create".into(),
        command_schema_ver: 1,
        request_fingerprint: shadows::command::fingerprint("project.create", &params),
    };
    let project = storage
        .create_project(&ctx, "demo", "Demo", &dir)
        .await
        .unwrap();
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

    // The `before` count was captured and then discarded. Scoping the assertion
    // to this operation's own rows cannot see an event written against no
    // operation at all, or against the wrong one — both of which leave `kinds`
    // exactly right while the journal has grown by more than these three.
    let after: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM durable_event")
        .fetch_one(storage.reader())
        .await
        .unwrap();
    assert_eq!(
        after,
        before + 3,
        "three transitions must append exactly three events to the whole journal"
    );
}

/// Spec §2.3. The request and the terminal state are two separate facts in two
/// separate transactions. A request alone leaves the operation non-terminal.
#[tokio::test]
async fn a_cancellation_request_does_not_make_an_operation_terminal() {
    let (_t, storage, runtime, thread) = fixture().await;
    let op = storage
        .create_pending_operation(&thread, &runtime)
        .await
        .unwrap();
    storage.mark_operation_started(&op, &runtime).await.unwrap();

    storage
        .request_cancellation(&op, Actor::user("local"))
        .await
        .unwrap();

    let loaded = storage.get_operation(&op).await.unwrap();
    assert_eq!(
        loaded.status_kind, "Running",
        "a request is not a terminal state"
    );
    assert!(loaded.cancel_requested_at.is_some());
    assert!(loaded.finished_at.is_none());
}

/// Spec §2.3. Terminal Cancelled is written only after termination is
/// confirmed, and it is what closes the operation.
#[tokio::test]
async fn cancelled_is_written_after_confirmation_and_closes_the_operation() {
    let (_t, storage, runtime, thread) = fixture().await;
    let op = storage
        .create_pending_operation(&thread, &runtime)
        .await
        .unwrap();
    storage.mark_operation_started(&op, &runtime).await.unwrap();
    storage
        .request_cancellation(&op, Actor::user("local"))
        .await
        .unwrap();

    storage.mark_operation_cancelled(&op).await.unwrap();

    let loaded = storage.get_operation(&op).await.unwrap();
    assert_eq!(loaded.status_kind, "Cancelled");
    assert!(loaded.finished_at.is_some());
    assert!(
        loaded.cancel_requested_at.is_some(),
        "the request is retained as history"
    );

    let kinds: Vec<String> =
        sqlx::query_scalar("SELECT kind FROM durable_event WHERE operation_id = ? ORDER BY seq")
            .bind(op.as_str())
            .fetch_all(storage.reader())
            .await
            .unwrap();
    assert_eq!(
        kinds,
        vec![
            "OperationCreated",
            "OperationStarted",
            "OperationCancellationRequested",
            "OperationCancelled"
        ]
    );
}

/// Spec §8.4 case 4. A process that exited on its own before cancellation took
/// termination ownership is Completed by its own exit, not Cancelled. Shadows
/// does not claim to have stopped something that had already stopped.
#[tokio::test]
async fn a_natural_exit_wins_over_an_in_flight_cancellation() {
    let (_t, storage, runtime, thread) = fixture().await;
    let op = storage
        .create_pending_operation(&thread, &runtime)
        .await
        .unwrap();
    storage.mark_operation_started(&op, &runtime).await.unwrap();
    storage
        .request_cancellation(&op, Actor::user("local"))
        .await
        .unwrap();

    // The process exits before containment takes ownership.
    storage
        .mark_operation_completed(&op, serde_json::json!({ "ok": true }))
        .await
        .unwrap();

    let loaded = storage.get_operation(&op).await.unwrap();
    assert_eq!(loaded.status_kind, "Completed");
    assert!(
        loaded.cancel_requested_at.is_some(),
        "a terminal non-cancelled status may retain request metadata as history"
    );

    // Spec §8.4 case 5 in reverse: the late Cancelled must not overwrite it.
    let late = storage.mark_operation_cancelled(&op).await;
    assert!(matches!(late, Err(StorageError::TransitionConflict { .. })));
}

/// Spec §8.4 case 6. If termination cannot be confirmed, the request is
/// preserved and Cancelled is NOT written. The operation stays non-terminal
/// until recovery can make an honest Interrupted transition.
#[tokio::test]
async fn unconfirmed_termination_leaves_the_operation_non_terminal() {
    let (_t, storage, runtime, thread) = fixture().await;
    let op = storage
        .create_pending_operation(&thread, &runtime)
        .await
        .unwrap();
    storage.mark_operation_started(&op, &runtime).await.unwrap();
    storage
        .request_cancellation(&op, Actor::user("local"))
        .await
        .unwrap();

    // Termination failed: the daemon simply does not call mark_operation_cancelled.
    let loaded = storage.get_operation(&op).await.unwrap();
    assert_eq!(loaded.status_kind, "Running");

    // A later runtime resolves it honestly, and to Interrupted, not Cancelled,
    // because the final process outcome is unknown after a crash.
    let next = storage.register_runtime_instance("next").await.unwrap();
    let report = storage.reconcile_orphans(&next).await.unwrap();
    assert_eq!(report.interrupted, vec![op.clone()]);
    let after = storage.get_operation(&op).await.unwrap();
    assert_eq!(after.status_kind, "Interrupted");
    assert!(
        after.cancel_requested_at.is_some(),
        "the request stays visible on the interrupted record"
    );
}

/// Spec §6.18 scope semantics. Every operation event carries its operation's
/// thread, not only its operation — otherwise the thread cursor a client
/// follows (§2.10) never selects it. All seven operation event kinds are
/// produced here, across four operations, and every one must name the thread.
#[tokio::test]
async fn every_operation_event_is_scoped_to_its_thread() {
    let (_t, storage, runtime, thread) = fixture().await;

    let completed = storage
        .create_pending_operation(&thread, &runtime)
        .await
        .unwrap();
    storage
        .mark_operation_started(&completed, &runtime)
        .await
        .unwrap();
    storage
        .mark_operation_completed(&completed, serde_json::json!({}))
        .await
        .unwrap();

    let failed = storage
        .create_pending_operation(&thread, &runtime)
        .await
        .unwrap();
    storage
        .mark_operation_failed(&failed, FailureStage::Spawn, "no such file")
        .await
        .unwrap();

    let cancelled = storage
        .create_pending_operation(&thread, &runtime)
        .await
        .unwrap();
    storage
        .request_cancellation(&cancelled, Actor::user("local"))
        .await
        .unwrap();
    storage.mark_operation_cancelled(&cancelled).await.unwrap();

    let interrupted = storage
        .create_pending_operation(&thread, &runtime)
        .await
        .unwrap();
    storage
        .mark_operation_started(&interrupted, &runtime)
        .await
        .unwrap();
    let next = storage.register_runtime_instance("next").await.unwrap();
    storage.reconcile_orphans(&next).await.unwrap();

    let rows: Vec<(String, Option<String>)> = sqlx::query_as(
        "SELECT kind, thread_id FROM durable_event WHERE operation_id IS NOT NULL ORDER BY seq",
    )
    .fetch_all(storage.reader())
    .await
    .unwrap();
    let mut kinds: Vec<&str> = rows.iter().map(|(k, _)| k.as_str()).collect();
    kinds.sort();
    kinds.dedup();
    assert_eq!(
        kinds,
        vec![
            "OperationCancellationRequested",
            "OperationCancelled",
            "OperationCompleted",
            "OperationCreated",
            "OperationFailed",
            "OperationInterrupted",
            "OperationStarted",
        ],
        "every operation event kind was produced"
    );
    for (kind, thread_id) in &rows {
        assert_eq!(
            thread_id.as_deref(),
            Some(thread.as_str()),
            "{kind} must carry its operation's thread"
        );
    }
}

/// Migration 0002 backfills the thread onto operation events written before
/// the write path carried it. Reproduced by clearing the column on existing
/// rows and forgetting that 0002 ran, so the next open applies it again.
#[tokio::test]
async fn migration_backfills_the_thread_onto_existing_operation_events() {
    let (tmp, storage, runtime, thread) = fixture().await;
    let op = storage
        .create_pending_operation(&thread, &runtime)
        .await
        .unwrap();
    storage.mark_operation_started(&op, &runtime).await.unwrap();
    sqlx::query("UPDATE durable_event SET thread_id = NULL WHERE operation_id IS NOT NULL")
        .execute(storage.reader())
        .await
        .unwrap();
    sqlx::query("DELETE FROM _sqlx_migrations WHERE version = 2")
        .execute(storage.reader())
        .await
        .unwrap();
    drop(storage);

    let storage = Storage::open(&tmp.path().join("s.sqlite3")).await.unwrap();
    let threads: Vec<Option<String>> = sqlx::query_scalar(
        "SELECT thread_id FROM durable_event WHERE operation_id = ? ORDER BY seq",
    )
    .bind(op.as_str())
    .fetch_all(storage.reader())
    .await
    .unwrap();
    assert_eq!(threads.len(), 2, "created and started");
    assert!(
        threads
            .iter()
            .all(|t| t.as_deref() == Some(thread.as_str())),
        "every existing operation event was backfilled: {threads:?}"
    );
}
