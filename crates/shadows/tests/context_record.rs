use shadows_core::testing::acp;
use shadows_core::{CarriedContext, OperationId, Turns};
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
