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

#[tokio::test]
async fn a_planner_reads_the_workspace_a_person_wrote() {
    let l = listening_app().await;
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
