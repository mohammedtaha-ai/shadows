use std::sync::Arc;

use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use serde_json::{Value, json};
use shadows_core::testing::{LiveHandles, Runtime, Sessions, Storage, acp, secrets};
use shadows_core::{AppCore, AylulConfig, CoreParts, SecretKey};
use tower::ServiceExt;

use crate::AppState;

pub(crate) struct World {
    _tmp: tempfile::TempDir,
    pub(crate) core: Arc<AppCore>,
    router: axum::Router,
}

pub(crate) async fn world(configured: bool, key: bool) -> World {
    let tmp = tempfile::tempdir().unwrap();
    let storage = Arc::new(Storage::open(&tmp.path().join("test.db")).await.unwrap());
    let secrets = secrets(storage.clone(), key.then(|| SecretKey::from_bytes([7; 32])));
    // Every request here must finish before a spawn; an inert adapter makes that explicit.
    let adapter = Arc::new(shadows_agent::claude::ClaudeAdapter {
        node: tmp.path().join("absent-node"),
        adapter: tmp.path().join("absent-adapter.js"),
        agent: tmp.path().join("absent-agent"),
        adapter_version: String::new(),
        agent_version: String::new(),
    });
    let sessions = Sessions::new(
        adapter,
        storage.clone(),
        shadows_core::testing::SessionsConfig {
            secrets: Some(secrets.clone()),
            aylul: configured.then(|| AylulConfig {
                home_root: tmp.path().to_owned(),
                project_slug: "aylul".into(),
            }),
            ..acp::test_config()
        },
    );
    let core = AppCore::assemble(CoreParts {
        secrets,
        runtime: Arc::new(Runtime::start(storage.clone()).await.unwrap().0),
        storage,
        sessions,
        handles: Arc::new(LiveHandles::default()),
        bus: tokio::sync::broadcast::channel(16).0,
        ui: tokio::sync::broadcast::channel(16).0,
        mcp_url: acp::MCP_URL.into(),
    });
    if configured {
        core.projects()
            .ensure_assistant("aylul", "AYLUL", tmp.path())
            .await
            .unwrap();
    }
    let router = crate::router(
        AppState {
            core: core.clone(),
            allowed_origins: vec![],
            shutdown: tokio::sync::watch::channel(false).1,
        },
        axum::Router::new(),
    );
    World {
        _tmp: tmp,
        core,
        router,
    }
}

pub(crate) async fn request(
    w: &World,
    method: &str,
    path: &str,
    text: &str,
) -> (StatusCode, String) {
    let response = w
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method(method)
                .uri(path)
                .header("content-type", "application/json")
                .body(Body::from(text.to_owned()))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), 65536).await.unwrap();
    (status, String::from_utf8(bytes.to_vec()).unwrap())
}

const BODY: &str = r#"{"command_id":"c","value":"synthetic-http-fixture"}"#;

#[tokio::test]
async fn put_then_list_shows_name_only() {
    let w = agent_world(true, true).await;
    assert_eq!(
        request(&w, "PUT", "/api/aylul/agents/aylul/env/API_KEY", BODY)
            .await
            .0,
        StatusCode::OK
    );
    let (status, text) = request(&w, "GET", "/api/aylul/agents/aylul/env", "").await;
    assert_eq!(status, StatusCode::OK);
    let list: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(list[0]["name"], "API_KEY");
    assert!(list[0]["updated_at"].is_string());
    assert_eq!(list[0].as_object().unwrap().len(), 2);
    assert!(
        !text.contains("synthetic-http-fixture"),
        "list contains metadata only"
    );
}

#[tokio::test]
async fn put_response_has_no_value() {
    let w = agent_world(true, true).await;
    let (status, text) = request(&w, "PUT", "/api/aylul/agents/aylul/env/API_KEY", BODY).await;
    assert_eq!(status, StatusCode::OK);
    let entry: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(entry.as_object().unwrap().len(), 2);
    assert!(
        !text.contains("synthetic-http-fixture"),
        "write returns metadata only"
    );
}

#[tokio::test]
async fn bad_name_is_400() {
    let w = agent_world(true, true).await;
    let (status, text) = request(&w, "PUT", "/api/aylul/agents/aylul/env/bad", BODY).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(
        serde_json::from_str::<Value>(&text).unwrap()["code"],
        "SECRET_NAME_INVALID"
    );
}

#[tokio::test]
async fn numeric_value_body_error_does_not_echo() {
    let w = agent_world(true, true).await;
    for body in [
        r#"{"command_id":"c","value":918273645}"#,
        r#"{"command_id":"c","value":"synthetic-http-fixture""#,
    ] {
        let (status, text) = request(&w, "PUT", "/api/aylul/agents/aylul/env/API_KEY", body).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert!(!text.contains("918273645"), "numeric input is never echoed");
        assert!(
            !text.contains("synthetic-http-fixture"),
            "malformed input is never echoed"
        );
        assert_eq!(
            serde_json::from_str::<Value>(&text).unwrap()["message"],
            "the request body is not valid JSON for this route"
        );
    }
}

#[tokio::test]
async fn without_agent_list_is_404() {
    let w = agent_world(false, true).await;
    let (status, text) = request(&w, "GET", "/api/aylul/agents/aylul/env", "").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(
        serde_json::from_str::<Value>(&text).unwrap()["code"],
        "AYLUL_PROFILE_NOT_FOUND"
    );
}

#[tokio::test]
async fn turn_without_key_is_aylul_key_missing() {
    let w = agent_world(true, true).await;
    let p = w
        .core
        .projects()
        .ensure_assistant("aylul", "AYLUL", w._tmp.path())
        .await
        .unwrap();
    let t = w
        .core
        .threads()
        .create("t".into(), &p.id, "talk", Some("aylul"), Some("aylul"))
        .await
        .unwrap();
    let body = json!({"command_id":"turn", "prompt":"hi", "model":"fake-large",
        "mode":"acceptEdits", "effort":"high"})
    .to_string();
    let (status, text) = request(&w, "POST", &format!("/api/threads/{}/turns", t.id), &body).await;
    assert_eq!(status, StatusCode::CONFLICT);
    let error: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(error["code"], "AYLUL_KEY_MISSING");
    assert!(error["message"].as_str().unwrap().contains("test"));
}

#[tokio::test]
async fn delete_returns_204_and_no_key_is_503() {
    let w = agent_world(true, true).await;
    assert_eq!(
        request(&w, "PUT", "/api/aylul/agents/aylul/env/API_KEY", BODY)
            .await
            .0,
        StatusCode::OK
    );
    assert_eq!(
        request(
            &w,
            "DELETE",
            "/api/aylul/agents/aylul/env/API_KEY?command_id=remove",
            ""
        )
        .await,
        (StatusCode::NO_CONTENT, String::new())
    );
    assert!(w.core.secrets().list_env("aylul").await.unwrap().is_empty());
    let unavailable = agent_world(true, false).await;
    let (status, text) = request(&unavailable, "GET", "/api/aylul/agents/aylul/env", "").await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(
        serde_json::from_str::<Value>(&text).unwrap()["code"],
        "SECRETS_UNAVAILABLE"
    );
}

async fn agent_world(configured: bool, key: bool) -> World {
    let w = world(configured, key).await;
    if configured {
        w.core
            .profiles()
            .create_provider(
                "seed-provider".into(),
                shadows_core::ProviderInput {
                    preset: None,
                    label: None,
                    name: Some("Test".into()),
                    base_url: Some("https://example.com".into()),
                    models: vec!["fake-large".into()]
                        .into_iter()
                        .map(|id| shadows_core::ProviderModelInput {
                            id,
                            context_window: None,
                        })
                        .collect(),
                },
            )
            .await
            .unwrap();
        w.core
            .profiles()
            .create_agent(
                "seed-agent".into(),
                shadows_core::AgentInput {
                    name: "AYLUL".into(),
                    provider: "test".into(),
                    default_model: "fake-large".into(),
                },
            )
            .await
            .unwrap();
    }
    w
}
