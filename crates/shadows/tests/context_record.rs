use serde_json::{Value, json};
use shadows_core::testing::{CommandContext, acp};
use shadows_core::{CarriedContext, Forget, HarnessError, OperationId, StorageError, Turns};
#[path = "fixtures/app.rs"]
mod app;

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

async fn operation(app: &app::App) -> OperationId {
    let op = Turns::create_pending_operation_for_test(
        &app.storage,
        &app.thread,
        app.runtime.instance_id(),
    )
    .await
    .unwrap();
    Turns::mark_operation_started_for_test(&app.storage, &op, app.runtime.instance_id())
        .await
        .unwrap();
    op
}

async fn listed_invocation(app: &app::App, op: &OperationId) -> Value {
    let list: Vec<Value> =
        app::get_json(app, &format!("/api/threads/{}/operations", app.thread)).await;
    list.into_iter()
        .find(|item| item["id"] == op.as_str())
        .expect("operation is listed")["invocation"]
        .clone()
}

#[tokio::test]
async fn an_invocation_without_a_record_has_no_planning_context() {
    let app = app::test_app().await;
    let op = app::start_and_finish(&app, "/compact", app::default_settings()).await;
    let invocation = listed_invocation(&app, &op.id).await;
    assert_eq!(invocation.get("planning_context"), Some(&Value::Null));
    app::shut_down_app(app).await;
}

#[tokio::test]
async fn a_prepared_record_reads_prepared() {
    let app = app::test_app().await;
    let op = app::start_and_finish(&app, "/compact", app::default_settings()).await;
    app.core
        .harness()
        .prepare_context(&op.id, &app.thread, "session-a", &carried("index"))
        .await
        .unwrap();
    assert_eq!(
        listed_invocation(&app, &op.id).await["planning_context"],
        json!("prepared")
    );
    app::shut_down_app(app).await;
}

#[tokio::test]
async fn a_heard_record_reads_heard() {
    let app = app::test_app().await;
    let op = app::start_and_finish(&app, "Read the project", app::default_settings()).await;
    assert_eq!(
        listed_invocation(&app, &op.id).await["planning_context"],
        json!("heard")
    );
    let single = Turns::get_operation_for_test(&app.storage, &op.id)
        .await
        .unwrap();
    let single = serde_json::to_value(single).unwrap();
    assert_eq!(single["invocation"]["planning_context"], json!("heard"));
    app::shut_down_app(app).await;
}

#[tokio::test]
async fn a_prepared_context_is_read_back_unheard() {
    let app = app::test_app().await;
    let op = operation(&app).await;
    app.core
        .harness()
        .prepare_context(&op, &app.thread, "session-a", &carried("index"))
        .await
        .unwrap();
    assert_eq!(
        app.core.harness().context_record(&op).await.unwrap(),
        Some((carried("index"), false))
    );
    app::shut_down_app(app).await;
}

#[tokio::test]
async fn only_a_heard_context_counts() {
    let app = app::test_app().await;
    let op = operation(&app).await;
    app.core
        .harness()
        .prepare_context(&op, &app.thread, "session-a", &carried("first"))
        .await
        .unwrap();
    assert_eq!(
        app.core
            .harness()
            .heard_context_digest("session-a")
            .await
            .unwrap(),
        None
    );
    app.core.harness().mark_context_heard(&op).await.unwrap();
    assert_eq!(
        app.core
            .harness()
            .heard_context_digest("session-a")
            .await
            .unwrap(),
        Some("first".into())
    );
    app::shut_down_app(app).await;
}

#[tokio::test]
async fn a_new_session_on_the_same_thread_has_heard_nothing() {
    let app = app::test_app().await;
    let op = operation(&app).await;
    app.core
        .harness()
        .prepare_context(&op, &app.thread, "session-a", &carried("first"))
        .await
        .unwrap();
    app.core.harness().mark_context_heard(&op).await.unwrap();
    assert_eq!(
        app.core
            .harness()
            .heard_context_digest("session-b")
            .await
            .unwrap(),
        None
    );
    app::shut_down_app(app).await;
}

#[tokio::test]
async fn the_newest_heard_digest_wins() {
    let app = app::test_app().await;
    let first = operation(&app).await;
    app.core
        .harness()
        .prepare_context(&first, &app.thread, "session-a", &carried("first"))
        .await
        .unwrap();
    let second = operation(&app).await;
    app.core
        .harness()
        .prepare_context(&second, &app.thread, "session-a", &carried("second"))
        .await
        .unwrap();
    app.core
        .harness()
        .mark_context_heard(&second)
        .await
        .unwrap();
    app.core.harness().mark_context_heard(&first).await.unwrap();
    assert_eq!(
        app.core
            .harness()
            .heard_context_digest("session-a")
            .await
            .unwrap(),
        Some("second".into())
    );
    app::shut_down_app(app).await;
}

async fn digest(app: &app::App) -> Option<String> {
    app.core
        .harness()
        .heard_context_digest("session-a")
        .await
        .unwrap()
}

async fn prepared(app: &app::App, text: &str) -> OperationId {
    let op = operation(app).await;
    let harness = app.core.harness();
    harness
        .prepare_context(&op, &app.thread, "session-a", &carried(text))
        .await
        .unwrap();
    op
}

fn request(command: &str) -> Forget {
    Forget::Request {
        command_id: command.into(),
    }
}

async fn forgets(app: &app::App) -> i64 {
    sqlx::query_scalar("SELECT COUNT(*) FROM context_forget")
        .fetch_one(app.storage.reader())
        .await
        .unwrap()
}

#[tokio::test]
async fn a_forget_makes_a_heard_context_owed() {
    let app = app::test_app().await;
    let op = prepared(&app, "first").await;
    app.core.harness().mark_context_heard(&op).await.unwrap();
    assert_eq!(digest(&app).await, Some("first".into()));
    let forget = request("forget-1");
    app.core
        .harness()
        .forget_context(&app.thread, forget)
        .await
        .unwrap();
    assert_eq!(digest(&app).await, None);
    app::shut_down_app(app).await;
}

#[tokio::test]
async fn a_context_prepared_after_the_forget_counts() {
    let app = app::test_app().await;
    let old = prepared(&app, "old").await;
    app.core.harness().mark_context_heard(&old).await.unwrap();
    let forget = request("forget-1");
    app.core
        .harness()
        .forget_context(&app.thread, forget)
        .await
        .unwrap();
    let new = prepared(&app, "new").await;
    app.core.harness().mark_context_heard(&new).await.unwrap();
    assert_eq!(digest(&app).await, Some("new".into()));
    app::shut_down_app(app).await;
}

#[tokio::test]
async fn a_forget_in_the_turn_that_carried_it_still_counts_as_lost() {
    let app = app::test_app().await;
    let op = prepared(&app, "carried").await;
    let forget = Forget::Compaction {
        operation: op.clone(),
        trigger: Some("automatic".into()),
    };
    app.core
        .harness()
        .forget_context(&app.thread, forget)
        .await
        .unwrap();
    app.core.harness().mark_context_heard(&op).await.unwrap();
    assert_eq!(digest(&app).await, None);
    app::shut_down_app(app).await;
}

#[tokio::test]
async fn a_replayed_request_forgets_once() {
    let app = app::test_app().await;
    for _ in 0..2 {
        let forget = request("same");
        app.core
            .harness()
            .forget_context(&app.thread, forget)
            .await
            .unwrap();
    }
    assert_eq!(forgets(&app).await, 1);
    app::shut_down_app(app).await;
}

#[tokio::test]
async fn a_request_with_other_params_conflicts() {
    let app = app::test_app().await;
    let thread = app.thread.as_str().to_owned();
    app.storage
        .write_txn(move |conn| {
            Box::pin(async move {
                let other = app::ctx("same", "thread.other");
                CommandContext::record_command(conn, &other, "Thread", &thread, "X", "1", "t").await
            })
        })
        .await
        .unwrap();
    let result = app
        .core
        .harness()
        .forget_context(&app.thread, request("same"))
        .await;
    assert!(matches!(
        result,
        Err(HarnessError::Storage(StorageError::CommandConflict))
    ));
    assert_eq!(forgets(&app).await, 0);
    app::shut_down_app(app).await;
}

#[tokio::test]
async fn an_aylul_thread_cannot_forget() {
    let app = app::test_app().await;
    let thread = shadows_core::Threads::create_planning_thread_for_test(
        &app.storage,
        &app::ctx("aylul", "thread.create"),
        &app.project,
        "Assistant",
        "aylul",
        None,
    )
    .await
    .unwrap();
    let result = app
        .core
        .harness()
        .forget_context(&thread.id, request("x"))
        .await;
    assert!(matches!(result, Err(HarnessError::InvalidCommand(_))));
    let compaction = Forget::Compaction {
        operation: operation(&app).await,
        trigger: Some("automatic".into()),
    };
    app.core
        .harness()
        .forget_context(&thread.id, compaction)
        .await
        .unwrap();
    assert_eq!(forgets(&app).await, 0);
    app::shut_down_app(app).await;
}
