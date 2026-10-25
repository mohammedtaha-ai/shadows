use crate::aylul_env::tests::{World, request, world};
use serde_json::{Value, json};
use shadows_core::{AgreementVersion, ErrorCode, WorkflowId};

async fn http(w: &World, method: &str, path: &str, body: Value) -> Value {
    let (status, text) = request(w, method, path, &body.to_string()).await;
    assert_eq!(status.as_u16(), 200, "{text}");
    serde_json::from_str(&text).unwrap()
}

#[tokio::test]
async fn agreement_adapter_pairs_preserve_content_and_reach() {
    let w = world(true, true).await;
    let project = w.core.agents().assistant_project().await.unwrap();
    let grant = w
        .core
        .grants()
        .issue("grant".into(), &project)
        .await
        .unwrap()
        .grant;
    let root = format!("/api/projects/{project}/agreements");
    let content = json!({"capability":"  Login  ","purpose":"Login","behavior":"",
        "acceptance":[],"parties":[],"openapi":{}});
    let person: AgreementVersion = serde_json::from_value(
        http(
            &w,
            "POST",
            &root,
            json!({"command_id":"person","content":content}),
        )
        .await,
    )
    .unwrap();
    let agent = w
        .core
        .design()
        .start_agreement_for(
            &grant,
            "agent".into(),
            None,
            Some(serde_json::from_value(content.clone()).unwrap()),
            None,
        )
        .await
        .map_err(shadows_core::CoreError::from)
        .unwrap();
    assert_eq!(person.content, agent.content);
    assert_eq!(person.content.capability, "Login");
    assert_eq!(person.state, agent.state);
    assert_ne!(
        serde_json::to_value(&person.writer).unwrap(),
        serde_json::to_value(&agent.writer).unwrap()
    );

    let list = http(&w, "GET", &root, Value::Null).await;
    let scoped = w
        .core
        .design()
        .agreements_for(&grant)
        .await
        .map_err(shadows_core::CoreError::from)
        .unwrap();
    assert_eq!(list, serde_json::to_value(scoped).unwrap());
    let path = format!("{root}/{}", person.agreement_id);
    let read = http(&w, "GET", &path, Value::Null).await;
    let scoped = w
        .core
        .design()
        .agreement_for(&grant, &person.agreement_id, None)
        .await
        .map_err(shadows_core::CoreError::from)
        .unwrap();
    assert_eq!(read, serde_json::to_value(scoped).unwrap());

    let edited = json!({"capability":"Login","purpose":"Changed","behavior":"",
        "acceptance":[],"parties":[],"openapi":{}});
    let person: AgreementVersion = serde_json::from_value(
        http(
            &w,
            "PUT",
            &path,
            json!({"command_id":"person-edit","version":1,"expected_revision":0,
            "content":edited}),
        )
        .await,
    )
    .unwrap();
    let agent = w
        .core
        .design()
        .edit_agreement_for(
            &grant,
            "agent-edit".into(),
            &agent.agreement_id,
            1,
            0,
            serde_json::from_value(edited).unwrap(),
        )
        .await
        .map_err(shadows_core::CoreError::from)
        .unwrap();
    assert_eq!(person.content, agent.content);
    assert_eq!(person.revision, agent.revision);
    let review = http(&w, "GET", &format!("{path}/review"), Value::Null).await;
    let scoped = w
        .core
        .plans()
        .review_agreement_for(&grant, &person.agreement_id)
        .await
        .map_err(shadows_core::CoreError::from)
        .unwrap();
    assert_eq!(review, serde_json::to_value(scoped).unwrap());

    w.core
        .grants()
        .revoke("revoke".into(), &grant.id)
        .await
        .unwrap();
    assert_eq!(
        w.core
            .design()
            .agreements_for(&grant)
            .await
            .map_err(shadows_core::CoreError::from)
            .unwrap_err()
            .code(),
        ErrorCode::GrantInvalid
    );
    http(&w, "GET", &root, Value::Null).await;
}

#[tokio::test]
async fn plan_adapter_pairs_keep_their_authorization_boundaries() {
    let w = world(true, true).await;
    let project = w.core.agents().assistant_project().await.unwrap();
    let grant = w
        .core
        .grants()
        .issue("grant".into(), &project)
        .await
        .unwrap()
        .grant;
    let id = WorkflowId::from_literal("00000000-0000-0000-0000-000000000001");
    let person = w
        .core
        .plans()
        .get(&id)
        .await
        .map_err(shadows_core::CoreError::from)
        .unwrap_err();
    let agent = w
        .core
        .plans()
        .get_for(&grant, Some(&id), None)
        .await
        .map_err(shadows_core::CoreError::from)
        .unwrap_err();
    assert_eq!(person.code(), ErrorCode::InvalidCommand);
    assert_eq!(person.to_string(), "not found: workflow");
    assert_eq!(agent.code(), ErrorCode::GrantScope);
    assert_eq!(
        agent.to_string(),
        "that plan is not in this grant's project"
    );
    let refused = w
        .core
        .plans()
        .approve_as_planner(&grant, &id, 0)
        .await
        .map_err(shadows_core::CoreError::from)
        .unwrap_err();
    assert_eq!(refused.code(), ErrorCode::GrantScope);
    assert_eq!(
        refused.to_string(),
        "approval needs a Planner's conversation"
    );
}
