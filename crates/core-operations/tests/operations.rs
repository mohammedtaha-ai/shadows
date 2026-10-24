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
}
