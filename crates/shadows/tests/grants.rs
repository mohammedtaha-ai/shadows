//! MCP grants: a token shown once and stored only as its hash,
//! thread grants for the Planner and project grants for external agents,
//! each issued and revoked, and the draft refs an external agent starts
//! plans with.

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
async fn a_named_connection_changes_scope_without_replacing_its_token() {
    let app = test_app().await;
    let (other, _) = other_project(&app).await;
    let path = "/api/external-connections";
    let (status, issued) = post(
        &app,
        path,
        json!({ "command_id": "named", "name": " Laptop ", "project_ids": [app.project] }),
    )
    .await;
    assert_eq!(status, 200, "{issued}");
    assert_eq!(issued["connection"]["name"], "Laptop");
    assert_eq!(issued["connection"]["project_revision"], 0);
    let token = issued["token"].as_str().unwrap();
    let grants = app.core.grants();
    let connection = grants.authorize_external(token).await.unwrap().unwrap();
    let id = &connection.id;
    let original = grants.external_project(id, &app.project).await.unwrap();

    let path = format!("{path}/{id}");
    let change = json!({ "command_id": "change", "expected_revision": 0, "project_ids": [other] });
    let (status, changed) = call(&app, "PATCH", &path, Some(change.clone())).await;
    assert_eq!(status, 200, "{changed}");
    assert_eq!(changed["project_revision"], 1);
    assert_eq!(changed["project_ids"], json!([other]));
    assert!(changed.get("token").is_none());
    assert_eq!(
        call(&app, "PATCH", &path, Some(change)).await,
        (200, changed)
    );
    let current = grants.authorize_external(token).await.unwrap().unwrap();
    assert_eq!(current.project_ids, vec![other.clone()]);
    assert!(grants.external_project(id, &app.project).await.is_err());
    let kept = grants.external_project(id, &other).await.unwrap();

    let edit = |command, revision| {
        json!({
            "command_id": command, "expected_revision": revision,
            "project_ids": [other, app.project]
        })
    };
    let (status, conflict) = call(&app, "PATCH", &path, Some(edit("stale", 0))).await;
    assert_eq!(status, 409, "{conflict}");
    assert_eq!(conflict["code"], "REVISION_CONFLICT");
    let (status, restored) = call(&app, "PATCH", &path, Some(edit("restore", 1))).await;
    assert_eq!(status, 200, "{restored}");
    assert_eq!(
        grants.external_project(id, &other).await.unwrap().id,
        kept.id
    );
    assert_ne!(
        grants.external_project(id, &app.project).await.unwrap().id,
        original.id
    );

    let clear = json!({ "command_id": "clear", "expected_revision": 2, "project_ids": [] });
    let (status, cleared) = call(&app, "PATCH", &path, Some(clear)).await;
    assert_eq!(status, 200, "{cleared}");
    assert!(
        grants
            .authorize_external(token)
            .await
            .unwrap()
            .unwrap()
            .project_ids
            .is_empty()
    );
    app.core
        .shut_down(std::time::Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
}

#[tokio::test]
async fn a_replayed_issue_does_not_show_the_token_again() {
    let app = test_app().await;
    let path = "/api/external-connections";
    let (status, refused) =
        post(&app, path, json!({ "command_id": "g1", "project_ids": [] })).await;
    assert_eq!(status, 422, "{refused}");
    let (status, first) = post(
        &app,
        path,
        json!({ "command_id": "g1", "project_ids": [app.project, app.project] }),
    )
    .await;
    assert_eq!(status, 200, "{first}");
    let token = first["token"].as_str().unwrap().to_string();
    assert_eq!(first["mcp_url"], acp::MCP_URL);
    assert_eq!(first["connection"]["project_ids"], json!([app.project]));

    let (status, replay) = post(
        &app,
        path,
        json!({ "command_id": "g1", "project_ids": [app.project] }),
    )
    .await;
    assert_eq!(status, 200, "{replay}");
    assert_eq!(replay["connection"], first["connection"]);
    assert_eq!(replay["token"], Value::Null);

    // The list never carries a token; newest first.
    let (_, second) = post(
        &app,
        path,
        json!({ "command_id": "g2", "project_ids": [app.project] }),
    )
    .await;
    let listed: Value = get_json(&app, path).await;
    assert_eq!(listed, json!([second["connection"], first["connection"]]));
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

    app.core
        .shut_down(std::time::Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
}

#[tokio::test]
async fn a_grant_is_revoked_by_delete_with_its_command_id_in_the_query() {
    let app = test_app().await;
    let path = "/api/external-connections";
    let (_, issued) = post(
        &app,
        path,
        json!({ "command_id": "g1", "project_ids": [app.project] }),
    )
    .await;
    let id = issued["connection"]["id"].as_str().unwrap();

    let (status, body) = call(
        &app,
        "DELETE",
        &format!("/api/external-connections/{id}"),
        None,
    )
    .await;
    assert_eq!(status, 400, "no command id: {body}");

    let delete = format!("/api/external-connections/{id}?command_id=r1");
    let (status, revoked) = call(&app, "DELETE", &delete, None).await;
    assert_eq!(status, 200, "{revoked}");
    assert!(revoked["revoked_at"].is_string(), "{revoked}");
    let (status, replay) = call(&app, "DELETE", &delete, None).await;
    assert_eq!((status, &replay), (200, &revoked));

    let token = issued["token"].as_str().unwrap();
    assert_eq!(
        app.core.grants().authorize_external(token).await.unwrap(),
        None
    );

    let unknown = "/api/external-connections/00000000-0000-4000-8000-000000000000?command_id=r2";
    let (status, body) = call(&app, "DELETE", unknown, None).await;
    assert_eq!(status, 404, "{body}");

    app.core
        .shut_down(std::time::Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
}

#[tokio::test]
async fn grant_events_never_carry_the_token() {
    let app = test_app().await;
    let (_, issued) = post(
        &app,
        "/api/external-connections",
        json!({ "command_id": "g1", "project_ids": [app.project] }),
    )
    .await;
    let project_token = issued["token"].as_str().unwrap().to_string();
    let grant_id = issued["connection"]["id"].as_str().unwrap().to_string();
    let (_, revoked) = call(
        &app,
        "DELETE",
        &format!("/api/external-connections/{grant_id}?command_id=r1"),
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
    assert_eq!(payloads.len(), 4);
    assert_eq!(payloads[0].0, "McpGrantIssued");
    assert_eq!(payloads[0].1["kind"], "project");
    assert_eq!(
        payloads[1],
        (
            "McpGrantRevoked".into(),
            json!({"grant_id": payloads[0].1["grant_id"]})
        )
    );
    assert_eq!(
        payloads[2],
        (
            "McpGrantIssued".into(),
            json!({"grant_id": thread_grant.id, "kind": "thread"})
        )
    );
    assert_eq!(
        payloads[3],
        (
            "McpGrantRevoked".into(),
            json!({"grant_id": thread_grant.id})
        )
    );

    let every: Vec<String> = sqlx::query_scalar("SELECT payload_json FROM durable_event")
        .fetch_all(app.storage.reader())
        .await
        .unwrap();
    for token in [project_token.as_str(), thread_token.as_str()] {
        assert!(every.iter().all(|p| !p.contains(token)), "{token}");
        assert!(every.iter().all(|p| !p.contains(&Token::hash_token(token))));
    }

    app.core
        .shut_down(std::time::Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
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

    app.core
        .shut_down(std::time::Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
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

    app.core
        .shut_down(std::time::Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
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

    app.core
        .shut_down(std::time::Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
}

/// The grant a session's opening issues is a durable event of the
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

    app.core
        .shut_down(std::time::Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
}

#[tokio::test]
async fn an_approval_needs_a_message_from_the_person_in_its_turn() {
    use shadows_core::{Actor, Approver, Grants, GrantsError, ThreadEntryKind, Threads};
    let app = test_app().await;
    let (grant, _) = Grants::issue_thread_grant_for_test(&app.storage, &app.thread)
        .await
        .unwrap();
    let writer = Writer::from_grant(&grant, None).unwrap();
    let (thread, project) = (app.thread.clone(), app.project.clone());
    let executor = app
        .storage
        .write_txn(move |conn| {
            Box::pin(async move {
                Threads::create_executor_in(conn, &project, "T1".into(), "claude-code").await
            })
        })
        .await
        .unwrap();
    let op = shadows_core::Turns::create_pending_operation_for_test(
        &app.storage,
        &app.thread,
        app.runtime.instance_id(),
    )
    .await
    .unwrap()
    .as_str()
    .to_string();
    // Writes one UserMessage of the turn by `author`, then asks for the approval.
    let step = |author: Actor| {
        let (thread, project, writer) = (thread.clone(), app.project.clone(), writer.clone());
        let op = op.clone();
        let storage = app.storage.clone();
        async move {
            storage
                .write_txn(move |conn| {
                    Box::pin(async move {
                        let entry = Threads::append_entry_in(
                            conn,
                            &thread,
                            shadows_core::testing::NewThreadEntry {
                                kind: ThreadEntryKind::UserMessage,
                                author,
                                body: "words",
                                refs: &[],
                                card: None,
                                operation_id: Some(&op),
                            },
                            "2026-11-12T00:00:00Z",
                        )
                        .await?;
                        let harness = Threads::harness_in(conn, &thread, &project).await?;
                        let approver = Approver::Planner(shadows_core::PlannerApprover {
                            writer,
                            turn: op.clone(),
                            harness,
                        });
                        Ok((
                            entry.id,
                            Grants::authorise_in(conn, &approver, &project).await,
                        ))
                    })
                })
                .await
                .unwrap()
        }
    };
    let by_executor = Actor {
        kind: "Executor".into(),
        id: executor.as_str().to_string(),
    };
    let (_, refused) = step(by_executor.clone()).await;
    assert!(
        matches!(&refused, Err(GrantsError::GrantScopeMessage(m))
            if m.starts_with("Only the person approves")),
        "{refused:?}"
    );
    let (person, allowed) = step(Actor::user("local")).await;
    assert_eq!(allowed.unwrap(), Some(person.clone()));
    let (_, still) = step(by_executor).await;
    assert_eq!(
        still.unwrap(),
        Some(person),
        "the person's latest message is the request"
    );
    app.core
        .shut_down(std::time::Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
}

/// A connection on `app.project`: its token and its project grant.
async fn connection(app: &App, command: &str) -> (String, shadows_core::Grant) {
    let (status, issued) = post(
        app,
        "/api/external-connections",
        json!({ "command_id": command, "project_ids": [app.project] }),
    )
    .await;
    assert_eq!(status, 200, "{issued}");
    let token = issued["token"].as_str().unwrap().to_string();
    let id = (app
        .core
        .grants()
        .authorize_external(&token)
        .await
        .unwrap()
        .unwrap())
    .id;
    let grant = app
        .core
        .grants()
        .external_project(&id, &app.project)
        .await
        .unwrap();
    (token, grant)
}

/// An ended attempt's report kept for `grant`, claimed until `claimed_until` when given.
async fn keep_report(app: &App, grant: &shadows_core::Grant, claimed_until: Option<&str>) {
    sqlx::query(
        "INSERT INTO run_attempts
           (id, project_id, workflow_id, version, task, number, thread_id, model, state,
            writes_json, started_at, ended_at, command_id, fingerprint)
         VALUES ('attempt-1', ?, 'wf-1', 1, 1, 1, ?, 'fake', 'committed', '[]',
                 '2026-10-05T00:00:00Z', '2026-10-05T00:01:00Z', 'attempt-1', 'f')",
    )
    .bind(app.project.as_str())
    .bind(app.thread.as_str())
    .execute(app.storage.reader())
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO run_author_report
           (report_id, kind, source_id, attempt_id, project_id, grant_id, workflow_id, task, body,
            created_at, claimed_until)
         VALUES ('report-1', 'ended', 'attempt-1', 'attempt-1', ?, ?, 'wf-1', 1, 'Built.',
                 '2026-10-05T00:01:00Z', ?)",
    )
    .bind(app.project.as_str())
    .bind(grant.id.as_str())
    .bind(claimed_until)
    .execute(app.storage.reader())
    .await
    .unwrap();
}

async fn http_wait(app: &App, token: &str, body: Value) -> (u16, Value) {
    use axum::body::Body;
    use axum::http::Request;
    use tower::ServiceExt;
    let request = Request::builder()
        .method("POST")
        .uri("/api/external/task-wait")
        .header("content-type", "application/json")
        .header("authorization", format!("Bearer {token}"))
        .body(Body::from(body.to_string()))
        .unwrap();
    let response = app.router.clone().oneshot(request).await.unwrap();
    let status = response.status().as_u16();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let value = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap()
    };
    (status, value)
}

async fn report_open(app: &App) -> bool {
    sqlx::query_scalar(
        "SELECT acked_at IS NULL FROM run_author_report WHERE report_id = 'report-1'",
    )
    .fetch_one(app.storage.reader())
    .await
    .unwrap()
}

#[tokio::test]
async fn the_http_wait_answers_with_the_connections_bearer() {
    let app = test_app().await;
    let (token, grant) = connection(&app, "wait-answers").await;
    keep_report(&app, &grant, None).await;
    let body = json!({ "project_id": app.project, "seconds": 1 });
    let (status, wait) = http_wait(&app, &token, body).await;
    assert_eq!(status, 200, "{wait}");
    assert_eq!(wait["outcome"], "ended");
    assert_eq!(wait["reports"][0]["report_id"], "report-1");
    assert_eq!(wait["reports"][0]["report"], "Built.");
    app::shut_down_app(app).await;
}

#[tokio::test]
async fn the_http_wait_acknowledges_what_it_names() {
    let app = test_app().await;
    let (token, grant) = connection(&app, "wait-acks").await;
    keep_report(&app, &grant, None).await;
    let first = json!({ "project_id": app.project, "seconds": 1 });
    let (_, wait) = http_wait(&app, &token, first).await;
    assert_eq!(wait["reports"][0]["report_id"], "report-1");
    let body = json!({ "project_id": app.project, "seconds": 1, "ack": ["report-1"] });
    let (status, wait) = http_wait(&app, &token, body).await;
    assert_eq!(status, 200, "{wait}");
    assert_eq!(wait["reports"], json!([]), "{wait}");
    assert!(!report_open(&app).await);
    app::shut_down_app(app).await;
}

#[tokio::test]
async fn the_http_wait_refuses_a_revoked_bearer() {
    let app = test_app().await;
    let (token, grant) = connection(&app, "wait-revoked").await;
    keep_report(&app, &grant, None).await;
    let connection = app
        .core
        .grants()
        .authorize_external(&token)
        .await
        .unwrap()
        .unwrap();
    app.core
        .grants()
        .revoke_external("revoke".into(), &connection.id)
        .await
        .unwrap();
    let body = json!({ "project_id": app.project, "seconds": 1 });
    assert_eq!(http_wait(&app, &token, body.clone()).await.0, 401);
    assert_eq!(http_wait(&app, "no-such-token", body).await.0, 401);
    assert!(report_open(&app).await);
    app::shut_down_app(app).await;
}

#[tokio::test]
async fn the_http_wait_refuses_a_connection_revoked_while_it_waits() {
    let app = test_app().await;
    let (token, grant) = connection(&app, "wait-revoked-during").await;
    // A claim held by an earlier answer: the wait sleeps until it lapses, then claims again.
    let lapse: String = sqlx::query_scalar(
        "SELECT strftime('%Y-%m-%dT%H:%M:%f', 'now', '+1 seconds') || '000000Z'",
    )
    .fetch_one(app.storage.reader())
    .await
    .unwrap();
    keep_report(&app, &grant, Some(&lapse)).await;
    let connection = app
        .core
        .grants()
        .authorize_external(&token)
        .await
        .unwrap()
        .unwrap();
    let body = json!({ "project_id": app.project, "seconds": 5 });
    let waiting = http_wait(&app, &token, body);
    let revoke = async {
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        app.core
            .grants()
            .revoke_external("revoke-during".into(), &connection.id)
            .await
            .unwrap();
    };
    let ((status, _), ()) = tokio::join!(waiting, revoke);
    assert_eq!(status, 401);
    assert!(report_open(&app).await);
    app::shut_down_app(app).await;
}

#[tokio::test]
async fn the_http_wait_refuses_an_unselected_project() {
    let app = test_app().await;
    let (other, _) = other_project(&app).await;
    let (token, _) = connection(&app, "wait-unselected").await;
    let body = json!({ "project_id": other, "seconds": 1 });
    let (status, refusal) = http_wait(&app, &token, body).await;
    assert_eq!(status, 404, "{refusal}");
    app::shut_down_app(app).await;
}

#[tokio::test]
async fn a_connection_names_its_wait_url() {
    let app = test_app().await;
    let (status, issued) = post(
        &app,
        "/api/external-connections",
        json!({ "command_id": "wait-url", "project_ids": [app.project] }),
    )
    .await;
    assert_eq!(status, 200, "{issued}");
    let base = shadows_core::testing::acp::MCP_URL
        .strip_suffix("/mcp")
        .unwrap();
    let url = format!("{base}/api/external/task-wait");
    assert_eq!(issued["connection"]["wait_url"], url);
    let listed: Value = get_json(&app, "/api/external-connections").await;
    assert_eq!(listed[0]["wait_url"], url);
    app::shut_down_app(app).await;
}
