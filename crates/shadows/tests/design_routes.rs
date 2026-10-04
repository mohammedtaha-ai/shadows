use serde_json::{Value, json};
use shadows_core::testing::{EventCursor, acp};
#[path = "fixtures/app.rs"]
mod app;

#[tokio::test]
async fn mixed_workspace_batch_rolls_back_and_outcome_pages_are_bounded() {
    let app = app::test_app().await;
    let edits = format!("/api/projects/{}/design/edits", app.project);
    let outcomes = format!("/api/projects/{}/design/outcomes", app.project);
    let content = json!({
        "title":"نتيجة",
        "intended_result":"آمن",
        "acceptance":["يعمل"]
    });
    let ids: Vec<_> = (0..51).map(|_| uuid::Uuid::new_v4().to_string()).collect();
    let ops: Vec<_> = ids
        .iter()
        .map(|id| {
            json!({"kind":"OutcomeCreate","id":id,"parent":null,"before":null,"content":content})
        })
        .collect();
    assert_eq!(
        app::call(
            &app,
            "POST",
            &edits,
            Some(json!({"command_id":"many","expected_revision":0,"ops":ops}))
        )
        .await
        .0,
        200
    );
    let first = app::call(&app, "GET", &outcomes, None).await.1;
    assert_eq!(first["items"].as_array().unwrap().len(), 50);
    let cursor = first["next"].as_str().unwrap();
    let last = app::call(&app, "GET", &format!("{outcomes}?after={cursor}"), None)
        .await
        .1;
    assert_eq!(last["items"].as_array().unwrap().len(), 1);
    assert_eq!(last["items"][0]["id"], ids[50]);
    assert!(last["next"].is_null());
    let failed = json!({"command_id":"rollback","expected_revision":1,"ops":[
        {"kind":"OutcomePut","id":ids[0],
         "content":{"title":"should rollback","intended_result":"","acceptance":[]}},
        {"kind":"OutcomePartPut","outcome":ids[0],"part":uuid::Uuid::new_v4().to_string()}
    ]});
    assert_ne!(
        app::call(&app, "POST", &edits, Some(failed.clone()))
            .await
            .0,
        200
    );
    let detail = app::call(&app, "GET", &format!("{outcomes}/{}", ids[0]), None)
        .await
        .1;
    assert_eq!(detail["revision"], 1);
    assert_eq!(detail["outcome"]["content"], content);
    assert!(detail["parts"].as_array().unwrap().is_empty());
    assert_ne!(app::call(&app, "POST", &edits, Some(failed)).await.0, 200);
    assert_eq!(
        app.storage
            .read_project_events_after(EventCursor(0), &app.project, 100)
            .await
            .unwrap()
            .len(),
        1
    );
}

#[tokio::test]
async fn outcome_routes_round_trip_independent_hierarchy_and_references() {
    let app = app::test_app().await;
    let edits = format!("/api/projects/{}/design/edits", app.project);
    let outcomes = format!("/api/projects/{}/design/outcomes", app.project);
    let part = uuid::Uuid::new_v4().to_string();
    let root = uuid::Uuid::new_v4().to_string();
    let child = uuid::Uuid::new_v4().to_string();
    let content = json!({
        "title":" نتيجة ",
        "intended_result":" نتائج\n ",
        "acceptance":["قبول أول","قبول ثان"]
    });
    let body = json!({"command_id":"outcomes","expected_revision":0,"ops":[
        {"kind":"PartCreate","id":part,"parent":null,"before":null,
         "content":{"title":"جزء","responsibility":"","design":"","kind":null}},
        {"kind":"OutcomeCreate","id":root,"parent":null,"before":null,"content":content},
        {"kind":"OutcomeCreate","id":child,"parent":root,"before":null,"content":content},
        {"kind":"OutcomePartPut","outcome":child,"part":part}
    ]});
    assert_eq!(
        app::call(&app, "POST", &edits, Some(body)).await,
        (200, json!({"revision":1}))
    );
    let detail = app::call(&app, "GET", &format!("{outcomes}/{child}"), None).await;
    assert_eq!(detail.0, 200);
    assert_eq!(detail.1["outcome"]["content"]["title"], "نتيجة");
    assert_eq!(
        detail.1["outcome"]["content"]["intended_result"],
        " نتائج\n "
    );
    assert_eq!(
        detail.1["outcome"]["content"]["acceptance"],
        content["acceptance"]
    );
    assert_eq!(detail.1["parts"], json!([part]));
    assert_eq!(detail.1["ancestors"][0]["id"], root);
    assert_eq!(
        app::call(&app, "GET", &format!("{outcomes}?parent={root}"), None)
            .await
            .1["items"][0]["id"],
        child
    );
    let invalid = app::call(
        &app,
        "GET",
        &format!("{outcomes}?parent={root}&after={root}"),
        None,
    )
    .await;
    assert_eq!(invalid.0, 422);
    assert_eq!(invalid.1["code"], "INVALID_COMMAND");
    assert!(invalid.1["message"].as_str().unwrap().contains(&root));
    let events = app
        .storage
        .read_project_events_after(EventCursor(0), &app.project, 100)
        .await
        .unwrap();
    assert_eq!(events.len(), 1);
    let payload: Value = serde_json::from_str(&events[0].payload_json).unwrap();
    assert_eq!(payload["changed_outcomes"].as_array().unwrap().len(), 2);
    assert_eq!(payload["changed_parts"], json!([part]));
    assert!(!payload.to_string().contains("قبول أول"));
}

#[tokio::test]
async fn part_routes_paginate_and_refuse_invalid_references() {
    let app = app::test_app().await;
    let edits = format!("/api/projects/{}/design/edits", app.project);
    let parts = format!("/api/projects/{}/design/parts", app.project);
    let a = uuid::Uuid::new_v4().to_string();
    let b = uuid::Uuid::new_v4().to_string();
    let content = json!({
        "title":" قسم ",
        "responsibility":" مسؤولية ",
        "design":"تصميم\n",
        "kind":null
    });
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
    let invalid = app::call(
        &app,
        "POST",
        &edits,
        Some(json!({"command_id":"cycle","expected_revision":1,"ops":[
            {"kind":"PartMove","id":a,"parent":b,"before":null}
        ]})),
    )
    .await;
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
    let content = json!({
        "purpose": " رؤية \n",
        "users": "مطورون",
        "goals": "اهداف",
        "boundaries": "",
        "technical_direction": "Rust"
    });
    let request = json!({
        "command_id": "vision",
        "expected_revision": 0,
        "ops": [{ "kind": "VisionPut", "content": content }]
    });
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
