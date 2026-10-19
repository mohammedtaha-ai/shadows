use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use serde_json::json;
use shadows::daemon::{
    bridge::Bridge,
    http::{StateData, router},
};
use std::{path::PathBuf, sync::Arc};
use tokio::sync::{Mutex, Semaphore};
use tower::ServiceExt;

async fn fixture() -> (tempfile::TempDir, Arc<Mutex<Bridge>>) {
    let root = tempfile::tempdir().unwrap();
    let entry = root.path().join("assistant.mjs");
    std::fs::write(&entry, "test-only peer").unwrap();
    let config = root.path().join("config.json");
    std::fs::write(&config, "{}").unwrap();
    let node = PathBuf::from(env!("CARGO_BIN_EXE_assistant-fixture"));
    let (bridge, _) = Bridge::start(&node, &entry, &config).await.unwrap();
    (root, Arc::new(Mutex::new(bridge)))
}

#[tokio::test]
async fn daemon_requires_capability_and_refuses_browser_origins() {
    let (_root, bridge) = fixture().await;
    let app = router(StateData {
        bridge: bridge.clone(),
        token: "synthetic-capability".into(),
        admission: Arc::new(Semaphore::new(8)),
    });
    for (auth, origin, expected) in [
        (false, false, StatusCode::FORBIDDEN),
        (true, true, StatusCode::FORBIDDEN),
        (true, false, StatusCode::OK),
    ] {
        let mut request = Request::post("/assistant").header("content-type", "application/json");
        if auth {
            request = request.header("authorization", "Bearer synthetic-capability");
        }
        if origin {
            request = request.header("origin", "http://untrusted.test");
        }
        let response = app
            .clone()
            .oneshot(request.body(Body::from(r#"{"action":"status"}"#)).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), expected);
        let bytes = to_bytes(response.into_body(), 131_072).await.unwrap();
        assert!(!String::from_utf8_lossy(&bytes).contains("synthetic-capability"));
    }
    bridge.lock().await.stop();
}

#[tokio::test]
async fn child_death_never_replays_the_last_command() {
    let (_root, bridge) = fixture().await;
    assert_eq!(
        bridge
            .lock()
            .await
            .call(json!({"action":"crash"}))
            .await
            .unwrap_err(),
        "outcome_unknown"
    );
    assert_eq!(
        bridge
            .lock()
            .await
            .call(json!({"action":"status"}))
            .await
            .unwrap_err(),
        "unavailable"
    );
}
