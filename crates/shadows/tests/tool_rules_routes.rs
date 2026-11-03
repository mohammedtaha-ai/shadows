//! Per-agent MCP rule operations through the real HTTP router and Core.
use axum::{
    Json, Router,
    body::{Body, to_bytes},
    http::Request,
    routing::post,
};
use serde_json::{Value, json};
use shadows_core::{AppCore, AylulConfig, KeySource, SecretKey, StartConfig};
use shadows_http::AppState;
use std::{sync::Arc, time::Duration};
use tower::ServiceExt;

struct World {
    core: Arc<AppCore>,
    router: Router,
    path: String,
    server: tokio::task::JoinHandle<()>,
    _tmp: tempfile::TempDir,
}

impl World {
    async fn request(&self, method: &str, suffix: &str, body: Value) -> (u16, Value) {
        let response = self
            .router
            .clone()
            .oneshot(
                Request::builder()
                    .method(method)
                    .uri(format!("{}{suffix}", self.path))
                    .header("content-type", "application/json")
                    .body(Body::from(body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        let status = response.status().as_u16();
        let bytes = to_bytes(response.into_body(), 65536).await.unwrap();
        (
            status,
            serde_json::from_slice(&bytes).unwrap_or(Value::Null),
        )
    }
    async fn close(self) {
        self.server.abort();
        self.core
            .shut_down(Duration::from_secs(5), std::future::pending())
            .await
            .unwrap();
    }
}

async fn world() -> World {
    let tmp = tempfile::tempdir().unwrap();
    let core = AppCore::start(
        &StartConfig {
            secret_key: KeySource::Given(Some(SecretKey::from_bytes([7; 32]))),
            db_path: tmp.path().join("test.db"),
            node_path: "absent".into(),
            adapter_path: "absent".into(),
            harness_path: "absent".into(),
            aylul: Some(AylulConfig {
                home_root: tmp.path().to_owned(),
                project_slug: "aylul".into(),
            }),
        },
        "http://127.0.0.1/mcp".into(),
    )
    .await
    .unwrap();
    core.projects()
        .ensure_assistant("aylul", "AYLUL", tmp.path())
        .await
        .unwrap();
    core.providers()
        .create_provider(
            "provider".into(),
            serde_json::from_value(json!({
                "name":"Provider","base_url":"https://example.com/anthropic","models":[{"id":"m"}]
            }))
            .unwrap(),
        )
        .await
        .unwrap();
    core.agents()
        .create_agent(
            "agent".into(),
            serde_json::from_value(json!({
                "name":"AYLUL","provider":"provider","default_model":"m"
            }))
            .unwrap(),
        )
        .await
        .unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/mcp", listener.local_addr().unwrap());
    let service = Router::new().route(
        "/mcp",
        post(|Json(q): Json<Value>| async move {
            if q.get("id").is_none() {
                return (axum::http::StatusCode::ACCEPTED, Json(Value::Null));
            }
            let result = if q["method"] == "initialize" {
                json!({"protocolVersion":"2025-03-26","capabilities":{"tools":{}},
                "serverInfo":{"name":"fixture","version":"1"}})
            } else {
                json!({"tools":[{"name":"read","description":"Read a file",
                "inputSchema":{"type":"object"}}]})
            };
            (
                axum::http::StatusCode::OK,
                Json(json!({"jsonrpc":"2.0","id":q["id"],"result":result})),
            )
        }),
    );
    let server = tokio::spawn(async move {
        axum::serve(listener, service).await.unwrap();
    });
    let item = core
        .extensions()
        .add_mcp(
            "mcp".into(),
            serde_json::from_value(json!({
                "name":"docs","transport":{"type":"http","url":url,"headers":[]}
            }))
            .unwrap(),
        )
        .await
        .unwrap();
    let router = shadows::cli::router(AppState {
        core: core.clone(),
        allowed_origins: vec![],
        shutdown: tokio::sync::watch::channel(false).1,
    });
    World {
        core,
        router,
        path: format!("/api/aylul/agents/aylul/extensions/mcp/{}", item.id),
        server,
        _tmp: tmp,
    }
}

#[tokio::test]
async fn tools_and_rules_are_listed() {
    let w = world().await;
    assert_eq!(
        w.request("GET", "/tools", Value::Null).await,
        (
            200,
            json!({
                "tools":[{"name":"read","description":"Read a file"}],"list_error":null,"rules":[]
            })
        )
    );
    w.close().await;
}

#[tokio::test]
async fn rules_are_replaced() {
    let w = world().await;
    let body = json!({"command_id":"one","rules":[{"tool":"read","rule":"allow"}]});
    assert_eq!(
        w.request("PUT", "/tool-rules", body).await,
        (200, json!([{"tool":"read","rule":"allow"}]))
    );
    let body = json!({"command_id":"two","rules":[{"tool":"read","rule":"ask"}]});
    assert_eq!(
        w.request("PUT", "/tool-rules", body).await,
        (200, json!([]))
    );
    assert_eq!(
        w.request("GET", "/tools", Value::Null).await.1["rules"],
        json!([])
    );
    w.close().await;
}

#[tokio::test]
async fn an_unknown_item_is_404() {
    let mut w = world().await;
    w.path = "/api/aylul/agents/aylul/extensions/mcp/absent".into();
    assert_eq!(w.request("GET", "/tools", Value::Null).await.0, 404);
    assert_eq!(
        w.request("PUT", "/tool-rules", json!({"command_id":"x","rules":[]}))
            .await
            .0,
        404
    );
    w.path = "/api/aylul/agents/absent/extensions/mcp/absent".into();
    assert_eq!(w.request("GET", "/tools", Value::Null).await.0, 404);
    w.close().await;
}

#[tokio::test]
async fn a_bad_tool_name_is_400() {
    let w = world().await;
    assert_eq!(
        w.request(
            "PUT",
            "/tool-rules",
            json!({"command_id":"x",
        "rules":[{"tool":"bad name","rule":"deny"}]})
        )
        .await
        .0,
        400
    );
    w.close().await;
}

#[tokio::test]
async fn a_replayed_put_conflicts_on_change() {
    let w = world().await;
    let body = json!({"command_id":"x","rules":[{"tool":"read","rule":"deny"}]});
    let first = w.request("PUT", "/tool-rules", body.clone()).await;
    assert_eq!(first.0, 200);
    assert_eq!(w.request("PUT", "/tool-rules", body).await, first);
    assert_eq!(
        w.request("PUT", "/tool-rules", json!({"command_id":"x","rules":[]}))
            .await
            .0,
        409
    );
    w.close().await;
}

#[tokio::test]
async fn missing_tools_keep_their_rule() {
    let w = world().await;
    assert_eq!(
        w.request(
            "PUT",
            "/tool-rules",
            json!({"command_id":"x",
        "rules":[{"tool":"removed","rule":"deny"}]})
        )
        .await
        .0,
        200
    );
    let (status, body) = w.request("GET", "/tools", Value::Null).await;
    assert_eq!(status, 200);
    assert_eq!(
        body["tools"],
        json!([{"name":"read","description":"Read a file"}])
    );
    assert_eq!(body["rules"], json!([{"tool":"removed","rule":"deny"}]));
    w.close().await;
}
