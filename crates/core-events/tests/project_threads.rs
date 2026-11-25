use core_db::{CommandContext, Storage};
use core_events::Events;
use core_operations::{OperationId, Operations};
use core_projects::{ProjectDirectory, ProjectId, Projects};
use core_runtime::Runtime;
use core_threads::{ThreadId, Threads};
use std::collections::BTreeMap;
use std::sync::Arc;

async fn thread_in(storage: &Arc<Storage>, root: &std::path::Path, slug: &str) -> ThreadId {
    let directory = root.join(slug);
    std::fs::create_dir(&directory).unwrap();
    let ctx = CommandContext::user(
        format!("{slug}-project"),
        "project.create",
        serde_json::json!({ "slug": slug }),
    );
    let dir = ProjectDirectory::resolve(&directory).unwrap();
    let project = Projects::new(storage.clone())
        .create_for_test(&ctx, slug, slug, &dir, &BTreeMap::new())
        .await
        .unwrap();
    let ctx = CommandContext::user(
        format!("{slug}-thread"),
        "thread.create",
        serde_json::json!({ "project": project.id }),
    );
    Threads::create_planning_thread_for_test(storage, &ctx, &project.id, "t", "harness", None)
        .await
        .unwrap()
        .id
}

async fn end_a_turn(storage: &Arc<Storage>, thread: &ThreadId) {
    let runtime = Runtime::register_runtime_instance_for_test(storage, &Storage::now())
        .await
        .unwrap();
    let (op, thread) = (OperationId::generate(), thread.clone());
    storage
        .write_txn(move |conn| {
            Box::pin(async move {
                let ts = Storage::now();
                Operations::insert_pending_in(conn, &op, &thread, &runtime, &ts)
                    .await?
                    .log();
                Operations::started_in(conn, &op, &runtime, &ts)
                    .await?
                    .log();
                Operations::completed_in(conn, &op, "{}", None, &ts)
                    .await?
                    .log();
                Ok(())
            })
        })
        .await
        .unwrap();
}

async fn remove(storage: &Arc<Storage>, thread: &ThreadId) {
    let thread = thread.clone();
    let ctx = CommandContext::user(
        format!("remove-{thread}"),
        "thread.remove",
        serde_json::json!({ "thread": thread.as_str() }),
    );
    storage
        .write_txn(move |conn| {
            Box::pin(async move {
                let ts = Storage::now();
                let current = Threads::remove_in(conn, &thread, &ts).await?;
                Threads::record_removal_in(conn, &ctx, &current, &ts).await
            })
        })
        .await
        .unwrap();
}

#[tokio::test]
async fn the_project_stream_says_when_a_thread_list_changes() {
    let tmp = tempfile::tempdir().unwrap();
    let storage = Arc::new(Storage::open(&tmp.path().join("test.db")).await.unwrap());
    let thread = thread_in(&storage, tmp.path(), "project").await;
    let other = thread_in(&storage, tmp.path(), "other").await;
    let project: ProjectId = Threads::get_thread_for_test(&storage, &thread)
        .await
        .unwrap()
        .project_id;
    let after = Events::current_cursor_for_test(&storage).await.unwrap();
    for t in [&other, &thread] {
        assert!(
            Threads::title_from_harness_for_test(&storage, t, "Renamed")
                .await
                .unwrap()
        );
        end_a_turn(&storage, t).await;
        remove(&storage, t).await;
    }
    let rows = Events::read_project_events_after_for_test(storage.clone(), after, &project, 10)
        .await
        .unwrap();
    let kinds: Vec<&str> = rows.iter().map(|r| r.kind.as_str()).collect();
    assert_eq!(
        kinds,
        ["ThreadRetitled", "OperationCompleted", "ThreadRemoved"]
    );
    assert!(rows.windows(2).all(|w| w[0].seq < w[1].seq));
    assert!(rows.iter().all(|r| r.thread_id.as_ref() == Some(&thread)));
    storage.close().await.unwrap();
}
