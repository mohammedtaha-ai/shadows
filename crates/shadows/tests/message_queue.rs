//! Spec §20: writing while a turn runs — the queue and Send now.

#[path = "fixtures/app.rs"]
mod app;

use app::{ctx, test_app};
use shadows_core::StorageError;
use shadows_core::testing::acp;
use shadows_core::testing::queue::{QueueAnswer, new_queued};
use shadows_core::testing::turn::{new_turn, turn_command};

#[tokio::test]
async fn waiting_messages_keep_their_order_and_a_removed_one_is_gone() {
    let app = test_app().await;
    // A queue needs a busy thread: hold one turn open at the store.
    let settings = app::settings();
    app.storage
        .start_turn(
            &turn_command("t0", &app.thread, "hang", &settings),
            new_turn(&app.thread, &app.runtime, "hang", &settings),
        )
        .await
        .unwrap();
    let first = match app
        .storage
        .queue_message(&ctx("q1", "turn.queue"), &app.thread, new_queued("one"))
        .await
        .unwrap()
    {
        QueueAnswer::Waiting(m) => m,
        QueueAnswer::Idle => panic!("a busy thread queues"),
    };
    app.storage
        .queue_message(&ctx("q2", "turn.queue"), &app.thread, new_queued("two"))
        .await
        .unwrap();
    let listed = app.storage.queued_messages(&app.thread).await.unwrap();
    let prompts: Vec<&str> = listed.iter().map(|m| m.prompt.as_str()).collect();
    assert_eq!(prompts, ["one", "two"]);

    app.storage
        .unqueue_message(&ctx("u1", "turn.unqueue"), &app.thread, &first.id)
        .await
        .unwrap();
    let again = app
        .storage
        .unqueue_message(&ctx("u2", "turn.unqueue"), &app.thread, &first.id)
        .await;
    assert!(matches!(again, Err(StorageError::QueuedMessageGone)));
}
