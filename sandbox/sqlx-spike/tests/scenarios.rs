//! SeaORM spike — scenario tests against SQLite (in-memory).

use shadows_domain::{
    CommandId, CommandRecord, DurableEvent, EventKind, Operation, OperationId, OperationKind,
    OperationOutcome, OperationStatus, Principal, Project, ProjectId, ResearchArtifact, ResearchId,
    RuntimeInstanceId, SearchQuery, SearchScope, TaskId, ThreadId, Timestamp, WorkflowId,
};
use shadows_sqlx_spike::ports::{
    AtomicCommandTx, EventStore, MultiEntityTx, MultiOp, OperationStore, ProjectStore,
    ResearchStore, SearchIndex,
};
use shadows_sqlx_spike::storage::sqlite::SqliteBackend;

async fn fresh() -> SqliteBackend {
    let b = SqliteBackend::connect_in_memory().await.expect("connect");
    b.run_migrations().await.expect("migrations");
    b
}

async fn fresh_concurrency_test() -> SqliteBackend {
    // File-based with WAL: demonstrates real concurrent SQLite behavior.
    // In-memory SQLite deadlocks under write contention — that is itself a
    // finding (see report).
    let dir = std::env::temp_dir().join(format!("shadows-sqlx-spike-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).expect("mkdir");
    let path = dir.join("concurrency.db");
    SqliteBackend::connect_file(path.to_str().unwrap())
        .await
        .expect("connect file")
}

fn build_op_event(
    kind: OperationKind,
    status: OperationStatus,
) -> impl FnOnce(
    OperationId,
) -> std::pin::Pin<
    Box<
        dyn std::future::Future<
                Output = Result<(Operation, DurableEvent), shadows_sqlx_spike::ports::DomainError>,
            > + Send,
    >,
> {
    move |assigned| {
        Box::pin(async move {
            let op = Operation {
                id: assigned,
                kind,
                status,
                thread_id: Some(ThreadId::new()),
                workflow_id: None,
                task_id: Some(TaskId::new()),
                runtime_instance_id: RuntimeInstanceId::new(),
                created_at: Timestamp::now_utc(),
                durable_seq: 0,
                outcome: Some(OperationOutcome::Success),
            };
            let evt = DurableEvent {
                id: shadows_domain::EventId::new(),
                durable_seq: 0,
                kind: EventKind::OperationStarted,
                occurred_at: Timestamp::now_utc(),
                operation_id: Some(assigned),
                payload: serde_json::json!({"k":"v"}),
            };
            Ok((op, evt))
        })
    }
}

#[tokio::test]
async fn s1_atomic_command_happy_path() {
    let b = fresh().await;
    let cmd_id = CommandId::new();
    let principal = Principal::new();
    let result = AtomicCommandTx::run(
        &b,
        cmd_id,
        principal,
        "scope:test".into(),
        build_op_event(OperationKind::ExecutionRun, OperationStatus::Running),
    )
    .await
    .expect("atomic run");
    assert_eq!(result.command_id, cmd_id);

    let events = EventStore::read_after(&b, 0, 10)
        .await
        .expect("read events");
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].kind, EventKind::OperationStarted);
}

#[tokio::test]
async fn s1_atomic_command_idempotent_same_id() {
    let b = fresh().await;
    let cmd_id = CommandId::new();
    let principal = Principal::new();
    let r1 = AtomicCommandTx::run(
        &b,
        cmd_id,
        principal,
        "s".into(),
        build_op_event(OperationKind::PlannerTurn, OperationStatus::Running),
    )
    .await
    .expect("first");
    let r2 = AtomicCommandTx::run(
        &b,
        cmd_id,
        principal,
        "s".into(),
        build_op_event(OperationKind::PlannerTurn, OperationStatus::Running),
    )
    .await
    .expect("second");
    assert_eq!(r1.command_id, r2.command_id);
    let events = EventStore::read_after(&b, 0, 100).await.expect("events");
    assert_eq!(events.len(), 1, "exactly one event for idempotent command");
}

#[tokio::test]
async fn s1_atomic_command_failure_before_commit() {
    let b = fresh().await;
    let cmd_id = CommandId::new();
    let principal = Principal::new();
    let res = AtomicCommandTx::run(&b, cmd_id, principal, "s".into(), |_op_id| {
        Box::pin(async move {
            Err(shadows_sqlx_spike::ports::DomainError::Invalid(
                "nope".into(),
            ))
        })
    })
    .await;
    assert!(res.is_err());
    let events = EventStore::read_after(&b, 0, 10).await.expect("read");
    assert_eq!(events.len(), 0);
}

#[tokio::test]
async fn s2_durable_event_ordering() {
    let b = fresh().await;
    let e1 = DurableEvent {
        id: shadows_domain::EventId::new(),
        durable_seq: 0,
        kind: EventKind::OperationStarted,
        occurred_at: Timestamp::now_utc(),
        operation_id: None,
        payload: serde_json::json!({"i": 1}),
    };
    let e2 = DurableEvent {
        id: shadows_domain::EventId::new(),
        durable_seq: 0,
        kind: EventKind::OperationCompleted,
        occurred_at: Timestamp::now_utc(),
        operation_id: None,
        payload: serde_json::json!({"i": 2}),
    };
    EventStore::append(&b, e1).await.expect("append 1");
    EventStore::append(&b, e2).await.expect("append 2");
    let read = EventStore::read_after(&b, 0, 10).await.expect("read");
    assert_eq!(read.len(), 2);
    assert!(
        read[0].durable_seq < read[1].durable_seq,
        "ordering by seq only"
    );
    let after = EventStore::read_after(&b, read[0].durable_seq, 10)
        .await
        .expect("after");
    assert_eq!(after.len(), 1);
    assert_eq!(after[0].kind, EventKind::OperationCompleted);
}

#[tokio::test]
async fn s3_search_research() {
    let b = fresh().await;
    let pid = ProjectId::new();
    ProjectStore::insert(
        &b,
        Project {
            id: pid,
            name: "p".into(),
            created_at: Timestamp::now_utc(),
        },
    )
    .await
    .expect("insert project");

    let r1 = ResearchArtifact {
        id: ResearchId::new(),
        project_id: pid,
        title: "Rust async runtime".into(),
        source: None,
        summary: "Tokio is the async runtime used here.".into(),
        created_at: Timestamp::now_utc(),
    };
    let r2 = ResearchArtifact {
        id: ResearchId::new(),
        project_id: pid,
        title: "Postgres tuning".into(),
        source: None,
        summary: "Some random postgres notes here.".into(),
        created_at: Timestamp::now_utc(),
    };
    ResearchStore::insert(&b, r1).await.expect("insert 1");
    ResearchStore::insert(&b, r2).await.expect("insert 2");

    let hits = SearchIndex::search(
        &b,
        SearchQuery {
            text: "tokio".into(),
            scope: SearchScope::Research,
            limit: 5,
        },
    )
    .await
    .expect("search");
    assert!(!hits.is_empty(), "expected at least one hit for 'tokio'");
}

#[tokio::test]
async fn s5_multi_entity_tx() {
    let b = fresh().await;
    let op_id = OperationId::new();
    let ops = vec![
        MultiOp::InsertOperation(Operation {
            id: op_id,
            kind: OperationKind::ExecutionRun,
            status: OperationStatus::Running,
            thread_id: None,
            workflow_id: None,
            task_id: None,
            runtime_instance_id: RuntimeInstanceId::new(),
            created_at: Timestamp::now_utc(),
            durable_seq: 1,
            outcome: None,
        }),
        MultiOp::AppendEvent(DurableEvent {
            id: shadows_domain::EventId::new(),
            durable_seq: 1,
            kind: EventKind::OperationStarted,
            occurred_at: Timestamp::now_utc(),
            operation_id: Some(op_id),
            payload: serde_json::json!({}),
        }),
        MultiOp::InsertCommand(CommandRecord {
            command_id: CommandId::new(),
            principal: Principal::new(),
            scope: "s".into(),
            operation_id: op_id,
            recorded_at: Timestamp::now_utc(),
        }),
        MultiOp::UpdateWorkflow(WorkflowId::new(), "open".into()),
    ];
    MultiEntityTx::run(&b, ops).await.expect("multi-tx");
    let read_events = EventStore::read_after(&b, 0, 10)
        .await
        .expect("read events");
    assert_eq!(read_events.len(), 1);
}

#[tokio::test]
async fn s6_domain_isolation() {
    let path = "E:/Globalprojects/shadows/sandbox/shared-domain/src";
    for entry in std::fs::read_dir(path).expect("read_dir") {
        let p = entry.unwrap().path();
        if p.extension().and_then(|s| s.to_str()) != Some("rs") {
            continue;
        }
        let content = std::fs::read_to_string(&p).expect("read");
        // Scan for actual `use ... :: ... ;` lines that import a forbidden crate.
        for line in content.lines() {
            let l = line.trim_start();
            if l.starts_with("use ") {
                for forbidden in &[
                    "sea_orm",
                    "sqlx",
                    "sea_query",
                    "rusqlite",
                    "tokio_postgres",
                    "diesel",
                    "libsqlite3_sys",
                ] {
                    assert!(
                        !l.contains(forbidden),
                        "{} contains forbidden '{}' in '{}'",
                        p.display(),
                        forbidden,
                        l
                    );
                }
            }
        }
    }
}

#[tokio::test]
async fn s7_typed_data_roundtrip() {
    let b = fresh().await;
    let op_id = OperationId::new();
    let op = Operation {
        id: op_id,
        kind: OperationKind::Verification,
        status: OperationStatus::Pending,
        thread_id: Some(ThreadId::new()),
        workflow_id: Some(WorkflowId::new()),
        task_id: None,
        runtime_instance_id: RuntimeInstanceId::new(),
        created_at: Timestamp::now_utc(),
        durable_seq: 7,
        outcome: None,
    };
    OperationStore::insert(&b, op.clone())
        .await
        .expect("insert");
    let got = OperationStore::get(&b, op_id)
        .await
        .expect("get")
        .expect("found");
    assert_eq!(got.kind, OperationKind::Verification);
    assert_eq!(got.status, OperationStatus::Pending);
    assert_eq!(got.thread_id, op.thread_id);
    assert_eq!(got.workflow_id, op.workflow_id);
    assert_eq!(got.task_id, None);
    assert_eq!(got.durable_seq, 7);
}

#[tokio::test]
async fn s8_concurrent_atomic_commands() {
    // File-based SQLite avoids the in-memory deadlock.
    // We still expect some deadlocks/busies under contention — that's the
    // real SQLite behavior and worth reporting. We accept up to 2 retries
    // per task via run_with_retry at the storage layer (not in test).
    let b = std::sync::Arc::new(fresh_concurrency_test().await);
    let mut handles = Vec::new();
    for _ in 0..8 {
        let b = b.clone();
        handles.push(tokio::spawn(async move {
            let cmd_id = CommandId::new();
            AtomicCommandTx::run(
                &*b,
                cmd_id,
                Principal::new(),
                "s".into(),
                build_op_event(OperationKind::ExecutionRun, OperationStatus::Running),
            )
            .await
        }));
    }
    let mut successes = 0;
    for h in handles {
        match h.await.unwrap() {
            Ok(_) => successes += 1,
            Err(e) => eprintln!("concurrent task failed: {e}"),
        }
    }
    // Documented: SQLite write contention may lose some tasks.
    assert!(successes >= 1, "at least one task must succeed");
    let evts = EventStore::read_after(&*b, 0, 1000).await.expect("read");
    assert_eq!(evts.len() as u32, successes, "events match successes");
    let mut seen = std::collections::HashSet::new();
    for e in &evts {
        assert!(
            seen.insert(e.durable_seq),
            "duplicate durable_seq: {}",
            e.durable_seq
        );
    }
}
