//! Extension routes return metadata and per-agent switches, never source paths.
use crate::aylul_env::tests::{request, world};
use axum::http::StatusCode;
use serde_json::{Value, json};

#[tokio::test]
async fn extension_routes_round_trip() {
    let w = world(true, true).await;
    let provider =
        serde_json::from_value(json!({"preset":"minimax","models":[{"id":"m"}]})).unwrap();
    w.core
        .providers()
        .create_provider("p".into(), provider)
        .await
        .unwrap();
    w.core
        .agents()
        .create_agent(
            "a".into(),
            shadows_core::AgentInput {
                name: "Agent".into(),
                provider: "minimax".into(),
                default_model: "m".into(),
                character_color: None,
            },
        )
        .await
        .unwrap();
    let tmp = tempfile::tempdir().unwrap();
    std::fs::write(
        tmp.path().join("SKILL.md"),
        "---\nname: route-skill\ndescription: Test greeting\n---\nsay hi\n",
    )
    .unwrap();
    let body = json!({"command_id":"import","kind":"skill","path":tmp.path()}).to_string();
    let (status, text) = request(&w, "POST", "/api/aylul/extensions", &body).await;
    assert_eq!(status, StatusCode::CREATED);
    assert!(!text.contains(tmp.path().to_str().unwrap()));
    let item: Value = serde_json::from_str(&text).unwrap();
    let id = item["id"].as_str().unwrap();
    let path = format!("/api/aylul/agents/agent/extensions/skill/{id}");
    let toggle = json!({"command_id":"enable","enabled":true}).to_string();
    assert_eq!(request(&w, "PUT", &path, &toggle).await.0, StatusCode::OK);
    let (status, view) = request(&w, "GET", "/api/aylul/agents/agent/extensions", "").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        serde_json::from_str::<Value>(&view).unwrap()["skills"][0]["enabled"],
        true
    );
    let (status, view) = request(&w, "GET", "/api/aylul/extensions", "").await;
    assert_eq!(status, StatusCode::OK);
    assert!(
        serde_json::from_str::<Value>(&view).unwrap()["skills"][0]
            .get("enabled")
            .is_none()
    );
    let changed = json!({"command_id":"enable","enabled":false}).to_string();
    assert_eq!(
        request(&w, "PUT", &path, &changed).await.0,
        StatusCode::CONFLICT
    );
    assert_eq!(
        request(&w, "POST", "/api/aylul/extensions", r#"{"kind":"bad"}"#)
            .await
            .0,
        StatusCode::BAD_REQUEST
    );
    let del = format!("/api/aylul/extensions/skill/{id}?command_id=delete");
    for _ in 0..2 {
        assert_eq!(
            request(&w, "DELETE", &del, "").await.0,
            StatusCode::NO_CONTENT
        );
    }
    assert_eq!(
        request(
            &w,
            "PUT",
            &path,
            r#"{"command_id":"missing","enabled":true}"#
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );
}
