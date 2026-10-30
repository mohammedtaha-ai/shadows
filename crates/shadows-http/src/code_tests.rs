use crate::aylul_env::tests::{request, world};
use axum::http::StatusCode;
use serde_json::Value;

#[tokio::test]
async fn settings_read_carries_the_bounds_the_write_enforces() {
    let w = world(false, true).await;
    let (status, text) = request(&w, "GET", "/api/code/settings", "").await;
    assert_eq!(status, StatusCode::OK);
    let settings: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(settings["active_limit_min"], 1);
    assert_eq!(settings["active_limit_max"], 20);

    w.core
        .shut_down(std::time::Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
}
