use serde_json::json;
use shadows_core::testing::acp;
use shadows_core::{Grant, VisionProposal};
#[path = "fixtures/app.rs"]
mod app;

async fn propose(app: &app::App, grant: &Grant, text: &str) -> VisionProposal {
    app.core
        .design()
        .propose_vision(
            grant,
            serde_json::from_value(json!({"action":"add", "field":"technical_direction",
            "text":text}))
            .unwrap(),
        )
        .await
        .unwrap()
}

#[tokio::test]
async fn an_added_item_keeps_the_id_it_was_proposed_with() {
    let app = app::test_app().await;
    let op = app::start_settled(&app, "hang").await;
    let (grant, _) = shadows_core::Grants::issue_thread_grant_for_test(&app.storage, &app.thread)
        .await
        .unwrap();
    let first = propose(&app, &grant, "Laravel").await;
    let second = propose(&app, &grant, "Telegram").await;
    assert_eq!(first.item_id.as_deref(), Some("TD1"));
    assert_eq!(second.item_id.as_deref(), Some("TD2"));
    assert_eq!(propose(&app, &grant, "Laravel").await, first);
    app.core
        .design()
        .approve_vision_proposal("approve-second".into(), &app.project, &second.id)
        .await
        .unwrap();
    app.core
        .design()
        .approve_vision_proposal("approve-first".into(), &app.project, &first.id)
        .await
        .unwrap();
    let vision = app.core.design().vision(&app.project).await.unwrap();
    assert!(
        vision
            .content
            .technical_direction
            .iter()
            .any(|item| item.id == "TD1" && item.text == "Laravel")
    );
    assert!(
        vision
            .content
            .technical_direction
            .iter()
            .any(|item| item.id == "TD2" && item.text == "Telegram")
    );
    let third = propose(&app, &grant, "Mercure").await;
    assert_eq!(third.item_id.as_deref(), Some("TD3"));
    let entries = serde_json::to_value(app::entries(&app).await).unwrap();
    assert!(
        entries
            .as_array()
            .unwrap()
            .iter()
            .any(|entry| entry["card"]["item_id"] == "TD1")
    );
    app.core.turns().stop(&op).await.unwrap();
    app::shut_down_app(app).await;
}

#[tokio::test]
async fn an_old_proposal_without_a_reserved_id_still_approves() {
    let app = app::test_app().await;
    let counter_json: String =
        sqlx::query_scalar("SELECT vision_counters FROM design_workspace WHERE project_id=?")
            .bind(app.project.as_str())
            .fetch_one(app.storage.reader())
            .await
            .unwrap();
    let op = app::start_settled(&app, "hang").await;
    let (grant, _) = shadows_core::Grants::issue_thread_grant_for_test(&app.storage, &app.thread)
        .await
        .unwrap();
    let proposal = propose(&app, &grant, "Legacy direction").await;
    sqlx::query("UPDATE design_vision_proposal SET reserved_item_id=NULL,item_id=NULL WHERE id=?")
        .bind(&proposal.id)
        .execute(app.storage.reader())
        .await
        .unwrap();
    sqlx::query("UPDATE design_workspace SET vision_counters=? WHERE project_id=?")
        .bind(counter_json)
        .bind(app.project.as_str())
        .execute(app.storage.reader())
        .await
        .unwrap();
    app.core
        .design()
        .approve_vision_proposal("approve".into(), &app.project, &proposal.id)
        .await
        .unwrap();
    let vision = app.core.design().vision(&app.project).await.unwrap();
    assert!(
        vision
            .content
            .technical_direction
            .iter()
            .any(|item| item.id == "TD1" && item.text == "Legacy direction")
    );
    app.core.turns().stop(&op).await.unwrap();
    app::shut_down_app(app).await;
}
