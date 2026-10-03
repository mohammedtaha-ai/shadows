use serde_json::{Value, json};
use shadows_core::testing::{EventCursor, acp};
#[path = "fixtures/app.rs"]
mod app;

#[tokio::test]
async fn vision_routes_round_trip_conflict_and_preserve_conversations() {
    let app = app::test_app().await;
    let vision = format!("/api/projects/{}/design/vision", app.project);
    let edits = format!("/api/projects/{}/design/edits", app.project);
    assert_eq!(app::call(&app, "GET", &vision, None).await.1["revision"], 0);
    let content = json!({ "purpose": " رؤية \n", "users": "مطورون", "goals": "اهداف", "boundaries": "", "technical_direction": "Rust" });
    let request = json!({ "command_id": "vision", "expected_revision": 0, "ops": [{ "kind": "VisionPut", "content": content }] });
    assert_eq!(
        app::call(&app, "POST", &edits, Some(request.clone())).await,
        (200, json!({"revision": 1}))
    );
    assert_eq!(
        app::call(&app, "GET", &vision, None).await,
        (200, json!({"revision": 1, "content": content}))
    );
    let mut stale = request.clone();
    stale["command_id"] = json!("stale");
    let (status, failure) = app::call(&app, "POST", &edits, Some(stale)).await;
    assert_eq!(status, 409);
    assert_eq!(failure["code"], "REVISION_CONFLICT");
    assert_eq!(failure["current_revision"], 1);
    let mut changed = request;
    changed["ops"][0]["content"]["purpose"] = json!("changed");
    let (status, failure) = app::call(&app, "POST", &edits, Some(changed)).await;
    assert_eq!(status, 409);
    assert_eq!(failure["code"], "COMMAND_CONFLICT");
    for path in [
        format!("/api/threads/{}", app.thread),
        format!("/api/threads/{}/entries", app.thread),
        format!("/api/projects/{}/threads", app.project),
        format!("/api/projects/{}/workflows", app.project),
    ] {
        assert_eq!(app::call(&app, "GET", &path, None).await.0, 200);
    }
    let events = app
        .storage
        .read_project_events_after(EventCursor(0), &app.project, 100)
        .await
        .unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].kind, "ProjectDesignChanged");
    let payload: Value = serde_json::from_str(&events[0].payload_json).unwrap();
    assert_eq!(payload["revision"], 1);
    assert_eq!(payload["vision_changed"], true);
    assert!(payload.get("content").is_none());
}
