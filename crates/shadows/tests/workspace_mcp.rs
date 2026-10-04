//! §18.10: a Planner reads the workspace a person wrote, and cannot write it.
use serde_json::json;
use shadows_core::testing::acp;
use shadows_core::{DesignOp, PartContent, PartId, VisionContent};
#[path = "fixtures/app.rs"]
mod app;
#[path = "fixtures/listening.rs"]
mod listening;
#[path = "fixtures/plan.rs"]
mod plan;
use listening::{listening_app, ok, thread_client};

#[tokio::test]
async fn a_planner_reads_the_workspace_a_person_wrote() {
    let l = listening_app().await;
    let vision = VisionContent {
        purpose: "Review pull requests".into(),
        users: "Small teams".into(),
        goals: "Comment in under a minute".into(),
        boundaries: "Never merges".into(),
        technical_direction: "Nemotron on Nebius".into(),
    };
    let part = PartContent {
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
            vec![
                DesignOp::VisionPut { content: vision },
                DesignOp::PartCreate {
                    id: PartId::generate(),
                    parent: None,
                    before: None,
                    content: part,
                },
            ],
        )
        .await
        .unwrap();

    let planner = thread_client(&l, &l.app.thread).await;
    let read = ok(&planner, "workspace_get", json!({})).await;
    assert_eq!(read["vision"]["content"]["purpose"], "Review pull requests");
    assert_eq!(read["parts"]["items"][0]["content"]["title"], "Backend");
    assert_eq!(read["outcomes"]["items"], json!([]));
}
