//! What the harness reports about a turn (spec §12.8): the answering model and
//! the context, written into the turn's invocation when it completes; the
//! account limits, kept per harness; a `usage` frame as each report arrives;
//! and the context breakdown, read on demand without leaving a trace.

use serde_json::{Value, json};

#[path = "fixtures/acp.rs"]
mod acp;
#[path = "fixtures/app.rs"]
mod app;

use app::{entries, get_json, post, start_settled, test_app, wait_terminal};

#[tokio::test]
async fn a_completed_turn_records_what_the_harness_reported() {
    let app = test_app().await;
    let done = wait_terminal(&app, &start_settled(&app, "usage").await).await;
    assert_eq!(done.status_kind, "Completed");
    let inv = done.invocation.unwrap();
    assert_eq!(inv.observed_model.as_deref(), Some("fake-large-answering"));
    assert_eq!(
        (inv.context_used, inv.context_window),
        (Some(1234), Some(1_000_000)),
        "the last report, not the 200k guess"
    );
    let limits = app
        .storage
        .latest_limits("claude-code")
        .await
        .unwrap()
        .unwrap();
    assert!((limits.seven_day.unwrap().utilization - 0.5).abs() < 1e-9);
    assert_eq!(limits.five_hour.unwrap().resets_at, 1790212200);
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
}

#[tokio::test]
async fn the_breakdown_is_read_on_demand_and_leaves_no_trace() {
    let app = test_app().await;
    wait_terminal(&app, &start_settled(&app, "hi").await).await;
    let before = entries(&app).await.len();
    let b: Value = get_json(&app, &format!("/api/threads/{}/context", app.thread)).await;
    assert_eq!(b["categories"][0]["name"], "Messages", "{b}");
    assert_eq!(b["categories"][2]["tokens"], 923_900);
    assert!(b["reason"].is_null());
    assert_eq!(entries(&app).await.len(), before, "no entry");
    let ops: Vec<Value> = get_json(&app, &format!("/api/threads/{}/operations", app.thread)).await;
    assert_eq!(ops.len(), 1, "no operation");
    let next = wait_terminal(&app, &start_settled(&app, "report").await).await;
    assert_eq!(
        next.status_kind, "Completed",
        "the session still runs turns"
    );
}

#[tokio::test]
async fn no_breakdown_before_the_first_turn_or_while_one_runs() {
    let app = test_app().await;
    let path = format!("/api/threads/{}/context", app.thread);
    let b: Value = get_json(&app, &path).await;
    assert!(b["categories"].is_null() && b["reason"].as_str().unwrap().contains("open"));
    post(
        &app,
        &format!("/api/threads/{}/session", app.thread),
        json!({}),
    )
    .await;
    let b: Value = get_json(&app, &path).await;
    assert!(b["categories"].is_null() && b["reason"].as_str().unwrap().contains("first"));
    let _running = start_settled(&app, "hang").await;
    let b2: Value = get_json(&app, &path).await;
    assert!(
        b2["categories"].is_null() && b2["reason"].as_str().unwrap().contains("running"),
        "{b2}"
    );
}

#[test]
fn parse_reads_the_category_table_only() {
    let md = "## Context Usage\n\n| Category | Tokens | Percentage |\n|---|---|---|\n| Messages | 3.8k | 0.4% |\n| Free space | 923.9k | 92.4% |\n\n### MCP Tools\n| Tool | Server | Tokens |\n| x | y | 400 |\n";
    let c = shadows::agent::breakdown::parse(md).unwrap();
    assert_eq!(c.len(), 2);
    assert_eq!((c[1].name.as_str(), c[1].tokens), ("Free space", 923_900));
    assert!((c[0].percent - 0.4).abs() < 1e-9);
    assert!(shadows::agent::breakdown::parse("no table").is_none());
}
