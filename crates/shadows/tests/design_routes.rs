use serde_json::{Value, json};
use shadows_core::testing::{EventCursor, acp};
#[path = "fixtures/app.rs"]
mod app;

#[tokio::test]
async fn map_proposal_routes_list_and_approve() {
    use shadows_core::{DesignOp, VisionField};
    let app = app::test_app_unseeded().await;
    let ops = [
        VisionField::Purpose,
        VisionField::Users,
        VisionField::Goals,
        VisionField::Boundaries,
        VisionField::TechnicalDirection,
    ]
    .into_iter()
    .map(|field| DesignOp::VisionItemAdd {
        field,
        text: "synthetic".into(),
    })
    .collect();
    app.core
        .plans()
        .edit_design("vision".into(), &app.project, 0, ops)
        .await
        .unwrap();
    let op = app::start_settled(&app, "hang").await;
    let (grant, _) = shadows_core::Grants::issue_thread_grant_for_test(&app.storage, &app.thread)
        .await
        .unwrap();
    let parts: Vec<_> = ["backend", "database", "api"]
        .into_iter()
        .map(|kind| {
            json!({"key":kind,"content":{"title":kind,"responsibility":kind,"design":"",
            "kind":kind,"folder":kind,"layers":layers(kind),"serves":["G1"]}})
        })
        .collect();
    let mut args = json!({"action":"first","parts":parts,
        "waivers":[{"part":"frontend","reason":"Terminal tool"}]});
    args["parts"].as_array_mut().unwrap().remove(1);
    let bad = app
        .core
        .design()
        .propose_map(&grant, serde_json::from_value(args.clone()).unwrap())
        .await
        .unwrap();
    let list = format!("/api/projects/{}/map-proposals", app.project);
    let (status, _) = app::call(&app, "GET", &list, None).await;
    assert_eq!(status, 200);
    let (status, error) = app::call(
        &app,
        "POST",
        &format!("{list}/{}/approve", bad.id),
        Some(json!({"command_id":"blocked"})),
    )
    .await;
    assert_eq!(status, 409);
    assert_eq!(error["code"], "STAGE_BLOCKED");
    args["parts"] = json!(
        ["backend", "database", "api"]
            .into_iter()
            .map(
                |kind| json!({"key":kind,"content":{"title":kind,"responsibility":kind,"design":"",
            "kind":kind,"folder":kind,"layers":layers(kind),"serves":["G1"]}})
            )
            .collect::<Vec<_>>()
    );
    let good = app
        .core
        .design()
        .propose_map(&grant, serde_json::from_value(args).unwrap())
        .await
        .unwrap();
    let approve = format!("{list}/{}/approve", good.id);
    for _ in 0..2 {
        assert_eq!(
            app::call(
                &app,
                "POST",
                &approve,
                Some(json!({"command_id":"approved"}))
            )
            .await,
            (200, json!({"revision":2}))
        );
    }
    let (foreign, _) = app::other_project(&app).await;
    assert_eq!(
        app::call(
            &app,
            "POST",
            &format!("/api/projects/{foreign}/map-proposals/{}/approve", good.id),
            Some(json!({"command_id":"foreign"}))
        )
        .await
        .0,
        404
    );
    let events = app
        .core
        .events()
        .read_project_events_after(EventCursor(0), &app.project, 100)
        .await
        .unwrap();
    assert_eq!(events.iter().filter(|e| e.kind == "MapProposed").count(), 2);
    app.core.turns().stop(&op).await.unwrap();
    app::shut_down_app(app).await;
}

#[tokio::test]
async fn vision_proposal_routes_list_and_approve() {
    let app = app::test_app().await;
    let op = app::start_settled(&app, "hang").await;
    for _ in 0..200 {
        if app.handles.running_for(&app.thread).await.as_ref() == Some(&op) {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(25)).await;
    }
    let (grant, _) = shadows_core::Grants::issue_thread_grant_for_test(&app.storage, &app.thread)
        .await
        .unwrap();
    let p = app
        .core
        .design()
        .propose_vision(
            &grant,
            shadows_core::VisionPropose {
                action: shadows_core::VisionAction::Add,
                field: Some(shadows_core::VisionField::Goals),
                item_id: None,
                text: Some("Ship".into()),
                replaces: None,
            },
        )
        .await
        .unwrap();
    let list = format!("/api/projects/{}/vision-proposals", app.project);
    let approve = format!("{list}/{}/approve", p.id);
    let (status, pending) = app::call(&app, "GET", &list, None).await;
    assert_eq!(status, 200);
    assert_eq!(pending[0]["status"], "Pending");
    let body = json!({"command_id":"approve"});
    assert_eq!(
        app::call(&app, "POST", &approve, Some(body.clone())).await,
        (200, json!({"revision":1}))
    );
    assert_eq!(
        app::call(&app, "POST", &approve, Some(body)).await,
        (200, json!({"revision":1}))
    );
    let done = app::call(&app, "GET", &list, None).await.1;
    assert_eq!(done[0]["status"], "Approved");
    assert_eq!(done[0]["item_id"], "G1");
    let (status, refusal) =
        app::call(&app, "POST", &approve, Some(json!({"command_id":"again"}))).await;
    assert_eq!(status, 422);
    assert_eq!(refusal["code"], "INVALID_COMMAND");
    let events = app
        .core
        .events()
        .read_project_events_after(EventCursor(0), &app.project, 100)
        .await
        .unwrap();
    assert!(events.iter().any(|e| e.kind == "VisionProposed"));
    assert_eq!(
        events
            .iter()
            .filter(|e| e.kind == "ProjectDesignChanged")
            .count(),
        1
    );
    app::shut_down_app(app).await;
}

#[tokio::test]
async fn standards_routes_read_the_base_and_save_additions() {
    let app = app::test_app().await;
    let standards = format!("/api/projects/{}/standards", app.project);
    let (status, body) = app::call(&app, "GET", &standards, None).await;
    assert_eq!(status, 200);
    assert_eq!(body["base"]["parts"][0]["name"], "backend");
    assert_eq!(body["additions"], Value::Null);
    let save = format!("{standards}/additions");
    let content = json!({"rules": [], "parts": [{"name": "billing", "owns": "Payments."}]});
    let request = json!({"command_id": "s1", "content": content});
    let (status, saved) = app::call(&app, "PUT", &save, Some(request.clone())).await;
    assert_eq!(status, 200, "{saved}");
    assert_eq!(saved["number"], 1);
    assert!(saved.get("id").is_none());
    assert_eq!(
        app::call(&app, "PUT", &save, Some(request)).await,
        (200, saved)
    );
    let bad = json!({"command_id":"s2", "content":{"rules":[],"parts":[
        {"name":"api","owns":"x"}]}});
    let (status, failure) = app::call(&app, "PUT", &save, Some(bad)).await;
    assert_eq!(
        (status, failure["code"].as_str()),
        (422, Some("INVALID_COMMAND"))
    );
    let events = app
        .core
        .events()
        .read_project_events_after(EventCursor(0), &app.project, 100);
    let events = events.await.unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].kind, "ProjectStandardsSaved");
    assert_eq!(
        serde_json::from_str::<Value>(&events[0].payload_json).unwrap(),
        json!({"number":1})
    );
    let stage = format!("/api/projects/{}/stage", app.project);
    assert_eq!(
        app::call(&app, "GET", &stage, None).await,
        (
            200,
            json!({"stage":"idea","missing":[],"drift":[],
            "map_revision":1})
        )
    );
    for suffix in ["standards", "stage"] {
        let unknown = format!("/api/projects/{}/{suffix}", uuid::Uuid::new_v4());
        assert_eq!(app::call(&app, "GET", &unknown, None).await.0, 404);
    }

    app.core
        .shut_down(std::time::Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
}

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
        app.core
            .events()
            .read_project_events_after(EventCursor(0), &app.project, 100)
            .await
            .unwrap()
            .len(),
        1
    );

    app.core
        .shut_down(std::time::Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
}

#[tokio::test]
async fn outcome_routes_round_trip_independent_hierarchy_and_references() {
    let app = app::test_app().await;
    shadows_core::AppCore::design_for_tests(app.storage.clone())
        .seed_map_for_tests(&app.project)
        .await
        .unwrap();
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
         "content":{"title":"جزء","responsibility":"","design":"","kind":null,"folder":"part"}},
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
        .core
        .events()
        .read_project_events_after(EventCursor(0), &app.project, 100)
        .await
        .unwrap();
    assert_eq!(events.len(), 1);
    let payload: Value = serde_json::from_str(&events[0].payload_json).unwrap();
    assert_eq!(payload["changed_outcomes"].as_array().unwrap().len(), 2);
    assert_eq!(payload["changed_parts"], json!([part]));
    assert!(!payload.to_string().contains("قبول أول"));

    app.core
        .shut_down(std::time::Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
}

#[tokio::test]
async fn part_routes_paginate_and_refuse_invalid_references() {
    let app = app::test_app_unseeded().await;
    shadows_core::AppCore::design_for_tests(app.storage.clone())
        .seed_map_for_tests(&app.project)
        .await
        .unwrap();
    let edits = format!("/api/projects/{}/design/edits", app.project);
    let parts = format!("/api/projects/{}/design/parts", app.project);
    let a = uuid::Uuid::new_v4().to_string();
    let b = uuid::Uuid::new_v4().to_string();
    let content = json!({
        "folder":"a",
        "title":" قسم ",
        "responsibility":" مسؤولية ",
        "design":"تصميم\n",
        "kind":null
    });
    let mut child_content = content.clone();
    child_content["folder"] = json!("a/b");
    let result = app::call(
        &app,
        "POST",
        &edits,
        Some(json!({"command_id":"parts","expected_revision":0,"ops":[
            {"kind":"PartCreate","id":a,"parent":null,"before":null,"content":content},
            {"kind":"PartCreate","id":b,"parent":a,"before":null,"content":child_content}
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
        .core
        .events()
        .read_project_events_after(EventCursor(0), &app.project, 100)
        .await
        .unwrap();
    assert_eq!(events.len(), 1);
    let payload: Value = serde_json::from_str(&events[0].payload_json).unwrap();
    assert_eq!(payload["changed_parts"].as_array().unwrap().len(), 2);
    assert_eq!(payload["vision_changed"], false);

    app.core
        .shut_down(std::time::Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
}

#[tokio::test]
async fn vision_routes_round_trip_conflict_and_preserve_conversations() {
    let app = app::test_app().await;
    let vision = format!("/api/projects/{}/design/vision", app.project);
    let edits = format!("/api/projects/{}/design/edits", app.project);
    assert_eq!(app::call(&app, "GET", &vision, None).await.1["revision"], 0);
    let item =
        |id: &str, text: &str| json!({"id":id,"text":text,"revision":1,"approved_from":null});
    let content = json!({"purpose":[item("PU1", " رؤية \n")], "users":[item("U1","مطورون")],
        "goals":[item("G1","اهداف")],"boundaries":[],"technical_direction":[item("TD1","Rust")]});
    let request = json!({
        "command_id": "vision",
        "expected_revision": 0,
        "ops": [
            {"kind":"VisionItemAdd","field":"purpose","text":" رؤية \n"},
            {"kind":"VisionItemAdd","field":"users","text":"مطورون"},
            {"kind":"VisionItemAdd","field":"goals","text":"اهداف"},
            {"kind":"VisionItemAdd","field":"technical_direction","text":"Rust"}
        ]
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
    changed["ops"][0]["text"] = json!("changed");
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
        .core
        .events()
        .read_project_events_after(EventCursor(0), &app.project, 100)
        .await
        .unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].kind, "ProjectDesignChanged");
    let payload: Value = serde_json::from_str(&events[0].payload_json).unwrap();
    assert_eq!(payload["revision"], 1);
    assert_eq!(payload["vision_changed"], true);
    assert!(payload.get("content").is_none());

    app.core
        .shut_down(std::time::Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
}

/// A project with a complete vision and an approved first map (design revision 2),
/// and a Planner grant for further proposals.
async fn mapped_app() -> (app::App, shadows_core::OperationId, shadows_core::Grant) {
    use shadows_core::{DesignOp, VisionField};
    let app = app::test_app_unseeded().await;
    let ops = [
        VisionField::Purpose,
        VisionField::Users,
        VisionField::Goals,
        VisionField::Boundaries,
        VisionField::TechnicalDirection,
    ]
    .into_iter()
    .map(|field| DesignOp::VisionItemAdd {
        field,
        text: "synthetic".into(),
    })
    .collect();
    app.core
        .plans()
        .edit_design("vision".into(), &app.project, 0, ops)
        .await
        .unwrap();
    let op = app::start_settled(&app, "hang").await;
    let (grant, _) = shadows_core::Grants::issue_thread_grant_for_test(&app.storage, &app.thread)
        .await
        .unwrap();
    let parts: Vec<_> = ["backend", "database", "api"]
        .into_iter()
        .map(|kind| {
            json!({"key":kind,"content":{"title":kind,"responsibility":kind,"design":"",
            "kind":kind,"folder":kind,"layers":layers(kind),"serves":["G1"]}})
        })
        .collect();
    let args = json!({"action":"first","parts":parts,
        "waivers":[{"part":"frontend","reason":"Terminal tool"}]});
    let first = app
        .core
        .design()
        .propose_map(&grant, serde_json::from_value(args).unwrap())
        .await
        .unwrap();
    let approve = format!(
        "/api/projects/{}/map-proposals/{}/approve",
        app.project, first.id
    );
    let approved = app::call(&app, "POST", &approve, Some(json!({"command_id":"first"}))).await;
    assert_eq!(approved, (200, json!({"revision":2})));
    (app, op, grant)
}

fn add_part(folder: Value) -> Value {
    json!({"action":"add","parts":[{"key":"web","content":{"title":"web",
        "responsibility":"web","design":"","folder":folder,"serves":["G1"]}}]})
}

fn layers(kind: &str) -> Vec<shadows_core::PartLayer> {
    shadows_core::Standards::base()
        .parts
        .iter()
        .find(|part| part.name == kind)
        .unwrap()
        .layers
        .iter()
        .filter(|layer| layer.required)
        .map(|layer| shadows_core::PartLayer {
            layer: layer.id.clone(),
            place: format!("{kind}/{}", layer.id),
            not_used: None,
        })
        .collect()
}

#[tokio::test]
async fn a_map_without_a_required_layer_is_refused_on_every_write_path() {
    let (app, op, grant) = mapped_app().await;
    let mut args = add_part(json!("web"));
    args["parts"][0]["content"]["kind"] = json!("frontend");
    let error = app
        .core
        .design()
        .propose_map(&grant, serde_json::from_value(args.clone()).unwrap())
        .await
        .unwrap_err();
    assert!(error.to_string().contains("screens"), "{error}");

    let mut web_layers = layers("frontend");
    for layer in &mut web_layers {
        layer.place = layer.place.replacen("frontend/", "web/", 1);
    }
    args["parts"][0]["content"]["layers"] = json!(web_layers);
    let pending = app
        .core
        .design()
        .propose_map(&grant, serde_json::from_value(args).unwrap())
        .await
        .unwrap();
    sqlx::query(
        "UPDATE design_map_proposal SET payload_json=\
        json_set(payload_json,'$.parts[0].content.layers',json('[]')) WHERE id=?",
    )
    .bind(&pending.id)
    .execute(app.storage.reader())
    .await
    .unwrap();
    let before = design_changes(&app).await;
    let route = format!(
        "/api/projects/{}/map-proposals/{}/approve",
        app.project, pending.id
    );
    let (status, error) = app::call(
        &app,
        "POST",
        &route,
        Some(json!({"command_id":"missing-layer"})),
    )
    .await;
    assert_eq!(status, 422, "{error}");
    assert!(error["message"].as_str().unwrap().contains("screens"));
    assert_eq!(design_changes(&app).await, before);

    let route = format!("/api/projects/{}/design/edits", app.project);
    let (status, error) = app::call(
        &app,
        "POST",
        &route,
        Some(json!({
            "command_id":"missing-layer-edit", "expected_revision":2, "ops":[{
                "kind":"PartCreate", "id":uuid::Uuid::new_v4().to_string(),
                "parent":null,"before":null,"content":{
                    "title":"web","kind":"frontend","folder":"web",
                    "responsibility":"web","design":""
                }
            }]
        })),
    )
    .await;
    assert_eq!(status, 422, "{error}");
    assert!(
        error["message"].as_str().unwrap().contains("screens"),
        "{error}"
    );
    assert_eq!(design_changes(&app).await, before);
    app.core.turns().stop(&op).await.unwrap();
    app::shut_down_app(app).await;
}

#[tokio::test]
async fn empty_optional_places_are_refused_on_every_write_path() {
    for rule in ["missing_layer"] {
        let (app, op, grant) = mapped_app().await;
        let layers: Vec<_> = ["screens", "features", "data_access"]
            .into_iter()
            .map(|id| json!({"layer":id,"place":format!("web/{id}"),"not_used":null}))
            .collect();
        let valid = json!({"title":"web","responsibility":"Client","design":"",
            "kind":"frontend","folder":"web","layers":layers,"serves":["G1"]});
        let mut invalid = valid.clone();
        invalid["layers"]
            .as_array_mut()
            .unwrap()
            .push(json!({"layer":"design_system","place":"","not_used":null}));
        let propose = |content| json!({"action":"add","parts":[{"key":"web","content":content}]});
        let error = app
            .core
            .design()
            .propose_map(
                &grant,
                serde_json::from_value(propose(invalid.clone())).unwrap(),
            )
            .await
            .unwrap_err();
        assert!(format!("{error:?}").contains(rule), "{error}");
        let pending = app
            .core
            .design()
            .propose_map(&grant, serde_json::from_value(propose(valid)).unwrap())
            .await
            .unwrap();
        sqlx::query(
            "UPDATE design_map_proposal SET payload_json=\
            json_set(payload_json,'$.parts[0].content',json(?)) WHERE id=?",
        )
        .bind(invalid.to_string())
        .bind(&pending.id)
        .execute(app.storage.reader())
        .await
        .unwrap();
        let before = design_changes(&app).await;
        let route = format!(
            "/api/projects/{}/map-proposals/{}/approve",
            app.project, pending.id
        );
        let (status, error) = app::call(
            &app,
            "POST",
            &route,
            Some(json!({"command_id":"invalid-layer"})),
        )
        .await;
        assert_eq!(status, 422, "{error}");
        assert!(error["message"].as_str().unwrap().contains(rule), "{error}");
        let route = format!("/api/projects/{}/design/edits", app.project);
        let (status, error) = app::call(
            &app,
            "POST",
            &route,
            Some(json!({
                "command_id":"invalid-layer-edit","expected_revision":2,"ops":[{
                    "kind":"PartCreate","id":uuid::Uuid::new_v4().to_string(),
                    "parent":null,"before":null,"content":invalid,
                }]
            })),
        )
        .await;
        assert_eq!(status, 422, "{error}");
        assert!(error["message"].as_str().unwrap().contains(rule), "{error}");
        assert_eq!(design_changes(&app).await, before);
        app.core.turns().stop(&op).await.unwrap();
        app::shut_down_app(app).await;
    }
}

async fn design_changes(app: &app::App) -> usize {
    app.core
        .events()
        .read_project_events_after(EventCursor(0), &app.project, 100)
        .await
        .unwrap()
        .iter()
        .filter(|e| e.kind == "ProjectDesignChanged")
        .count()
}

async fn root_parts(app: &app::App) -> usize {
    let parts = format!("/api/projects/{}/design/parts", app.project);
    let (status, body) = app::call(app, "GET", &parts, None).await;
    assert_eq!(status, 200);
    body["items"].as_array().unwrap().len()
}

#[tokio::test]
async fn a_new_part_without_a_folder_is_refused_on_every_write_path() {
    let (app, op, grant) = mapped_app().await;
    let refused = |message: &str| message.contains("Give the part one folder");

    // Proposal: the Planner's add is refused before it is recorded.
    let proposal = app
        .core
        .design()
        .propose_map(&grant, serde_json::from_value(add_part(json!(""))).unwrap())
        .await
        .unwrap_err();
    assert!(refused(&proposal.to_string()), "{proposal}");

    // Approval: a proposal recorded before the rule is rechecked as a whole candidate.
    let pending = app
        .core
        .design()
        .propose_map(
            &grant,
            serde_json::from_value(add_part(json!("web"))).unwrap(),
        )
        .await
        .unwrap();
    // Represents a proposal recorded before a part needed a folder.
    sqlx::query(
        "UPDATE design_map_proposal
         SET payload_json=json_set(payload_json,'$.parts[0].content.folder',json_quote(''))
         WHERE id=?",
    )
    .bind(&pending.id)
    .execute(app.storage.reader())
    .await
    .unwrap();
    let before = design_changes(&app).await;
    let approve = format!(
        "/api/projects/{}/map-proposals/{}/approve",
        app.project, pending.id
    );
    let (status, error) =
        app::call(&app, "POST", &approve, Some(json!({"command_id":"old"}))).await;
    assert_eq!(status, 422, "{error}");
    assert_eq!(error["code"], "MAP_REFUSED");
    assert!(refused(error["message"].as_str().unwrap()), "{error}");
    assert_eq!(design_changes(&app).await, before);
    assert_eq!(root_parts(&app).await, 3);

    // Person edit.
    let edits = format!("/api/projects/{}/design/edits", app.project);
    let body = json!({"command_id":"nofolder","expected_revision":2,"ops":[
        {"kind":"PartCreate","id":uuid::Uuid::new_v4().to_string(),"parent":null,
         "before":null,"content":{"title":"x","responsibility":"","design":"","kind":null}}
    ]});
    let (status, error) = app::call(&app, "POST", &edits, Some(body)).await;
    assert_eq!(status, 422, "{error}");
    assert_eq!(error["code"], "MAP_REFUSED");
    assert!(refused(error["message"].as_str().unwrap()), "{error}");
    assert_eq!(root_parts(&app).await, 3);

    app.core.turns().stop(&op).await.unwrap();
    app::shut_down_app(app).await;
}

#[tokio::test]
async fn a_stale_map_approval_writes_nothing() {
    let (app, op, grant) = mapped_app().await;
    let pending = app
        .core
        .design()
        .propose_map(
            &grant,
            serde_json::from_value(add_part(json!("web"))).unwrap(),
        )
        .await
        .unwrap();
    // The map moves after the proposal: a person gives "web" to another part.
    let edits = format!("/api/projects/{}/design/edits", app.project);
    let body = json!({"command_id":"claim","expected_revision":2,"ops":[
        {"kind":"PartCreate","id":uuid::Uuid::new_v4().to_string(),"parent":null,
         "before":null,"content":{"title":"site","responsibility":"","design":"",
         "kind":null,"folder":"web"}}
    ]});
    assert_eq!(
        app::call(&app, "POST", &edits, Some(body)).await,
        (200, json!({"revision":3}))
    );
    let before = design_changes(&app).await;
    let approve = format!(
        "/api/projects/{}/map-proposals/{}/approve",
        app.project, pending.id
    );
    let (status, error) =
        app::call(&app, "POST", &approve, Some(json!({"command_id":"stale"}))).await;
    assert_eq!(status, 422, "{error}");
    assert_eq!(error["code"], "MAP_REFUSED");
    assert!(
        error["message"]
            .as_str()
            .unwrap()
            .contains("folder_claimed_twice"),
        "{error}"
    );
    assert_eq!(design_changes(&app).await, before);
    assert_eq!(root_parts(&app).await, 4);
    let list = format!("/api/projects/{}/map-proposals", app.project);
    let (_, proposals) = app::call(&app, "GET", &list, None).await;
    let stored = proposals
        .as_array()
        .unwrap_or_else(|| panic!("{proposals}"))
        .iter()
        .find(|p| p["id"] == pending.id)
        .unwrap();
    assert_eq!(stored["status"], "Pending", "{stored}");
    // The person's edit stands; the next write starts from revision 3.
    let body = json!({"command_id":"after","expected_revision":3,"ops":[
        {"kind":"PartCreate","id":uuid::Uuid::new_v4().to_string(),"parent":null,
         "before":null,"content":{"title":"docs","responsibility":"","design":"",
         "kind":null,"folder":"docs"}}
    ]});
    assert_eq!(
        app::call(&app, "POST", &edits, Some(body)).await,
        (200, json!({"revision":4}))
    );

    app.core.turns().stop(&op).await.unwrap();
    app::shut_down_app(app).await;
}
