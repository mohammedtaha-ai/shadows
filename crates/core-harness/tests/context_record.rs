mod fixture;

use core_db::{CommandContext, Storage};
use core_harness::CarriedContext;
use core_operations::{OperationId, Operations};
use core_projects::Projects;
use core_runtime::Runtime;
use core_threads::{ThreadId, Threads};
use std::sync::Arc;

fn carried(text: &str) -> CarriedContext {
    CarriedContext {
        digest: text.into(),
        text: text.into(),
        sources: vec![],
        index_tokens: 1,
        detail_tokens: 0,
        over_tokens: 0,
        cuts: vec![],
    }
}

async fn operation(storage: &Arc<Storage>) -> (ThreadId, OperationId) {
    let folder = tempfile::tempdir().unwrap();
    let id = uuid::Uuid::new_v4().to_string();
    let project = Projects::new(storage.clone())
        .create(id.clone(), &id, "Demo", folder.path().to_str().unwrap())
        .await
        .unwrap();
    let thread = Threads::create_planning_thread_for_test(
        storage,
        &CommandContext::user(id, "thread.create", serde_json::json!({})),
        &project.id,
        "Planner",
        "claude-code",
        None,
    )
    .await
    .unwrap();
    let runtime = Runtime::start(storage.clone()).await.unwrap();
    let runtime_id = runtime.instance_id().clone();
    let thread = thread.id;
    let op = OperationId::from_recorded(uuid::Uuid::new_v4().to_string());
    let (thread_copy, op_copy) = (thread.clone(), op.clone());
    storage
        .write_txn(move |conn| {
            Box::pin(async move {
                let _ = Operations::insert_pending_in(
                    conn,
                    &op_copy,
                    &thread_copy,
                    &runtime_id,
                    &Storage::now(),
                )
                .await?;
                let _ =
                    Operations::started_in(conn, &op_copy, &runtime_id, &Storage::now()).await?;
                Ok(())
            })
        })
        .await
        .unwrap();
    (thread, op)
}

#[tokio::test]
async fn a_prepared_context_is_read_back_unheard() {
    let (_dir, storage, harness) = fixture::fixture().await;
    let (thread, op) = operation(&storage).await;
    harness
        .prepare_context(&op, &thread, "session-a", &carried("index"))
        .await
        .unwrap();
    assert_eq!(
        harness.context_record(&op).await.unwrap(),
        Some((carried("index"), false))
    );
    storage.close().await.unwrap();
}

#[tokio::test]
async fn only_a_heard_context_counts() {
    let (_dir, storage, harness) = fixture::fixture().await;
    let (thread, op) = operation(&storage).await;
    harness
        .prepare_context(&op, &thread, "session-a", &carried("first"))
        .await
        .unwrap();
    assert_eq!(
        harness.heard_context_digest("session-a").await.unwrap(),
        None
    );
    harness.mark_context_heard(&op).await.unwrap();
    assert_eq!(
        harness.heard_context_digest("session-a").await.unwrap(),
        Some("first".into())
    );
    storage.close().await.unwrap();
}

#[tokio::test]
async fn a_new_session_on_the_same_thread_has_heard_nothing() {
    let (_dir, storage, harness) = fixture::fixture().await;
    let (thread, op) = operation(&storage).await;
    harness
        .prepare_context(&op, &thread, "session-a", &carried("first"))
        .await
        .unwrap();
    harness.mark_context_heard(&op).await.unwrap();
    assert_eq!(
        harness.heard_context_digest("session-b").await.unwrap(),
        None
    );
    storage.close().await.unwrap();
}

#[tokio::test]
async fn the_newest_heard_digest_wins() {
    let (_dir, storage, harness) = fixture::fixture().await;
    let (thread, op) = operation(&storage).await;
    harness
        .prepare_context(&op, &thread, "session-a", &carried("first"))
        .await
        .unwrap();
    let (next_thread, next_op) = operation(&storage).await;
    harness
        .prepare_context(&next_op, &next_thread, "session-a", &carried("second"))
        .await
        .unwrap();
    harness.mark_context_heard(&next_op).await.unwrap();
    harness.mark_context_heard(&op).await.unwrap();
    assert_eq!(
        harness.heard_context_digest("session-a").await.unwrap(),
        Some("second".into())
    );
    storage.close().await.unwrap();
}
