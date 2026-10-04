//! A project's plan notifications through the real HTTP stream (§16.8).

use std::time::Duration;

use axum::body::{Body, BodyDataStream};
use axum::http::Request;
use serde_json::{Value, json};
use tokio_stream::StreamExt;
use tower::ServiceExt;

use shadows_core::testing::acp;
#[path = "fixtures/app.rs"]
mod app;
#[path = "fixtures/plan.rs"]
mod plan;

async fn next_frame(body: &mut BodyDataStream, pending: &mut String) -> (String, Value) {
    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            if let Some(end) = pending.find("\n\n") {
                let frame: String = pending.drain(..end + 2).collect();
                let event = frame.lines().find_map(|l| l.strip_prefix("event: "));
                let data = frame.lines().find_map(|l| l.strip_prefix("data: "));
                if let (Some(event), Some(data)) = (event, data) {
                    return (event.to_owned(), serde_json::from_str(data).unwrap());
                }
            } else {
                let chunk = body.next().await.expect("stream stays open").unwrap();
                pending.push_str(std::str::from_utf8(&chunk).unwrap());
            }
        }
    })
    .await
    .expect("the committed event reaches the stream")
}

#[tokio::test]
async fn design_stream_replays_resumes_and_keeps_projects_separate() {
    let app = app::test_app().await;
    let (foreign, _) = app::other_project(&app).await;
    let edits = format!("/api/projects/{}/design/edits", app.project);
    let path = format!("/api/projects/{}/events", app.project);
    let vision = json!({"kind":"VisionPut","content":{"purpose":"رؤية","users":"","goals":"",
        "boundaries":"","technical_direction":""}});
    let first = json!({"command_id":"vision","expected_revision":0,"ops":[vision]});
    assert_eq!(
        app::call(&app, "POST", &edits, Some(first.clone())).await.0,
        200
    );
    // Immutable command replay creates no second notification.
    assert_eq!(app::call(&app, "POST", &edits, Some(first)).await.0, 200);
    let response = app
        .router
        .clone()
        .oneshot(Request::get(&path).body(Body::empty()).unwrap())
        .await
        .unwrap();
    let mut body = response.into_body().into_data_stream();
    let mut pending = String::new();
    let (frame, replay) = next_frame(&mut body, &mut pending).await;
    assert_eq!(frame, "durable");
    assert_eq!(replay["kind"], "ProjectDesignChanged");
    assert_eq!(replay["payload"]["vision_changed"], true);
    assert_eq!(next_frame(&mut body, &mut pending).await.0, "caught-up");
    let part = uuid::Uuid::new_v4().to_string();
    let outcome = uuid::Uuid::new_v4().to_string();
    let mixed = json!({"command_id":"mixed","expected_revision":1,"ops":[
        {"kind":"PartCreate","id":part,"parent":null,"before":null,"content":
            {"title":"قسم","responsibility":"","design":"","kind":null}},
        {"kind":"OutcomeCreate","id":outcome,"parent":null,"before":null,"content":
            {"title":"نتيجة","intended_result":"","acceptance":[]}},
        {"kind":"OutcomePartPut","outcome":outcome,"part":part}
    ]});
    let foreign_edit = json!({"command_id":"foreign","expected_revision":0,"ops":[vision]});
    assert_eq!(
        app::call(
            &app,
            "POST",
            &format!("/api/projects/{foreign}/design/edits"),
            Some(foreign_edit)
        )
        .await
        .0,
        200
    );
    assert_eq!(app::call(&app, "POST", &edits, Some(mixed)).await.0, 200);
    let (_, live) = next_frame(&mut body, &mut pending).await;
    assert_eq!(live["payload"]["project_id"], json!(app.project));
    assert_eq!(live["payload"]["changed_parts"], json!([part]));
    assert_eq!(live["payload"]["changed_outcomes"], json!([outcome]));
    drop(body);
    let response = app
        .router
        .clone()
        .oneshot(
            Request::get(format!("{path}?after={}", replay["seq"]))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let mut resumed = response.into_body().into_data_stream();
    let mut pending = String::new();
    assert_eq!(next_frame(&mut resumed, &mut pending).await.1, live);
    assert_eq!(next_frame(&mut resumed, &mut pending).await.0, "caught-up");
    assert!(
        tokio::time::timeout(Duration::from_millis(150), resumed.next())
            .await
            .is_err()
    );
}

#[tokio::test]
async fn a_project_stream_sends_another_conversations_plan_edit() {
    let app = app::test_app().await;
    let (other_project, other_thread) = app::other_project(&app).await;
    let path = format!("/api/projects/{}/events", app.project);
    let response = app
        .router
        .clone()
        .oneshot(Request::get(&path).body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    let mut body = response.into_body().into_data_stream();
    let mut pending = String::new();
    assert_eq!(next_frame(&mut body, &mut pending).await.0, "caught-up");

    // The stream belongs to the project, independently of the conversation.
    let draft = plan::draft(&app).await;
    let (event, started) = next_frame(&mut body, &mut pending).await;
    assert_eq!(event, "durable");
    assert_eq!(started["kind"], "WorkflowDraftStarted");
    assert_eq!(
        started["payload"],
        json!({"plan_id": draft.plan_id, "workflow_id": draft.workflow_id})
    );

    let foreign = plan::draft_on(&app, &other_thread, "foreign-draft").await;
    plan::edit(&app, &foreign.workflow_id, 0, &[plan::add(1)]).await;
    let writer = plan::planner(&app, &app.thread).await;
    let ops = [plan::add(1)];
    app.storage
        .edit_plan(
            &plan::edit_ctx(&writer, &draft.workflow_id, 0, &ops),
            &writer,
            None,
            &draft.workflow_id,
            0,
            &ops,
        )
        .await
        .unwrap();
    let (_, edited) = next_frame(&mut body, &mut pending).await;
    assert_eq!(edited["kind"], "WorkflowEdited");
    assert_eq!(edited["payload"], started["payload"]);
    let seq = edited["seq"].as_i64().unwrap();
    assert!(seq > started["seq"].as_i64().unwrap());

    // A marker proves the first edit arrived once and no foreign event leaked.
    plan::edit(&app, &draft.workflow_id, 1, &[plan::add(2)]).await;
    let (_, marker) = next_frame(&mut body, &mut pending).await;
    assert_eq!(marker["kind"], "WorkflowEdited");
    assert_eq!(marker["payload"], started["payload"]);
    assert!(marker["seq"].as_i64().unwrap() > seq);

    let resumed = app
        .router
        .clone()
        .oneshot(
            Request::get(format!("{path}?after={seq}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let mut resumed = resumed.into_body().into_data_stream();
    let mut replay = String::new();
    assert_eq!(
        next_frame(&mut resumed, &mut replay).await.1["seq"],
        marker["seq"]
    );
    assert_eq!(next_frame(&mut resumed, &mut replay).await.0, "caught-up");

    app.core
        .plans()
        .approve("freeze".into(), &draft.workflow_id, 2)
        .await
        .unwrap();
    assert_eq!(
        next_frame(&mut body, &mut pending).await.1["kind"],
        "WorkflowFrozen"
    );
    app.core
        .plans()
        .archive("archive".into(), &draft.plan_id)
        .await
        .unwrap();
    let (_, archived) = next_frame(&mut body, &mut pending).await;
    assert_eq!(archived["kind"], "PlanArchived");
    assert_eq!(archived["payload"], started["payload"]);
    app.core
        .plans()
        .unarchive("unarchive".into(), &draft.plan_id)
        .await
        .unwrap();
    assert_eq!(
        next_frame(&mut body, &mut pending).await.1["kind"],
        "PlanUnarchived"
    );
    assert!(
        tokio::time::timeout(Duration::from_millis(150), body.next())
            .await
            .is_err()
    );

    let unknown = shadows_core::ProjectId::from_literal(uuid::Uuid::new_v4().to_string());
    assert_eq!(
        app::call(
            &app,
            "GET",
            &format!("/api/projects/{unknown}/events"),
            None
        )
        .await
        .0,
        404
    );
    app.core
        .threads()
        .remove("remove-other".into(), &other_thread)
        .await
        .unwrap();
    app.core
        .projects()
        .remove("remove-project".into(), &other_project)
        .await
        .unwrap();
    assert_eq!(
        app::call(
            &app,
            "GET",
            &format!("/api/projects/{other_project}/events"),
            None
        )
        .await
        .0,
        404
    );
}
