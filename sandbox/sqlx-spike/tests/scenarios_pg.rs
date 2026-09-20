//! Postgres scenario tests for the SQLx spike.

use shadows_domain::{
    CommandId, CommandRecord, DurableEvent, EventKind, Operation, OperationId, OperationKind,
    OperationOutcome, OperationStatus, Principal, Project, ProjectId, ResearchArtifact, ResearchId,
    RuntimeInstanceId, SearchQuery, SearchScope, TaskId, ThreadId, Timestamp, WorkflowId,
};
use shadows_sqlx_spike::ports::{
    AtomicCommandTx, EventStore, MultiEntityTx, MultiOp, OperationStore, ProjectStore,
    ResearchStore, SearchIndex,
};
use shadows_sqlx_spike::storage::postgres::PostgresBackend;

const PG_URL: &str = "postgres://shadows:test@127.0.0.1:5433/shadows";

async fn fresh() -> PostgresBackend {
    // Note: tests share a Postgres DB so the durable_seq counter and existing
    // rows accumulate across tests. Tests that need isolation use a baseline
    // read of MAX(durable_seq) before they write.
    PostgresBackend::connect(PG_URL)
        .await
        .expect("postgres connect")
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
async fn pg_s1_atomic_command_happy_path() {
    let b = fresh().await;
    let baseline_seq: u64 =
        sqlx::query_as::<_, (i64,)>("SELECT COALESCE(MAX(durable_seq), 0) FROM durable_event")
            .fetch_one(b.pool())
            .await
            .expect("baseline")
            .0 as u64;
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

    let events = EventStore::read_after(&b, baseline_seq, 10)
        .await
        .expect("read events");
    assert_eq!(events.len(), 1, "exactly one event from this command");
    assert_eq!(events[0].kind, EventKind::OperationStarted);
}

#[tokio::test]
async fn pg_s1_idempotent() {
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
}

#[tokio::test]
async fn pg_s2_event_ordering() {
    let b = fresh().await;
    let baseline_seq: u64 =
        sqlx::query_as::<_, (i64,)>("SELECT COALESCE(MAX(durable_seq), 0) FROM durable_event")
            .fetch_one(b.pool())
            .await
            .expect("baseline")
            .0 as u64;
    EventStore::append(
        &b,
        DurableEvent {
            id: shadows_domain::EventId::new(),
            durable_seq: 0,
            kind: EventKind::OperationStarted,
            occurred_at: Timestamp::now_utc(),
            operation_id: None,
            payload: serde_json::json!({"i": 1}),
        },
    )
    .await
    .expect("a1");
    EventStore::append(
        &b,
        DurableEvent {
            id: shadows_domain::EventId::new(),
            durable_seq: 0,
            kind: EventKind::OperationCompleted,
            occurred_at: Timestamp::now_utc(),
            operation_id: None,
            payload: serde_json::json!({"i": 2}),
        },
    )
    .await
    .expect("a2");
    let read = EventStore::read_after(&b, baseline_seq, 1000)
        .await
        .expect("read");
    assert_eq!(read.len(), 2);
    let mut last = baseline_seq;
    for e in &read {
        assert!(e.durable_seq > last, "ordering by seq only");
        last = e.durable_seq;
    }
}

#[tokio::test]
async fn pg_s3_search_research() {
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
async fn pg_s5_multi_entity_tx() {
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
    assert!(!read_events.is_empty());
}

#[tokio::test]
async fn pg_s7_typed_data_roundtrip() {
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
    assert_eq!(got.task_id, None);
    assert_eq!(got.durable_seq, 7);
}

#[tokio::test]
async fn pg_s8_concurrent_atomic_commands() {
    let b = std::sync::Arc::new(fresh().await);
    // Establish a baseline so we only inspect events from THIS run.
    let baseline_seq: u64 =
        sqlx::query_as::<_, (i64,)>("SELECT COALESCE(MAX(durable_seq), 0) FROM durable_event")
            .fetch_one(b.pool())
            .await
            .expect("baseline")
            .0 as u64;
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
        if h.await.unwrap().is_ok() {
            successes += 1;
        }
    }
    assert_eq!(successes, 8, "all 8 concurrent commands should succeed");
    let evts = EventStore::read_after(&*b, baseline_seq, 1000)
        .await
        .expect("read");
    assert_eq!(evts.len(), 8, "exactly 8 events from this run");
    let mut seen = std::collections::HashSet::new();
    for e in &evts {
        assert!(
            seen.insert(e.durable_seq),
            "duplicate durable_seq within run: {}",
            e.durable_seq
        );
    }
}
