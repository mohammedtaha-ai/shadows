//! What the harness reports about a turn: the answering model and
//! the context, written into the turn's invocation when it completes; the
//! account limits, kept per harness; a `usage` frame as each report arrives.

use shadows_core::testing::acp;
#[path = "fixtures/app.rs"]
mod app;

use app::{entries, start_settled, test_app, test_app_at, wait_terminal};

#[tokio::test]
async fn a_completed_turn_records_what_the_harness_reported() {
    let app = test_app().await;
    let done = wait_terminal(&app, &start_settled(&app, "usage").await).await;
    assert_eq!(done.status_kind, "Completed");
    let inv = done.invocation.unwrap();
    assert_eq!(inv.observed_model.as_deref(), Some("fake-large-answering"));
    let session: Option<String> =
        sqlx::query_scalar("SELECT native_session_id FROM agent_invocation WHERE operation_id = ?")
            .bind(done.id.as_str())
            .fetch_one(app.storage.reader())
            .await
            .unwrap();
    assert!(
        session.is_some_and(|s| s.starts_with("fake-")),
        "the session it ran in"
    );
    assert_eq!(
        (inv.context_used, inv.context_window),
        (Some(1234), Some(1_000_000)),
        "the last report, not the 200k guess"
    );
    let limits = app
        .core
        .harness()
        .latest_limits("claude-code")
        .await
        .unwrap()
        .unwrap();
    assert!((limits.seven_day.unwrap().utilization - 0.5).abs() < 1e-9);
    assert_eq!(limits.five_hour.unwrap().resets_at, 1790212200);

    app.core
        .shut_down(std::time::Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
}

#[tokio::test]
async fn the_completed_event_carries_the_invocation() {
    let app = test_app().await;
    let mut sub = app::subscribe(&app, &app.thread).await;
    start_settled(&app, "usage").await;
    loop {
        let frame = app::next_frame_named(&mut sub, "durable").await;
        if frame["kind"] == "OperationCompleted" {
            let inv = &frame["payload"]["invocation"];
            assert_eq!(inv["observed_model"], "fake-large-answering");
            assert_eq!(inv["context_window"], 1_000_000);
            assert_eq!(inv["requested_model"], "fake-large");
            break;
        }
    }

    app.core
        .shut_down(std::time::Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
}

#[tokio::test]
async fn a_turn_without_usage_leaves_the_observation_unavailable() {
    let app = test_app().await;
    let inv = wait_terminal(&app, &start_settled(&app, "hi").await)
        .await
        .invocation
        .unwrap();
    assert!(
        inv.context_used.is_none() && inv.context_window.is_none() && inv.observed_model.is_none()
    );

    app.core
        .shut_down(std::time::Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
}

#[tokio::test]
async fn a_usage_frame_reaches_a_subscriber_of_the_thread() {
    let app = test_app().await;
    let mut sub = app::subscribe(&app, &app.thread).await;
    start_settled(&app, "usage").await;
    let first = app::next_frame_named(&mut sub, "usage").await;
    assert_eq!(first["thread_id"], app.thread.as_str());
    assert_eq!(first["context_used"], 1234);
    assert_eq!(first["context_window"], 200_000);
    let frame = app::next_frame_named(&mut sub, "usage").await;
    assert_eq!(frame["context_window"], 1_000_000);
    assert!(frame["limits"]["seven_day"].is_object(), "{frame}");

    app.core
        .shut_down(std::time::Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
}

/// A turn waits for a session holder to finish, then collects only its own output.
#[tokio::test]
async fn a_turn_started_during_a_session_hold_keeps_its_output() {
    use shadows_agent::events::HarnessEvent;
    use std::time::Duration;

    let tmp = tempfile::tempdir().unwrap();
    let app = test_app_at(tmp.path()).await;
    wait_terminal(&app, &start_settled(&app, "hi").await).await;
    let harness = app.core.harness();
    let opened = harness.open(&app.thread).await.unwrap();
    let mut held = harness.lease_events(&app.thread, &opened).await.unwrap();
    let start = start_settled(&app, "two-messages");
    tokio::pin!(start);

    tokio::time::timeout(Duration::from_secs(5), async {
        let (answer, ()) = tokio::join!(
            opened
                .connection()
                .prompt(&opened.session_id, "wait-for-release", &[]),
            async {
                loop {
                    let event = held.recv().await.expect("the holder's events stay open");
                    if let HarnessEvent::Chunk { text, .. } = event {
                        assert_eq!(text, "waiting");
                        break;
                    }
                }
                assert!(
                    tokio::time::timeout(Duration::from_millis(200), &mut start)
                        .await
                        .is_err(),
                    "the turn must wait while the holder owns the session"
                );
                std::fs::write(tmp.path().join("release"), "").unwrap();
            }
        );
        answer.unwrap();
        while held.try_recv().is_ok() {}
        harness.give_back_events(&app.thread, &opened, held).await;
    })
    .await
    .expect("the holder finishes and returns the session");

    let op = start.await;
    assert_eq!(wait_terminal(&app, &op).await.status_kind, "Completed");
    let bodies: Vec<String> = entries(&app).await.into_iter().map(|e| e.body).collect();
    assert!(bodies.iter().any(|b| b == "first"), "{bodies:?}");
    assert!(bodies.iter().any(|b| b == "second"), "{bodies:?}");
    assert!(!bodies.iter().any(|b| b == "waiting"), "{bodies:?}");

    app.core
        .shut_down(std::time::Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
}
