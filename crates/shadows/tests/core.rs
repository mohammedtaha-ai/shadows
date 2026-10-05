//! Spec §14.4: the application is one `AppCore`; adapters and shutdown reach it through that.

use shadows_core::testing::acp;
#[path = "fixtures/app.rs"]
mod app;
#[path = "fixtures/listening.rs"]
mod listening;
#[path = "fixtures/plan.rs"]
mod plan;

use app::{
    call, default_settings, patch, post, start_and_finish, start_on, test_app, wait_terminal,
};
use listening::{listening_app, mcp_client, ok};
use serde_json::json;
use shadows_core::testing::fingerprint;

#[tokio::test]
async fn the_router_is_built_from_the_core() {
    let app = test_app().await;
    let (status, list) = call(&app, "GET", "/api/projects", None).await;
    assert_eq!(status, 200);
    // `app::names` reads ids; a project's id is generated, so its name is read.
    assert_eq!(list[0]["name"], "Demo");
    assert_eq!(list[0]["id"], app.project.as_str());
    assert_eq!(list.as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn shut_down_through_the_core_cancels_and_records() {
    let app = test_app().await;
    let op = start_on(&app, app.thread.as_str(), "hang", default_settings()).await;
    let kind = app
        .core
        .shut_down(std::time::Duration::from_secs(10), std::future::pending())
        .await
        .unwrap();
    assert_eq!(kind, shadows_core::StopKind::Graceful);
    assert_eq!(wait_terminal(&app, &op).await.status_kind, "Cancelled");
}

/// Spec §14.9: a grant issued and revoked through the routes, and the bearer
/// check on `/mcp`, answer as they did before `Grants` owned them.
#[tokio::test]
async fn revoked_grant_is_refused_after_the_move() {
    let l = listening_app().await;
    let path = format!("/api/projects/{}/mcp-grants", l.app.project);
    let (status, issued) = post(&l.app, &path, json!({ "command_id": "g1" })).await;
    assert_eq!(status, 200, "{issued}");
    let token = issued["token"].as_str().unwrap().to_string();
    let id = issued["grant"]["id"].as_str().unwrap().to_string();

    let client = mcp_client(&l.base, &token).await;
    ok(&client, "workflow_list", json!({})).await;

    let delete = format!("/api/mcp-grants/{id}?command_id=r1");
    let (status, revoked) = call(&l.app, "DELETE", &delete, None).await;
    assert_eq!(status, 200, "{revoked}");

    // `mcp_client` panics on a refused connection: a raw request instead.
    const INITIALIZE: &str = concat!(
        r#"{"jsonrpc":"2.0","id":0,"method":"initialize","#,
        r#""params":{"protocolVersion":"2025-11-25","capabilities":{},"#,
        r#""clientInfo":{"name":"raw","version":"1"}}}"#,
    );
    let response = reqwest::Client::new()
        .post(format!("{}/mcp", l.base))
        .header("content-type", "application/json")
        .header("accept", "application/json, text/event-stream")
        .bearer_auth(&token)
        .body(INITIALIZE)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 401);
    assert_eq!(response.text().await.unwrap(), "");
}

/// A replayed issue or revoke matches a command an earlier daemon recorded
/// only when its principal, kind, schema version and fingerprint parameters
/// are the ones that daemon used. This reads the command log and compares it
/// with each command's parameters as the code before the move wrote them
/// (§14.9).
#[tokio::test]
async fn grant_command_fingerprints_do_not_move() {
    let app = test_app().await;
    let path = format!("/api/projects/{}/mcp-grants", app.project);
    let (status, issued) = post(&app, &path, json!({ "command_id": "pin-issue" })).await;
    assert_eq!(status, 200, "{issued}");
    let grant = issued["grant"]["id"].clone();
    let delete = format!(
        "/api/mcp-grants/{}?command_id=pin-revoke",
        grant.as_str().unwrap()
    );
    let (status, revoked) = call(&app, "DELETE", &delete, None).await;
    assert_eq!(status, 200, "{revoked}");

    let mut recorded: Vec<(String, String, String, String, i64, String)> = sqlx::query_as(
        "SELECT principal_kind, principal_id, command_id, command_kind, command_schema_ver,
                request_fingerprint
           FROM command_record
          WHERE command_kind IN ('McpGrantIssue', 'McpGrantRevoke')",
    )
    .fetch_all(app.storage.reader())
    .await
    .unwrap();
    recorded.sort();
    let pinned = |id: &str, kind: &str, params: serde_json::Value| {
        let (user, local) = ("User".to_string(), "local".to_string());
        (
            user,
            local,
            id.to_string(),
            kind.to_string(),
            1,
            fingerprint(kind, &params),
        )
    };
    let mut expected = vec![
        pinned(
            "pin-issue",
            "McpGrantIssue",
            json!({ "project": app.project }),
        ),
        pinned("pin-revoke", "McpGrantRevoke", json!({ "grant": grant })),
    ];
    expected.sort();
    assert_eq!(recorded, expected);
}

/// Spec §14.9: a project create sent twice answers the first project, and two
/// spellings of one folder are one request, because the directory is resolved
/// before the fingerprint is taken; another body under the same id conflicts.
#[tokio::test]
async fn project_create_replays_after_the_move() {
    let app = test_app().await;
    let dir = tempfile::tempdir().unwrap();
    let plain = dir.path().to_string_lossy().to_string();
    let body = |directory: &str| {
        json!({
            "command_id": "p-replay", "slug": "second", "name": "Second", "directory": directory,
        })
    };
    let (status, first) = post(&app, "/api/projects", body(&plain)).await;
    assert_eq!(status, 200, "{first}");
    let (status, again) = post(&app, "/api/projects", body(&format!("{plain}/"))).await;
    assert_eq!(status, 200, "{again}");
    assert_eq!(again["id"], first["id"]);
    let (_, list) = call(&app, "GET", "/api/projects", None).await;
    assert_eq!(list.as_array().unwrap().len(), 2);

    let mut other = body(&plain);
    other["name"] = json!("Another");
    let (status, conflict) = post(&app, "/api/projects", other).await;
    assert_eq!(status, 409, "{conflict}");
    assert_eq!(conflict["code"], "COMMAND_CONFLICT");
}

/// A replayed project, thread or instructions command matches one an earlier
/// daemon recorded only when its principal, kind, schema version and
/// fingerprint parameters are the ones that daemon used. This reads the
/// command log and compares it with each command's parameters as the code
/// before the move wrote them (§14.9).
#[tokio::test]
async fn project_thread_and_instructions_command_fingerprints_do_not_move() {
    let app = test_app().await;
    let dir = tempfile::tempdir().unwrap();
    let body = json!({
        "command_id": "pin-project", "slug": "pinned", "name": "Pinned",
        "directory": dir.path().to_string_lossy(),
    });
    let (status, project) = post(&app, "/api/projects", body).await;
    assert_eq!(status, 200, "{project}");
    let (project_id, directory) = (project["id"].clone(), project["directory"].clone());

    // Unsorted, with a repeat: the fingerprint is over the set as normalized.
    let modes = json!({ "claude-code": ["auto", "acceptEdits", "auto"] });
    let path = format!("/api/projects/{}", project_id.as_str().unwrap());
    let body = json!({ "command_id": "pin-modes", "allowed_modes": modes });
    let (status, changed) = patch(&app, &path, body).await;
    assert_eq!(status, 200, "{changed}");

    // No harness named: the fingerprint names the default.
    let path = format!("/api/projects/{}/threads", app.project);
    let body = json!({ "command_id": "pin-thread", "title": "Pinned" });
    let (status, thread) = post(&app, &path, body).await;
    assert_eq!(status, 200, "{thread}");
    let thread_id = thread["id"].clone();

    let path = format!("/api/threads/{}", thread_id.as_str().unwrap());
    let body = json!({ "command_id": "pin-harness", "harness": "codex" });
    let (status, changed) = patch(&app, &path, body).await;
    assert_eq!(status, 200, "{changed}");

    start_and_finish(&app, "hello", default_settings()).await;
    let path = format!("/api/threads/{}/entries", app.thread);
    let (_, entries) = call(&app, "GET", &path, None).await;
    let last = entries.as_array().unwrap().last().unwrap()["id"].clone();
    let path = format!("/api/threads/{}/fork", app.thread);
    let body = json!({ "command_id": "pin-fork", "at_entry_id": last });
    let (status, fork) = post(&app, &path, body).await;
    assert_eq!(status, 201, "{fork}");

    let path = format!("/api/projects/{}/planner-instructions", app.project);
    let body = json!({ "command_id": "pin-instructions", "body": "Be brief." });
    let (status, saved) = call(&app, "PUT", &path, Some(body)).await;
    assert_eq!(status, 200, "{saved}");

    let mut recorded: Vec<(String, String, String, String, i64, String)> = sqlx::query_as(
        "SELECT principal_kind, principal_id, command_id, command_kind, command_schema_ver,
                request_fingerprint
           FROM command_record
          WHERE command_kind IN ('project.create', 'project.modes', 'thread.create',
                                 'thread.harness', 'thread.fork', 'PlannerInstructionsSave')
            AND command_id LIKE 'pin-%'",
    )
    .fetch_all(app.storage.reader())
    .await
    .unwrap();
    recorded.sort();
    let pinned = |id: &str, kind: &str, params: serde_json::Value| {
        let (user, local) = ("User".to_string(), "local".to_string());
        (
            user,
            local,
            id.to_string(),
            kind.to_string(),
            1,
            fingerprint(kind, &params),
        )
    };
    let modes = json!({ "claude-code": ["acceptEdits", "auto"] });
    let mut expected = vec![
        pinned(
            "pin-project",
            "project.create",
            json!({ "slug": "pinned", "name": "Pinned", "directory": directory }),
        ),
        pinned(
            "pin-modes",
            "project.modes",
            json!({ "project": project_id, "allowed_modes": modes }),
        ),
        pinned(
            "pin-thread",
            "thread.create",
            json!({ "project": app.project, "title": "Pinned", "harness": "claude-code" }),
        ),
        pinned(
            "pin-harness",
            "thread.harness",
            json!({ "thread_id": thread_id, "harness": "codex" }),
        ),
        pinned(
            "pin-fork",
            "thread.fork",
            json!({ "thread_id": app.thread, "at_entry_id": last }),
        ),
        pinned(
            "pin-instructions",
            "PlannerInstructionsSave",
            json!({ "project": app.project, "body": "Be brief." }),
        ),
    ];
    expected.sort();
    assert_eq!(recorded, expected);
}
