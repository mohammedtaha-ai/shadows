//! `GET /api/projects` marks AYLUL's project `assistant: true` and no other.

use serde_json::Value;
use shadows_core::testing::acp;

#[path = "fixtures/app.rs"]
mod app;

use app::{call, test_app};

#[tokio::test]
async fn projects_list_marks_the_assistant_project() {
    let app = test_app().await;
    let home = tempfile::tempdir().unwrap();
    app.core
        .projects()
        .ensure_assistant("aylul", "AYLUL", home.path())
        .await
        .unwrap();

    let (status, list) = call(&app, "GET", "/api/projects", None).await;
    assert_eq!(status, 200);
    let list = list.as_array().unwrap();
    let flag = |slug: &str| {
        list.iter()
            .find(|p| p["slug"] == slug)
            .map(|p| p["assistant"].clone())
    };
    assert_eq!(flag("aylul"), Some(Value::Bool(true)));
    let others: Vec<&Value> = list.iter().filter(|p| p["slug"] != "aylul").collect();
    assert!(!others.is_empty());
    assert!(others.iter().all(|p| p["assistant"] == Value::Bool(false)));

    app.core
        .shut_down(std::time::Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
}
