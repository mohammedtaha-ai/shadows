//! A Planner reads the workspace a person wrote, and cannot write it.
use serde_json::json;
use shadows_core::testing::acp;
use shadows_core::{DesignOp, PartContent, PartId, VisionField};
#[path = "fixtures/app.rs"]
mod app;
#[path = "fixtures/listening.rs"]
mod listening;
#[path = "fixtures/plan.rs"]
mod plan;
use listening::{listening_app, ok, thread_client};
#[path = "workspace_mcp/approval.rs"]
mod approval;
#[path = "workspace_mcp/delivery.rs"]
mod delivery;
#[path = "workspace_mcp/roadmap.rs"]
mod roadmap;

fn catalog_layers(kind: &str) -> Vec<shadows_core::PartLayer> {
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
async fn map_propose_before_a_complete_vision_answers_stage_blocked() {
    let l = listening_app().await;
    let op = app::start_settled(&l.app, "hang").await;
    let client = thread_client(&l, &l.app.thread).await;
    let (error, text) = listening::call(
        &client,
        "map_propose",
        json!({"action":"first",
        "parts":[{"key":"backend","content":{"title":"Backend","responsibility":"logic",
            "design":"","kind":"backend"}}]}),
    )
    .await;
    assert!(error);
    assert!(text.starts_with("STAGE_BLOCKED:"), "{text}");
    l.app.core.turns().stop(&op).await.unwrap();
    client.cancel().await.unwrap();
    app::shut_down_app(l.app).await;
}

#[tokio::test]
async fn the_planner_proposes_vision_cards_but_an_external_agent_cannot() {
    let l = listening_app().await;
    shadows_core::AppCore::design_for_tests(l.app.storage.clone())
        .seed_map_for_tests(&l.app.project)
        .await
        .unwrap();
    let op = app::start_settled(&l.app, "hang").await;
    for _ in 0..200 {
        if l.app.handles.running_for(&l.app.thread).await.as_ref() == Some(&op) {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(25)).await;
    }
    let planner = thread_client(&l, &l.app.thread).await;
    let result = ok(
        &planner,
        "vision_propose",
        json!({"action":"add","field":"goals","text":"Ship"}),
    )
    .await;
    assert_eq!(result["status"], "Pending");
    assert_eq!(
        l.app
            .core
            .design()
            .vision(&l.app.project)
            .await
            .unwrap()
            .revision,
        0
    );
    let (_, external) = listening::project_client(&l).await;
    let tools = external.list_all_tools().await.unwrap();
    assert!(!tools.iter().any(|t| t.name == "vision_propose"));
    app::shut_down_app(l.app).await;
}

#[tokio::test]
async fn an_external_agent_proposes_vision_cards_but_cannot_approve() {
    let l = listening_app().await;
    let (_, external) = listening::project_client(&l).await;
    let grant = l
        .app
        .core
        .grants()
        .external_project(external.connection(), &l.app.project)
        .await
        .unwrap();
    let design = l.app.core.design();
    let args = json!({
        "action": "add",
        "field": "goals",
        "text": "Ship",
        "command_id": "external-vision",
    });
    let proposal = design
        .propose_vision(&grant, serde_json::from_value(args.clone()).unwrap())
        .await
        .unwrap();
    let replay = design
        .propose_vision(&grant, serde_json::from_value(args).unwrap())
        .await
        .unwrap();
    assert_eq!(replay.id, proposal.id);
    let card = serde_json::to_value(&proposal).unwrap();
    assert_eq!(card["status"], "Pending");
    assert!(
        card.get("thread_id")
            .is_some_and(serde_json::Value::is_null)
    );
    let listed = design.vision_proposals(&l.app.project).await.unwrap();
    let listed = serde_json::to_value(&listed[0]).unwrap();
    assert_eq!(listed["connection_id"], external.connection().as_str());
    let events = l
        .app
        .core
        .events()
        .read_project_events_after(shadows_core::testing::EventCursor(0), &l.app.project, 100)
        .await
        .unwrap();
    let event = events
        .iter()
        .find(|event| event.kind == "VisionProposed")
        .unwrap();
    assert!(event.thread_id.is_none());
    assert!(
        design
            .approve_vision_proposal_as_planner(&grant, &proposal.id)
            .await
            .is_err()
    );
    let missing = design
        .propose_vision(
            &grant,
            serde_json::from_value(json!({"action":"add","field":"goals","text":"Other"})).unwrap(),
        )
        .await
        .unwrap_err();
    assert!(missing.to_string().contains("command_id"), "{missing}");
    external.cancel().await.unwrap();
    app::shut_down_app(l.app).await;
}

#[tokio::test]
async fn a_planner_reads_the_workspace_a_person_wrote() {
    let l = listening::listening_unseeded().await;
    shadows_core::AppCore::design_for_tests(l.app.storage.clone())
        .seed_map_for_tests(&l.app.project)
        .await
        .unwrap();
    let vision: Vec<_> = [
        (VisionField::Purpose, "Review pull requests"),
        (VisionField::Users, "Small teams"),
        (VisionField::Goals, "Comment in under a minute"),
        (VisionField::Boundaries, "Never merges"),
        (VisionField::TechnicalDirection, "Nemotron on Nebius"),
    ]
    .into_iter()
    .map(|(field, text)| DesignOp::VisionItemAdd {
        field,
        text: text.into(),
    })
    .collect();
    let part = PartContent {
        serves: vec![],
        folder: "backend".into(),
        layers: vec![],
        contract: Default::default(),
        title: "Backend".into(),
        responsibility: "Webhook and review flow".into(),
        design: String::new(),
        kind: None,
    };
    l.app
        .core
        .plans()
        .edit_design(
            "workspace".into(),
            &l.app.project,
            0,
            [
                vision,
                vec![DesignOp::PartCreate {
                    id: PartId::generate(),
                    parent: None,
                    before: None,
                    content: part,
                }],
            ]
            .concat(),
        )
        .await
        .unwrap();

    let planner = thread_client(&l, &l.app.thread).await;
    let read = ok(&planner, "workspace_get", json!({})).await;
    assert_eq!(
        read["vision"]["content"]["purpose"][0]["text"],
        "Review pull requests"
    );
    assert_eq!(read["parts"]["items"][0]["content"]["title"], "Backend");
    assert_eq!(read["outcomes"]["items"], json!([]));

    l.app
        .core
        .shut_down(std::time::Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
}
