//! §18.10: a Planner reads the workspace a person wrote, and cannot write it.
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
    l.app
        .storage
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
async fn a_planner_reads_the_workspace_a_person_wrote() {
    let l = listening::listening_unseeded().await;
    l.app
        .storage
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
        folders: vec![],
        files: vec![],
        contract: Default::default(),
        title: "Backend".into(),
        responsibility: "Webhook and review flow".into(),
        design: String::new(),
        kind: None,
    };
    l.app
        .core
        .design()
        .edit(
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
}
