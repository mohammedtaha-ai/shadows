use serde_json::json;
use shadows_core::testing::acp;
#[path = "fixtures/app.rs"]
mod app;
#[path = "fixtures/listening.rs"]
mod listening;
#[path = "fixtures/plan.rs"]
mod plan;
use listening::{listening_app, ok, project_client};

#[tokio::test]
async fn agreement_mcp_exposes_proposals_without_person_approval() {
    let l = listening_app().await;
    let (grant, client) = project_client(&l).await;
    let tools = client.list_all_tools().await.unwrap();
    let names: Vec<_> = tools.iter().map(|t| t.name.as_ref()).collect();
    assert!(names.contains(&"agreement_list"));
    assert!(!names.contains(&"agreement_agree"));
    let content = json!({"capability":"Login","purpose":"Login","behavior":"",
        "acceptance":[],"parties":[],"openapi":{}});
    let started = ok(
        &client,
        "agreement_start",
        json!({"command_id":"new",
        "content":content}),
    )
    .await;
    assert_eq!(started["writer"]["kind"], "Grant");
    assert_eq!(
        started["writer"]["id"],
        serde_json::to_value(&grant).unwrap()
    );
    let id = started["agreement_id"].clone();
    let read = ok(
        &client,
        "agreement_get",
        json!({"agreement_id":id,"version":1}),
    )
    .await;
    assert_eq!(read["version"], 1);
    l.app
        .core
        .grants()
        .revoke("revoke".into(), &grant)
        .await
        .unwrap();
    assert!(client.list_all_tools().await.is_err());
}
