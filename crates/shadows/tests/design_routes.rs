use serde_json::{Value, json};
use shadows_core::testing::{EventCursor, acp};
#[path = "fixtures/app.rs"]
mod app;

#[tokio::test]
async fn part_routes_paginate_and_refuse_invalid_references() {
    let app = app::test_app().await;
    let edits = format!("/api/projects/{}/design/edits", app.project);
    let parts = format!("/api/projects/{}/design/parts", app.project);
    let a = uuid::Uuid::new_v4().to_string();
    let b = uuid::Uuid::new_v4().to_string();
    let content =
        json!({"title":" قسم ","responsibility":" مسؤولية ","design":"تصميم\n","kind":null});
    let result = app::call(
        &app,
        "POST",
        &edits,
        Some(json!({"command_id":"parts","expected_revision":0,"ops":[
            {"kind":"PartCreate","id":a,"parent":null,"before":null,"content":content},
            {"kind":"PartCreate","id":b,"parent":a,"before":null,"content":content}
        ]})),
    )
    .await;
    assert_eq!(result, (200, json!({"revision":1})));
    let root = app::call(&app, "GET", &parts, None).await;
    assert_eq!(root.0, 200);
    assert_eq!(root.1["items"].as_array().unwrap().len(), 1);
    let children = app::call(&app, "GET", &format!("{parts}?parent={a}"), None).await;
    assert_eq!(children.0, 200);
    assert_eq!(children.1["items"][0]["id"], b);
    let invalid_cursor =
        app::call(&app, "GET", &format!("{parts}?parent={a}&after={a}"), None).await;
    assert_eq!(invalid_cursor.0, 422);
    assert_eq!(invalid_cursor.1["code"], "INVALID_COMMAND");
    assert!(invalid_cursor.1["message"].as_str().unwrap().contains(&a));
    assert!(
        invalid_cursor.1["message"]
            .as_str()
            .unwrap()
            .contains("cursor")
    );
    let detail = app::call(&app, "GET", &format!("{parts}/{b}"), None).await;
    assert_eq!(detail.1["ancestors"][0]["id"], a);
    assert_eq!(detail.1["part"]["content"]["title"], "قسم");
    let invalid=app::call(&app,"POST",&edits,Some(json!({"command_id":"cycle","expected_revision":1,"ops":[{"kind":"PartMove","id":a,"parent":b,"before":null}]}))).await;
    assert_eq!(invalid.0, 422);
    assert_eq!(invalid.1["code"], "INVALID_COMMAND");
    assert!(invalid.1["message"].as_str().unwrap().contains(&a));
    let events = app
        .storage
        .read_project_events_after(EventCursor(0), &app.project, 100)
        .await
        .unwrap();
    assert_eq!(events.len(), 1);
    let payload: Value = serde_json::from_str(&events[0].payload_json).unwrap();
    assert_eq!(payload["changed_parts"].as_array().unwrap().len(), 2);
    assert_eq!(payload["vision_changed"], false);
}

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
