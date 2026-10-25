//! MCP grants (spec §13.7): a token shown once and stored only as its hash,
//! thread grants for the Planner and project grants for external agents,
//! each issued and revoked, and the draft refs an external agent starts
//! plans with (§13.5).

use serde_json::{Value, json};
use shadows_core::testing::Token;
use shadows_core::testing::Writer;
use shadows_core::{DraftStarted, GrantId};

use shadows_core::testing::acp;
#[path = "fixtures/app.rs"]
mod app;
#[path = "fixtures/plan.rs"]
mod plan;

use app::{App, call, ctx, get_json, other_project, post, test_app};
use plan::{an_hour_ago, draft_ref_binding, insert_draft_ref, writer_ctx};

fn issue_ctx(id: &str) -> shadows_core::testing::CommandContext {
    ctx(id, "McpGrantIssue")
}

#[tokio::test]
async fn a_replayed_issue_does_not_show_the_token_again() {
    let app = test_app().await;
    let path = format!("/api/projects/{}/mcp-grants", app.project);
    let (status, first) = post(&app, &path, json!({ "command_id": "g1" })).await;
    assert_eq!(status, 200, "{first}");
    let token = first["token"].as_str().unwrap().to_string();
    assert_eq!(
        first["command"],
        format!(
            "claude mcp add --transport http shadows {} --header \"Authorization: Bearer {token}\"",
            acp::MCP_URL
        )
    );
    assert_eq!(first["grant"]["kind"], "project");

    let (status, replay) = post(&app, &path, json!({ "command_id": "g1" })).await;
    assert_eq!(status, 200, "{replay}");
    assert_eq!(replay["grant"], first["grant"]);
    assert_eq!(replay["token"], Value::Null);
    assert_eq!(replay["command"], Value::Null);

    // The list never carries a token; newest first.
    let (_, second) = post(&app, &path, json!({ "command_id": "g2" })).await;
    let listed: Value = get_json(&app, &path).await;
    assert_eq!(listed, json!([second["grant"], first["grant"]]));
    assert!(!listed.to_string().contains(&token));

    // Storage answers a replay the same way: the grant, no token.
    let g3 = issue_ctx("g3");
    let issue =
        || shadows_core::Grants::issue_project_grant_for_test(&app.storage, &g3, &app.project);
    let issued = issue().await.unwrap();
    assert!(issued.token.is_some());
    let replayed = issue().await.unwrap();
    assert!(replayed.token.is_none());
    assert_eq!(replayed.grant, issued.grant);
}

#[tokio::test]
async fn a_grant_is_revoked_by_delete_with_its_command_id_in_the_query() {
    let app = test_app().await;
    let path = format!("/api/projects/{}/mcp-grants", app.project);
    let (_, issued) = post(&app, &path, json!({ "command_id": "g1" })).await;
    let id = issued["grant"]["id"].as_str().unwrap();

    let (status, body) = call(&app, "DELETE", &format!("/api/mcp-grants/{id}"), None).await;
    assert_eq!(status, 400, "no command id: {body}");

    let delete = format!("/api/mcp-grants/{id}?command_id=r1");
    let (status, revoked) = call(&app, "DELETE", &delete, None).await;
    assert_eq!(status, 200, "{revoked}");
    assert!(revoked["revoked_at"].is_string(), "{revoked}");
    let (status, replay) = call(&app, "DELETE", &delete, None).await;
    assert_eq!((status, &replay), (200, &revoked));

    let token = issued["token"].as_str().unwrap();
    assert_eq!(
        shadows_core::Grants::grant_for_token_for_test(&app.storage, token)
            .await
            .unwrap(),
        None
    );

    let unknown = "/api/mcp-grants/00000000-0000-4000-8000-000000000000?command_id=r2";
    let (status, body) = call(&app, "DELETE", unknown, None).await;
    assert_eq!(status, 404, "{body}");
}

#[tokio::test]
async fn grant_events_never_carry_the_token() {
    let app = test_app().await;
    let (_, issued) = post(
        &app,
        &format!("/api/projects/{}/mcp-grants", app.project),
        json!({ "command_id": "g1" }),
    )
    .await;
    let project_token = issued["token"].as_str().unwrap().to_string();
    let grant_id = issued["grant"]["id"].as_str().unwrap().to_string();
    let (_, revoked) = call(
        &app,
        "DELETE",
        &format!("/api/mcp-grants/{grant_id}?command_id=r1"),
        None,
    )
    .await;
    assert!(revoked["revoked_at"].is_string());
    let (thread_grant, thread_token) =
        shadows_core::Grants::issue_thread_grant_for_test(&app.storage, &app.thread)
            .await
            .unwrap();
    shadows_core::Grants::revoke_thread_grant_for_test(&app.storage, &thread_grant.id)
        .await
        .unwrap();

    let events: Vec<(String, String)> = sqlx::query_as(
        "SELECT kind, payload_json FROM durable_event
          WHERE kind LIKE 'McpGrant%' ORDER BY seq",
    )
    .fetch_all(app.storage.reader())
    .await
    .unwrap();
    let payloads: Vec<(String, Value)> = events
        .into_iter()
        .map(|(k, p)| (k, serde_json::from_str(&p).unwrap()))
        .collect();
    assert_eq!(
        payloads,
        [
            (
                "McpGrantIssued".to_string(),
                json!({ "grant_id": grant_id, "kind": "project" })
            ),
            (
                "McpGrantRevoked".to_string(),
                json!({ "grant_id": grant_id })
            ),
            (
                "McpGrantIssued".to_string(),
                json!({ "grant_id": thread_grant.id, "kind": "thread" })
            ),
            (
                "McpGrantRevoked".to_string(),
                json!({ "grant_id": thread_grant.id })
            ),
        ]
    );

    let every: Vec<String> = sqlx::query_scalar("SELECT payload_json FROM durable_event")
        .fetch_all(app.storage.reader())
        .await
        .unwrap();
    for token in [project_token.as_str(), thread_token.as_str()] {
        assert!(every.iter().all(|p| !p.contains(token)), "{token}");
        assert!(every.iter().all(|p| !p.contains(&Token::hash_token(token))));
    }
}

#[tokio::test]
async fn startup_revokes_every_thread_grant_and_keeps_project_grants() {
    let app = test_app().await;
    let (_, other_thread) = other_project(&app).await;
    let second_thread = app::create_thread(&app, json!({ "command_id": "t2", "title": "B" })).await;
    let second_thread =
        serde_json::from_value::<shadows_core::ThreadId>(second_thread["id"].clone()).unwrap();
    let mut thread_tokens = Vec::new();
    for thread in [&app.thread, &second_thread, &other_thread] {
        thread_tokens.push(
            shadows_core::Grants::issue_thread_grant_for_test(&app.storage, thread)
                .await
                .unwrap()
                .1,
        );
    }
    let project = shadows_core::Grants::issue_project_grant_for_test(
        &app.storage,
        &issue_ctx("g1"),
        &app.project,
    )
    .await
    .unwrap();
    let project_token = project.token.unwrap();

    assert_eq!(
        shadows_core::Grants::revoke_all_thread_grants_for_test(&app.storage)
            .await
            .unwrap(),
        3
    );
    for token in &thread_tokens {
        assert_eq!(
            shadows_core::Grants::grant_for_token_for_test(&app.storage, token.as_str())
                .await
                .unwrap(),
            None
        );
    }
    assert_eq!(
        shadows_core::Grants::grant_for_token_for_test(&app.storage, project_token.as_str())
            .await
            .unwrap(),
        Some(project.grant.clone())
    );
    assert_eq!(
        shadows_core::Grants::list_project_grants_for_test(&app.storage, &app.project)
            .await
            .unwrap(),
        vec![project.grant]
    );
    // Nothing left to revoke: a second start revokes none.
    assert_eq!(
        shadows_core::Grants::revoke_all_thread_grants_for_test(&app.storage)
            .await
            .unwrap(),
        0
    );
}

/// An external start from scratch under `grant` with `draft_ref`. Each new
/// `command` misses the command log, so the ref is judged by `bind_draft_ref`.
async fn start(
    app: &App,
    grant: &GrantId,
    command: &str,
    draft_ref: &str,
) -> Result<DraftStarted, shadows_core::CoreError> {
    let writer = Writer::External {
        grant: grant.clone(),
    };
    let ctx = writer_ctx(&writer, command, "DraftStart", json!({ "r": draft_ref }));
    let (title, goal) = ("Search", "find things");
    shadows_core::AppCore::plans_for_tests(app.storage.clone())
        .start_draft_for_test(
            &ctx,
            &writer,
            &app.project,
            None,
            None,
            Some((title, goal)),
            None,
            None,
            Some(draft_ref),
            &[],
        )
        .await
        .map_err(shadows_core::CoreError::from)
}

#[tokio::test]
async fn a_draft_ref_expires_and_belongs_to_its_grant() {
    let app = test_app().await;
    let mine = shadows_core::Grants::issue_project_grant_for_test(
        &app.storage,
        &issue_ctx("g1"),
        &app.project,
    )
    .await
    .unwrap()
    .grant
    .id;
    let theirs = shadows_core::Grants::issue_project_grant_for_test(
        &app.storage,
        &issue_ctx("g2"),
        &app.project,
    )
    .await
    .unwrap()
    .grant
    .id;
    // Issued unused, for an hour, to the grant that asked.
    let fresh = shadows_core::Grants::prepare_draft_for_test(&app.storage, &mine)
        .await
        .unwrap();
    assert_eq!(draft_ref_binding(&app, &fresh).await, None);
    let (created, expires): (String, String) =
        sqlx::query_as("SELECT created_at, expires_at FROM draft_intent WHERE draft_ref = ?")
            .bind(&fresh)
            .fetch_one(app.storage.reader())
            .await
            .unwrap();
    let parse = |t: &str| {
        time::OffsetDateTime::parse(t, &time::format_description::well_known::Rfc3339).unwrap()
    };
    assert_eq!(parse(&expires) - parse(&created), time::Duration::HOUR);
    // Not deduplicated: a second call is a second ref.
    assert_ne!(
        shadows_core::Grants::prepare_draft_for_test(&app.storage, &mine)
            .await
            .unwrap(),
        fresh
    );

    // Another grant's ref, an unknown one, and an expired one start nothing.
    let expired = insert_draft_ref(&app, &mine, &an_hour_ago()).await;
    for (grant, command, draft_ref) in [
        (&theirs, "c-theirs", fresh.as_str()),
        (&mine, "c-unknown", "dr-unknown"),
        (&mine, "c-expired", expired.as_str()),
    ] {
        let refused = start(&app, grant, command, draft_ref).await;
        assert!(
            matches!(
                refused,
                Err(shadows_core::CoreError::Refused {
                    code: shadows_core::ErrorCode::GrantScope,
                    ..
                })
            ),
            "{command}: {refused:?}"
        );
    }
    assert_eq!(draft_ref_binding(&app, &fresh).await, None);
    assert_eq!(draft_ref_binding(&app, &expired).await, None);
    let threads =
        shadows_core::Threads::list_threads_for_project_for_test(&app.storage, &app.project).await;
    assert_eq!(threads.unwrap().len(), 1, "a refused start made no thread");

    // Its own grant starts a plan with it, and the ref is bound to that plan.
    let started = start(&app, &mine, "c-first", &fresh).await.unwrap();
    assert_eq!(
        draft_ref_binding(&app, &fresh).await.as_deref(),
        Some(started.workflow_id.as_str())
    );

    // Past its expiry, the start that used it still answers its plan; but a
    // bound ref starts no second plan.
    sqlx::query("UPDATE draft_intent SET expires_at = ? WHERE draft_ref = ?")
        .bind(an_hour_ago())
        .bind(&fresh)
        .execute(app.storage.reader())
        .await
        .unwrap();
    assert_eq!(
        start(&app, &mine, "c-first", &fresh).await.unwrap(),
        started
    );
    let second = start(&app, &mine, "c-second", &fresh).await;
    assert!(
        matches!(
            second,
            Err(shadows_core::CoreError::Refused {
                code: shadows_core::ErrorCode::GrantScope,
                ..
            })
        ),
        "{second:?}"
    );
    assert_eq!(
        draft_ref_binding(&app, &fresh).await.as_deref(),
        Some(started.workflow_id.as_str())
    );

    // Only a live project grant prepares a draft.
    let (thread_grant, _) =
        shadows_core::Grants::issue_thread_grant_for_test(&app.storage, &app.thread)
            .await
            .unwrap();
    let planner =
        shadows_core::Grants::prepare_draft_for_test(&app.storage, &thread_grant.id).await;
    assert!(
        matches!(planner, Err(shadows_core::GrantsError::GrantScope)),
        "{planner:?}"
    );
    shadows_core::Grants::revoke_grant_for_test(
        &app.storage,
        &ctx("r1", "McpGrantRevoke"),
        &theirs,
    )
    .await
    .unwrap();
    let revoked = shadows_core::Grants::prepare_draft_for_test(&app.storage, &theirs).await;
    assert!(
        matches!(revoked, Err(shadows_core::GrantsError::GrantInvalid)),
        "{revoked:?}"
    );
}

#[tokio::test]
async fn a_thread_grant_cannot_name_a_thread_of_another_project() {
    let app = test_app().await;
    let (_, foreign_thread) = other_project(&app).await;
    let refused = sqlx::query(
        "INSERT INTO mcp_grant (id, kind, thread_id, project_id, token_hash, created_at)
         VALUES ('g-mixed', 'thread', ?, ?, 'h-mixed', '2026-09-25T00:00:00Z')",
    )
    .bind(foreign_thread.as_str())
    .bind(app.project.as_str())
    .execute(app.storage.reader())
    .await;
    let error = refused.expect_err("a thread of another project is refused");
    assert!(
        error.to_string().contains("FOREIGN KEY"),
        "refused for another reason: {error}"
    );
    // Issued through the API, a thread grant takes its thread's own project.
    let (grant, _) =
        shadows_core::Grants::issue_thread_grant_for_test(&app.storage, &foreign_thread)
            .await
            .unwrap();
    assert_ne!(grant.project_id, app.project);
}

/// §13.7: the grant a session's opening issues is a durable event of the
/// thread, and a subscriber reads it at once, not at some later commit.
#[tokio::test]
async fn a_sessions_grant_reaches_a_subscriber_live() {
    let tmp = tempfile::tempdir().unwrap();
    let config = shadows_core::testing::SessionsConfig {
        mcp_url: Some(acp::MCP_URL.into()),
        ..acp::test_config()
    };
    let app = app::test_app_with(tmp.path(), config, acp::MCP_URL)
        .await
        .owning(tmp);
    let mut sub = app::subscribe(&app, &app.thread).await;
    let path = format!("/api/threads/{}/session", app.thread);
    let (s, _) = post(&app, &path, json!({})).await;
    assert_eq!(s, 200);
    let frame = app::next_frame_named(&mut sub, "durable").await;
    assert_eq!(
        (frame["kind"].as_str(), frame["thread_id"].as_str()),
        (Some("McpGrantIssued"), Some(app.thread.as_str()))
    );
}
