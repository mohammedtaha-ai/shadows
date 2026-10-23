//! §23.5–§23.6 through the real transports: the Planner proposes and approves
//! a roadmap over MCP, the person approves one over HTTP, and no plan tool
//! works before a roadmap is approved.
use super::*;
use shadows_core::testing::EventCursor;
use shadows_core::{OperationId, ProjectId, Stage, ThreadId};

/// A project at the roadmap stage, its directory written, with a running
/// Planner turn in a conversation of its own.
async fn structured(
    l: &listening::Listening,
    dir: &std::path::Path,
) -> (ProjectId, ThreadId, OperationId) {
    let core = &l.app.core;
    let p = core
        .projects()
        .create(
            "roadmap".into(),
            "roadmap",
            "Roadmap",
            dir.to_str().unwrap(),
        )
        .await
        .unwrap()
        .id;
    core.design().seed_map_for_tests(&p).await.unwrap();
    let ops = ["backend", "database", "api", "frontend"]
        .into_iter()
        .map(|kind| DesignOp::PartCreate {
            id: PartId::generate(),
            parent: None,
            before: None,
            content: PartContent {
                title: kind.into(),
                responsibility: kind.into(),
                kind: Some(kind.into()),
                folders: vec![kind.into()],
                ..Default::default()
            },
        })
        .collect();
    core.plans()
        .edit_design("parts".into(), &p, 0, ops)
        .await
        .unwrap();
    let thread = core
        .threads()
        .create("roadmap-thread".into(), &p, "Roadmap", None, None)
        .await
        .unwrap()
        .id;
    let op = app::start_on(&l.app, thread.as_str(), "hang", app::default_settings()).await;
    for _ in 0..200 {
        if l.app.handles.running_for(&thread).await.as_ref() == Some(&op) {
            return (p, thread, op);
        }
        tokio::time::sleep(std::time::Duration::from_millis(25)).await;
    }
    panic!("turn never became live")
}

async fn part_ids(l: &listening::Listening, p: &ProjectId) -> Vec<PartId> {
    let page = l.app.core.design().parts(p, None, None).await.unwrap();
    page.items.into_iter().map(|part| part.id).collect()
}

fn first(parts: &[PartId], title: &str) -> serde_json::Value {
    json!({"action":"first","outcomes":[{"key":"login","content":{"title":title,
        "intended_result":"People reach their projects","acceptance":["A wrong password fails"]},
        "parts":[parts[0], parts[1]]}]})
}

/// A Planner's client on `thread`, in its own project (not the app's).
async fn planner_client(l: &listening::Listening, thread: &ThreadId) -> listening::Client {
    let p = plan::project_of(&l.app, thread).await;
    let (_, token) = plan::issue_grant(&l.app, "thread", &p, Some(thread)).await;
    listening::mcp_client(&l.base, &token).await
}

#[tokio::test]
async fn the_planner_proposes_and_approves_a_roadmap_after_the_structure() {
    let l = listening::listening_unseeded().await;
    let dir = tempfile::tempdir().unwrap();
    let (p, thread, _) = structured(&l, dir.path()).await;
    let c = planner_client(&l, &thread).await;
    let parts = part_ids(&l, &p).await;
    let (error, text) = listening::call(&c, "roadmap_propose", first(&parts, "Login")).await;
    assert!(error && text.starts_with("STAGE_BLOCKED:"), "{text}");
    assert!(text.contains("press Create structure first"), "{text}");
    let (error, text) = listening::call(
        &c,
        "draft_start",
        json!({"title":"Plan","goal":"Ship","parts":[parts[0]]}),
    )
    .await;
    assert!(error && text.starts_with("STAGE_BLOCKED:"), "{text}");
    assert!(text.contains("no roadmap approved"), "{text}");
    l.app
        .core
        .design()
        .create_structure("write".into(), &p)
        .await
        .unwrap();
    let card = ok(&c, "roadmap_propose", first(&parts, "Login")).await;
    assert_eq!(card["status"], "Pending");
    ok(&c, "roadmap_approve", json!({"proposal_id":card["id"]})).await;
    let approved = &l.app.core.design().roadmap_proposals(&p).await.unwrap()[0];
    let by = approved.approved_by.as_ref().unwrap();
    assert_eq!(by.agent, "planner:claude-code");
    assert!(by.request.is_some(), "{by:?}");
    assert_eq!(
        l.app.core.design().stage(&p).await.unwrap().stage,
        Stage::Plans
    );
    let draft = ok(
        &c,
        "draft_start",
        json!({"title":"Plan","goal":"Ship","parts":[parts[0]]}),
    )
    .await;
    let edited = ok(
        &c,
        "plan_edit",
        json!({"workflow_id":draft["workflow_id"],"expected_revision":0,"ops":[plan::add(1)]}),
    )
    .await;
    ok(
        &c,
        "plan_approve",
        json!({"workflow_id":draft["workflow_id"],"expected_revision":edited["revision"]}),
    )
    .await;
    c.cancel().await.unwrap();
    app::shut_down_app(l.app).await;
}

#[tokio::test]
async fn roadmap_proposal_routes_list_and_approve() {
    let l = listening::listening_unseeded().await;
    let dir = tempfile::tempdir().unwrap();
    let (p, thread, op) = structured(&l, dir.path()).await;
    l.app
        .core
        .design()
        .create_structure("write".into(), &p)
        .await
        .unwrap();
    let c = planner_client(&l, &thread).await;
    let parts = part_ids(&l, &p).await;
    let card = ok(&c, "roadmap_propose", first(&parts, "Login")).await;
    // Without a running turn the Planner cannot approve, as for the map (§24.2).
    l.app.core.turns().stop(&op).await.unwrap();
    for _ in 0..200 {
        if l.app.handles.running_for(&thread).await.is_none() {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(25)).await;
    }
    let (error, text) =
        listening::call(&c, "roadmap_approve", json!({"proposal_id":card["id"]})).await;
    assert!(error, "{text}");
    let list = format!("/api/projects/{p}/roadmap-proposals");
    let (status, cards) = app::call(&l.app, "GET", &list, None).await;
    assert_eq!(status, 200);
    assert_eq!(cards[0]["id"], card["id"]);
    assert_eq!(cards[0]["outcomes"][0]["content"]["title"], "Login");
    let approve = format!("{list}/{}/approve", card["id"].as_str().unwrap());
    for _ in 0..2 {
        let (status, change) = app::call(
            &l.app,
            "POST",
            &approve,
            Some(json!({"command_id":"approved"})),
        )
        .await;
        assert_eq!(status, 200, "{change}");
    }
    let (_, cards) = app::call(&l.app, "GET", &list, None).await;
    assert_eq!(cards[0]["status"], "Approved");
    assert_eq!(cards[0]["approved_by"]["agent"], "human");
    let events = l
        .app
        .storage
        .read_project_events_after(EventCursor(0), &p, 100)
        .await
        .unwrap();
    let kinds: Vec<_> = events
        .iter()
        .map(|e| e.kind.as_str())
        .filter(|k| k.starts_with("Roadmap"))
        .collect();
    assert_eq!(kinds, ["RoadmapProposed", "RoadmapApproved"]);
    c.cancel().await.unwrap();
    app::shut_down_app(l.app).await;
}
