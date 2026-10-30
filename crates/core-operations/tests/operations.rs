use core_db::{CommandContext, Storage};
use core_operations::{OperationId, Operations};
use core_projects::{ProjectDirectory, Projects};
use core_runtime::Runtime;
use core_threads::Threads;
use std::collections::BTreeMap;

#[tokio::test]
async fn operations_tracks_thread_busy_and_recovery() {
    let tmp = tempfile::tempdir().unwrap();
    let db_path = tmp.path().join("test.sqlite3");
    let storage = std::sync::Arc::new(Storage::open(&db_path).await.unwrap());
    let operations = Operations::new(storage.clone());
    let projects = Projects::new(storage.clone());

    let runtime = Runtime::register_runtime_instance_for_test(&storage, "runtime-a")
        .await
        .unwrap();

    let ctx = CommandContext::user(
        "c1".into(),
        "project.create",
        serde_json::json!({ "slug": "p1" }),
    );
    let dir = ProjectDirectory::resolve(tmp.path()).unwrap();
    let project = projects
        .create_for_test(&ctx, "p1", "P1", &dir, &BTreeMap::new())
        .await
        .unwrap();

    let tctx = CommandContext::user(
        "c2".into(),
        "thread.create",
        serde_json::json!({ "project": project.id }),
    );
    let thread = Threads::create_planning_thread_for_test(
        &storage,
        &tctx,
        &project.id,
        "demo",
        "harness-a",
        None,
    )
    .await
    .unwrap();

    assert!(!operations.thread_is_busy(&thread.id).await.unwrap());

    let op_id = OperationId::generate();
    let now = Storage::now();
    storage
        .write_txn(|conn| {
            let op = op_id.clone();
            let th = thread.id.clone();
            let rt = runtime.clone();
            let ts = now.clone();
            Box::pin(async move { Operations::insert_pending_in(conn, &op, &th, &rt, &ts).await })
        })
        .await
        .unwrap()
        .log();

    assert!(operations.thread_is_busy(&thread.id).await.unwrap());
    assert_eq!(operations.running_for(&thread.id).await.unwrap(), None);

    let runtime_b = Runtime::register_runtime_instance_for_test(&storage, "runtime-b")
        .await
        .unwrap();
    let report = operations.recover(&runtime_b).await.unwrap();
    assert_eq!(report.interrupted, vec![op_id]);
    assert!(!operations.thread_is_busy(&thread.id).await.unwrap());

    storage.close().await.unwrap();
}

struct Fixture {
    _tmp: tempfile::TempDir,
    storage: std::sync::Arc<Storage>,
    operations: Operations,
    runtime: core_runtime::RuntimeInstanceId,
    thread: core_threads::ThreadId,
}

async fn fixture() -> Fixture {
    let tmp = tempfile::tempdir().unwrap();
    let storage = std::sync::Arc::new(Storage::open(&tmp.path().join("t.sqlite3")).await.unwrap());
    let projects = Projects::new(storage.clone());
    let runtime = Runtime::register_runtime_instance_for_test(&storage, "runtime-a")
        .await
        .unwrap();
    let ctx = CommandContext::user("c1".into(), "project.create", serde_json::json!({}));
    let dir = ProjectDirectory::resolve(tmp.path()).unwrap();
    let project = projects
        .create_for_test(&ctx, "p1", "P1", &dir, &BTreeMap::new())
        .await
        .unwrap();
    let tctx = CommandContext::user("c2".into(), "thread.create", serde_json::json!({}));
    let thread = Threads::create_planning_thread_for_test(
        &storage,
        &tctx,
        &project.id,
        "demo",
        "harness-a",
        None,
    )
    .await
    .unwrap();
    Fixture {
        operations: Operations::new(storage.clone()),
        storage,
        runtime,
        thread: thread.id,
        _tmp: tmp,
    }
}

impl Fixture {
    async fn create(&self) -> OperationId {
        let op = OperationId::generate();
        let (o, t, r) = (op.clone(), self.thread.clone(), self.runtime.clone());
        self.storage
            .write_txn(move |conn| {
                Box::pin(async move {
                    Operations::insert_pending_in(conn, &o, &t, &r, &Storage::now()).await
                })
            })
            .await
            .unwrap()
            .log();
        op
    }

    async fn start(&self, op: &OperationId) {
        let (o, r) = (op.clone(), self.runtime.clone());
        self.storage
            .write_txn(move |conn| {
                Box::pin(async move { Operations::started_in(conn, &o, &r, &Storage::now()).await })
            })
            .await
            .unwrap()
            .log();
    }

    async fn request_cancel(&self, op: &OperationId) {
        let o = op.clone();
        self.storage
            .write_txn(move |conn| {
                Box::pin(async move {
                    let actor = core_db::Actor::system();
                    Operations::request_cancellation_in(conn, &o, actor, &Storage::now()).await
                })
            })
            .await
            .unwrap();
    }

    async fn cancel(&self, op: &OperationId) {
        let o = op.clone();
        self.storage
            .write_txn(move |conn| {
                Box::pin(async move { Operations::cancelled_in(conn, &o, &Storage::now()).await })
            })
            .await
            .unwrap()
            .log();
    }

    async fn complete(&self, op: &OperationId) {
        let o = op.clone();
        self.storage
            .write_txn(move |conn| {
                Box::pin(async move {
                    let invocation = Some(serde_json::json!({ "agent": "a" }));
                    Operations::completed_in(conn, &o, "{}", invocation, &Storage::now()).await
                })
            })
            .await
            .unwrap()
            .log();
    }

    async fn fail(&self, op: &OperationId) {
        let o = op.clone();
        self.storage
            .write_txn(move |conn| {
                Box::pin(async move {
                    Operations::failed_in(conn, &o, "Run", "boom", &Storage::now()).await
                })
            })
            .await
            .unwrap()
            .log();
    }

    /// Every event of the operation as (kind, payload), in journal order.
    async fn events(&self, op: &OperationId) -> Vec<(String, serde_json::Value)> {
        let rows: Vec<(String, String)> = sqlx::query_as(
            "SELECT kind, payload_json FROM durable_event WHERE operation_id = ? ORDER BY seq",
        )
        .bind(op.as_str())
        .fetch_all(self.storage.reader())
        .await
        .unwrap();
        rows.into_iter()
            .map(|(kind, json)| (kind, serde_json::from_str(&json).unwrap()))
            .collect()
    }

    async fn latest_seq(&self, op: &OperationId) -> i64 {
        sqlx::query_scalar("SELECT MAX(seq) FROM durable_event WHERE operation_id = ?")
            .bind(op.as_str())
            .fetch_one(self.storage.reader())
            .await
            .unwrap()
    }

    async fn listed_seq(&self, op: &OperationId) -> i64 {
        let mut conn = self.storage.reader().acquire().await.unwrap();
        let rows = Operations::list_operations_for_thread_in(&mut conn, &self.thread)
            .await
            .unwrap();
        rows.into_iter().find(|r| &r.id == op).unwrap().seq
    }
}

fn flags(event: &(String, serde_json::Value)) -> (&str, &str, bool) {
    (
        event.0.as_str(),
        event.1["status_kind"].as_str().unwrap(),
        event.1["cancel_requested"].as_bool().unwrap(),
    )
}

#[tokio::test]
async fn every_lifecycle_event_payload_carries_status_and_cancel_flag() {
    let f = fixture().await;

    let cancelled = f.create().await;
    f.start(&cancelled).await;
    f.request_cancel(&cancelled).await;
    f.cancel(&cancelled).await;
    let events = f.events(&cancelled).await;
    let seen: Vec<_> = events.iter().map(flags).collect();
    assert_eq!(
        seen,
        vec![
            ("OperationCreated", "Pending", false),
            ("OperationStarted", "Running", false),
            ("OperationCancellationRequested", "Running", true),
            ("OperationCancelled", "Cancelled", true),
        ]
    );
    assert_eq!(events[0].1["kind"], "PlannerTurn");

    let completed = f.create().await;
    f.start(&completed).await;
    f.complete(&completed).await;
    let events = f.events(&completed).await;
    assert_eq!(
        flags(&events[2]),
        ("OperationCompleted", "Completed", false)
    );
    assert_eq!(events[2].1["invocation"]["agent"], "a");

    let failed = f.create().await;
    f.request_cancel(&failed).await;
    f.fail(&failed).await;
    let events = f.events(&failed).await;
    assert_eq!(
        flags(&events[1]),
        ("OperationCancellationRequested", "Pending", true)
    );
    assert_eq!(flags(&events[2]), ("OperationFailed", "Failed", true));
    assert_eq!(events[2].1["stage"], "Run");

    let lost = f.create().await;
    f.start(&lost).await;
    f.request_cancel(&lost).await;
    let next = Runtime::register_runtime_instance_for_test(&f.storage, "runtime-b")
        .await
        .unwrap();
    f.operations.recover(&next).await.unwrap();
    let events = f.events(&lost).await;
    assert_eq!(
        flags(&events[3]),
        ("OperationInterrupted", "Interrupted", true)
    );
    assert!(events[3].1["reason"].is_string());
}

#[tokio::test]
async fn a_listed_operation_seq_is_its_latest_event_sequence() {
    let f = fixture().await;
    let op = f.create().await;
    assert_eq!(f.listed_seq(&op).await, f.latest_seq(&op).await);
    f.start(&op).await;
    assert_eq!(f.listed_seq(&op).await, f.latest_seq(&op).await);
    f.request_cancel(&op).await;
    assert_eq!(f.listed_seq(&op).await, f.latest_seq(&op).await);
    f.cancel(&op).await;
    let latest = f.latest_seq(&op).await;
    assert_eq!(f.listed_seq(&op).await, latest);
}
