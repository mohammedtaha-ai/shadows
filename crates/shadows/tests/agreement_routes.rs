use serde_json::json;
use shadows_core::testing::acp;
#[path = "fixtures/app.rs"]
mod app;

#[tokio::test]
async fn agreement_routes_edit_incomplete_draft_and_preserve_revision_conflicts() {
    let app = app::test_app().await;
    let root = format!("/api/projects/{}/agreements", app.project);
    assert_eq!(app::call(&app, "GET", &root, None).await.0, 200);
    let content = json!({"capability":"تسجيل الدخول","purpose":"دخول آمن","behavior":"",
        "acceptance":[],"parties":[],"openapi":{}});
    let started = app::call(
        &app,
        "POST",
        &root,
        Some(json!({"command_id":"start",
        "content":content})),
    )
    .await;
    assert_eq!(started.0, 200);
    let id = started.1["agreement_id"].as_str().unwrap();
    assert!(!started.1["issues"].as_array().unwrap().is_empty());
    let path = format!("{root}/{id}");
    let edit = json!({"command_id":"edit","version":1,"expected_revision":0,"content":content});
    assert_eq!(
        app::call(&app, "PUT", &path, Some(edit.clone())).await.0,
        200
    );
    assert_eq!(app::call(&app, "PUT", &path, Some(edit)).await.0, 200);
    let stale = app::call(
        &app,
        "PUT",
        &path,
        Some(json!({"command_id":"stale",
        "version":1,"expected_revision":0,"content":content})),
    )
    .await;
    assert_eq!(stale.0, 409);
    let view = app::call(&app, "GET", &format!("{path}?version=1"), None).await;
    assert_eq!(view.0, 200);
    assert_eq!(view.1["revision"], 1);
    assert_eq!(view.1["content"]["capability"], "تسجيل الدخول");
}
