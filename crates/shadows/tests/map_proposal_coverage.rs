use shadows_core::testing::acp;
use shadows_core::{DesignOp, MapAction, MapPropose, PartContent, ProposedPart, VisionField};
#[path = "fixtures/app.rs"]
mod app;

#[tokio::test]
async fn a_map_answer_lists_the_unserved_vision_items() {
    let app = app::test_app().await;
    let part = app.part.clone().unwrap();
    let mut content = app
        .core
        .plans()
        .design_part(&app.project, &part)
        .await
        .unwrap()
        .part
        .content;
    content.serves = vec!["TD1".into()];
    let mut ops: Vec<_> = VisionField::ALL
        .into_iter()
        .map(|field| DesignOp::VisionItemAdd {
            field,
            text: format!("{} item", field.prefix()),
        })
        .collect();
    ops.push(DesignOp::VisionItemAdd {
        field: VisionField::TechnicalDirection,
        text: "Unserved technical direction".into(),
    });
    ops.push(DesignOp::PartPut { id: part, content });
    app.core
        .plans()
        .edit_design("vision".into(), &app.project, 0, ops)
        .await
        .unwrap();
    let op = app::start_settled(&app, "hang").await;
    let (grant, _) = shadows_core::Grants::issue_thread_grant_for_test(&app.storage, &app.thread)
        .await
        .unwrap();
    let proposal = app
        .core
        .design()
        .propose_map(
            &grant,
            MapPropose {
                action: MapAction::Add,
                parts: vec![ProposedPart {
                    key: "future".into(),
                    parent_key: None,
                    parent: None,
                    content: PartContent {
                        title: "Future".into(),
                        responsibility: "Support".into(),
                        folders: vec!["future".into()],
                        serves: vec!["PU1".into(), "G1".into()],
                        ..Default::default()
                    },
                }],
                part_id: None,
                waivers: vec![],
                replaces: None,
            },
        )
        .await
        .unwrap();
    assert_eq!(proposal.unserved, ["U1", "B1", "TD2"]);
    let read = app.core.design().map_proposals(&app.project).await.unwrap();
    assert_eq!(read[0].unserved, proposal.unserved);
    let entries = serde_json::to_value(app::entries(&app).await).unwrap();
    assert!(
        entries
            .as_array()
            .unwrap()
            .iter()
            .any(|entry| entry["card"]["unserved"] == serde_json::json!(["U1", "B1", "TD2"]))
    );
    app.core.turns().stop(&op).await.unwrap();
    app::shut_down_app(app).await;
}
