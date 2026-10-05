//! A conversation's harness and a project's allowed modes over HTTP (spec
//! §12.5, §12.6, §12.10): chosen at creation, changeable until the first turn,
//! refused outside the policy, and kept across a restart with the remembered
//! settings and limits.

use serde_json::{Value, json};

use shadows_core::testing::acp;
#[path = "fixtures/app.rs"]
mod app;
#[path = "fixtures/plan.rs"]
mod plan;

use app::{
    call, create_thread, default_settings, fresh_command, get_json, http_start, patch, post,
    shut_down_app, start_and_finish, start_and_finish_on, start_settled, test_app, test_app_at,
};

/// A removed conversation leaves the sidebar, refuses new work, and keeps its
/// history and project-owned plan while its running turn is cancelled.
#[tokio::test]
async fn a_deleted_conversation_is_gone_but_readable() {
    let app = test_app().await;
    let draft = plan::draft(&app).await;
    plan::edit(&app, &draft.workflow_id, 0, &[plan::add(1)]).await;
    let writer = plan::planner(&app, &app.thread).await;
    let op = start_settled(&app, "hang").await;

    let path = format!("/api/threads/{}?command_id=d1", app.thread);
    let ((status, removed), (attempted, refused, body)) =
        tokio::join!(call(&app, "DELETE", &path, None), async {
            // The row is committed before Stop waits for the hanging turn.
            tokio::time::timeout(std::time::Duration::from_secs(20), async {
                loop {
                    let removed_at: Option<String> =
                        sqlx::query_scalar("SELECT removed_at FROM planning_thread WHERE id = ?")
                            .bind(app.thread.as_str())
                            .fetch_one(app.storage.reader())
                            .await
                            .unwrap();
                    if removed_at.is_some() {
                        break;
                    }
                    tokio::task::yield_now().await;
                }
            })
            .await
            .expect("the delete must commit before Stop finishes");
            let attempted = app
                .storage
                .edit_plan(
                    &plan::edit_ctx(&writer, &draft.workflow_id, 1, &[plan::add(2)]),
                    &writer,
                    None,
                    &draft.workflow_id,
                    1,
                    &[plan::add(2)],
                )
                .await;
            let (refused, body) = http_start(
                &app,
                app.thread.as_str(),
                json!({
                    "command_id": fresh_command(), "prompt": "no", "model": "fake-large",
                    "mode": "acceptEdits", "effort": "high"
                }),
            )
            .await;
            (attempted, refused, body)
        });
    assert_eq!(status, 200, "{removed}");
    assert!(removed["removed_at"].is_string());
    assert_eq!(
        app.storage.get_operation(&op).await.unwrap().status_kind,
        "Cancelled"
    );
    assert_eq!(app.sessions.live_count().await, 0);

    let threads: Vec<Value> =
        get_json(&app, &format!("/api/projects/{}/threads", app.project)).await;
    assert!(!threads.iter().any(|t| t["id"] == app.thread.as_str()));
    let loaded: Value = get_json(&app, &format!("/api/threads/{}", app.thread)).await;
    assert_eq!(loaded["removed_at"], removed["removed_at"]);
    let _: Vec<Value> = get_json(&app, &format!("/api/threads/{}/entries", app.thread)).await;
    assert_eq!(
        (refused, body["code"].as_str()),
        (404, Some("INVALID_COMMAND"))
    );

    assert!(matches!(
        attempted,
        Err(shadows_core::StorageError::GrantInvalid)
    ));
    assert_eq!(
        app.storage
            .get_plan(&draft.workflow_id)
            .await
            .unwrap()
            .revision,
        1
    );
    assert!(matches!(
        app.storage.issue_thread_grant(&app.thread).await,
        Err(shadows_core::StorageError::NotFound(_))
    ));
    let plans: Vec<Value> =
        get_json(&app, &format!("/api/projects/{}/workflows", app.project)).await;
    assert_eq!(plans.len(), 1);
    let plan: Value = get_json(&app, &format!("/api/workflows/{}", draft.workflow_id)).await;
    assert_eq!(plan["written_by"]["thread_removed"], true);

    let (again_status, again) = call(&app, "DELETE", &path, None).await;
    assert_eq!((again_status, again), (200, removed));
    let (project_status, _) = call(
        &app,
        "DELETE",
        &format!("/api/projects/{}?command_id=pd1", app.project),
        None,
    )
    .await;
    assert_eq!(project_status, 200);
}

#[tokio::test]
async fn a_thread_can_be_created_on_a_harness_and_changed_until_its_first_turn() {
    let app = test_app().await;
    let t = create_thread(
        &app,
        json!({ "command_id": "c9", "title": "x", "harness": "codex" }),
    )
    .await;
    assert_eq!(t["harness"], "codex");
    assert!(t["forked_from_thread"].is_null());
    let id = t["id"].as_str().unwrap();
    let path = format!("/api/threads/{id}");
    let (s, t2) = patch(
        &app,
        &path,
        json!({ "command_id": "h1", "harness": "claude-code" }),
    )
    .await;
    assert_eq!((s, t2["harness"].as_str()), (200, Some("claude-code")));
    let done = start_and_finish_on(&app, id, "hi", default_settings()).await;
    assert_eq!(done.status_kind, "Completed");
    let (s3, b3) = patch(
        &app,
        &path,
        json!({ "command_id": "h2", "harness": "codex" }),
    )
    .await;
    assert_eq!((s3, b3["code"].as_str()), (409, Some("HARNESS_LOCKED")));
    let (s4, b4) = patch(
        &app,
        &path,
        json!({ "command_id": "h1", "harness": "claude-code" }),
    )
    .await;
    assert_eq!((s4, b4["harness"].as_str()), (200, Some("claude-code")));
}

#[tokio::test]
async fn a_thread_created_without_a_harness_runs_on_claude_code() {
    let app = test_app().await;
    let t = create_thread(&app, json!({ "command_id": "c9", "title": "x" })).await;
    assert_eq!(t["harness"], "claude-code");
}

#[tokio::test]
async fn changing_the_harness_closes_the_threads_adapter() {
    let app = test_app().await;
    post(
        &app,
        &format!("/api/threads/{}/session", app.thread),
        json!({}),
    )
    .await;
    assert_eq!(app.sessions.live_count().await, 1);
    let (s, _) = patch(
        &app,
        &format!("/api/threads/{}", app.thread),
        json!({ "command_id": "h1", "harness": "codex" }),
    )
    .await;
    assert_eq!(s, 200);
    assert_eq!(app.sessions.live_count().await, 0);
}

#[tokio::test]
async fn an_unknown_harness_or_a_mode_outside_the_policy_is_refused_when_set() {
    let app = test_app().await;
    let (s, b) = patch(
        &app,
        &format!("/api/threads/{}", app.thread),
        json!({ "command_id": "h1", "harness": "gemini" }),
    )
    .await;
    assert_eq!((s, b["code"].as_str()), (422, Some("SETTING_NOT_OFFERED")));
    let (s2, b2) = patch(
        &app,
        &format!("/api/projects/{}", app.project),
        json!({ "command_id": "m1", "allowed_modes": { "claude-code": ["bypassPermissions"] } }),
    )
    .await;
    assert_eq!(
        (s2, b2["code"].as_str()),
        (422, Some("SETTING_NOT_OFFERED"))
    );
    let (s3, b3) = post(
        &app,
        &format!("/api/projects/{}/threads", app.project),
        json!({ "command_id": "c9", "title": "x", "harness": "gemini" }),
    )
    .await;
    assert_eq!(
        (s3, b3["code"].as_str()),
        (422, Some("SETTING_NOT_OFFERED"))
    );
}

#[tokio::test]
async fn a_project_s_modes_can_be_replaced_and_a_replay_changes_nothing() {
    let app = test_app().await;
    let path = format!("/api/projects/{}", app.project);
    let body = json!({ "command_id": "m1", "allowed_modes": { "claude-code": ["acceptEdits"] } });
    let (s, p) = patch(&app, &path, body.clone()).await;
    assert_eq!(
        (s, &p["allowed_modes"]["claude-code"]),
        (200, &json!(["acceptEdits"]))
    );
    let (s2, again) = patch(&app, &path, body).await;
    assert_eq!((s2, again), (200, p));
    let (s3, b3) = patch(
        &app,
        &path,
        json!({ "command_id": "m1", "allowed_modes": { "claude-code": ["auto"] } }),
    )
    .await;
    assert_eq!((s3, b3["code"].as_str()), (409, Some("COMMAND_CONFLICT")));
}

#[tokio::test]
async fn a_turn_on_a_codex_thread_is_harness_unavailable() {
    let app = test_app().await;
    let t = create_thread(
        &app,
        json!({ "command_id": "c9", "title": "x", "harness": "codex" }),
    )
    .await;
    let (s, b) = http_start(
        &app,
        t["id"].as_str().unwrap(),
        json!({
            "command_id": "t1",
            "prompt": "hi",
            "model": "fake-small",
            "mode": "acceptEdits",
            "effort": "high"
        }),
    )
    .await;
    assert_eq!((s, b["code"].as_str()), (422, Some("HARNESS_UNAVAILABLE")));
}

#[tokio::test]
async fn settings_and_limits_survive_a_restart() {
    let dir = tempfile::tempdir().unwrap();
    let app = test_app_at(dir.path()).await;
    let done = start_and_finish(
        &app,
        "usage",
        json!({ "model": "fake-large", "mode": "auto", "effort": "max" }),
    )
    .await;
    assert_eq!(done.status_kind, "Completed");
    patch(
        &app,
        &format!("/api/projects/{}", app.project),
        json!({ "command_id": "m1", "allowed_modes": { "claude-code": ["acceptEdits"] } }),
    )
    .await;
    shut_down_app(app).await;
    let app = test_app_at(dir.path()).await;
    let h: Vec<Value> = get_json(&app, "/api/harnesses").await;
    assert_eq!(
        h[0]["remembered"],
        json!({ "model": "fake-large", "effort": "max" })
    );
    assert!(h[0]["limits"]["seven_day"].is_object());
    let p: Vec<Value> = get_json(&app, "/api/projects").await;
    assert_eq!(p[0]["allowed_modes"]["claude-code"], json!(["acceptEdits"]));
    let ops: Vec<Value> = get_json(&app, &format!("/api/threads/{}/operations", app.thread)).await;
    assert_eq!(ops[0]["invocation"]["context_used"], 1234);
    shut_down_app(app).await;
}

#[tokio::test]
async fn a_browser_may_send_patch_from_an_allowed_origin() {
    use axum::body::Body;
    use axum::http::Request;
    use tower::ServiceExt;
    let origin = "http://localhost:5173";
    let app = test_app().await;
    let state_router = shadows::cli::router(shadows_http::AppState {
        core: app.core.clone(),
        allowed_origins: vec![origin.to_string()],
        shutdown: tokio::sync::watch::channel(false).1,
    });
    let preflight = state_router
        .oneshot(
            Request::builder()
                .method("OPTIONS")
                .uri(format!("/api/threads/{}", app.thread))
                .header("origin", origin)
                .header("access-control-request-method", "PATCH")
                .header("access-control-request-headers", "content-type")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let methods = preflight
        .headers()
        .get("access-control-allow-methods")
        .map(|v| v.to_str().unwrap().to_string())
        .unwrap_or_default();
    assert!(methods.contains("PATCH"), "{methods}");
    // Picking a model sends a PUT (§12.7); the allowed list is one list.
    assert!(methods.contains("PUT"), "{methods}");
}
