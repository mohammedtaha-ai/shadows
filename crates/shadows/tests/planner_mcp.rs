//! The Planner's session opens with Shadows (spec §13.7–§13.8): Shadows' MCP
//! server behind a grant per adapter, Shadows' instructions with the
//! project's appended to Claude Code's prompt, and Shadows' tools
//! pre-approved. What changes after a session was created reaches it once,
//! as a context block after the person's text. `fake_acp` reports what its
//! session opened with and calls `/mcp` as Claude would.

use std::time::Duration;

use serde_json::{Value, json};
use shadows_core::WorkflowId;
use shadows_core::testing::Storage;
use shadows_core::testing::{SessionsConfig, prompt_version};

use shadows_core::testing::acp;
#[path = "fixtures/app.rs"]
mod app;
#[path = "fixtures/listening.rs"]
mod listening;
#[path = "fixtures/plan.rs"]
mod plan;
#[path = "fixtures/serve.rs"]
mod serve;

use app::{
    App, call, default_settings, last_agent_entry, last_agent_entry_on, post, shut_down_app,
    start_and_finish, start_and_finish_on, start_settled, wait_terminal,
};
use listening::{listening_app, listening_at, listening_with};
use plan::{add, events, issue_grant};
use serve::serve;

const PROMPT: &str = shadows_core::testing::PROMPT;
const CHANGED: &str = "[Shadows] The project's Planner instructions changed.";
const OURS: &str = "[Shadows] Shadows' instructions for you:";

/// The text of the fake's reply to `prompt`, a turn that completed.
async fn reply(app: &App, prompt: &str) -> String {
    let done = start_and_finish(app, prompt, default_settings()).await;
    assert_eq!(done.status_kind, "Completed", "{done:?}");
    last_agent_entry(app).await.body
}

/// The fake's `report`: what its session opened with, and this prompt's blocks.
async fn report(app: &App) -> Value {
    serde_json::from_str(&reply(app, "report").await).unwrap()
}

fn blocks(report: &Value) -> Vec<&str> {
    let list = report["blocks"].as_array().expect("blocks");
    list.iter().map(|b| b.as_str().unwrap()).collect()
}

/// `(token_hash, revoked_at)` of every thread grant, oldest first.
async fn thread_grants(storage: &Storage) -> Vec<(String, Option<String>)> {
    sqlx::query_as(
        "SELECT token_hash, revoked_at FROM mcp_grant WHERE kind = 'thread'
          ORDER BY created_at, id",
    )
    .fetch_all(storage.reader())
    .await
    .unwrap()
}

/// Whether the grant holding `hash` is still live.
async fn live(storage: &Storage, hash: &Value) -> bool {
    let hash = hash.as_str().expect("a bearer hash");
    let revoked: Option<String> =
        sqlx::query_scalar("SELECT revoked_at FROM mcp_grant WHERE token_hash = ?")
            .bind(hash)
            .fetch_one(storage.reader())
            .await
            .unwrap();
    revoked.is_none()
}

/// Saves the app's project's instructions through the route, as the settings
/// page does.
async fn save_instructions(app: &App, command: &str, body: &str) {
    let path = format!("/api/projects/{}/planner-instructions", app.project);
    let body = json!({ "command_id": command, "body": body });
    let (status, saved) = call(app, "PUT", &path, Some(body)).await;
    assert_eq!(status, 200, "{saved}");
}

async fn project_dir(app: &App) -> std::path::PathBuf {
    let context = app.storage.turn_context(&app.thread).await.unwrap();
    context.project_directory.expect("a project directory")
}

#[tokio::test]
async fn a_session_opens_with_shadows_mcp_instructions_and_the_pre_approval() {
    let l = listening_app().await;
    let r = report(&l.app).await;

    assert_eq!(
        r["mcp"],
        json!({ "name": "shadows", "url": format!("{}/mcp", l.base) })
    );
    let grants = thread_grants(&l.app.storage).await;
    assert_eq!(grants.len(), 1, "one grant for the one adapter: {grants:?}");
    assert_eq!(
        r["bearer_hash"], grants[0].0,
        "the bearer is the live grant's"
    );
    assert_eq!(grants[0].1, None);
    let first_line = PROMPT.lines().next().unwrap();
    assert!(r["append"].as_str().unwrap().contains(first_line), "{r}");
    assert_eq!(r["allowed"], json!(["mcp__shadows__*"]));
}

#[tokio::test]
async fn a_turn_edits_the_plan_through_mcp() {
    let l = listening_app().await;
    let started = reply(&l.app, r#"mcp draft_start {"title":"Login","goal":"g"}"#).await;
    let started: Value = serde_json::from_str(&started).expect(&started);
    let workflow = WorkflowId::from_literal(started["workflow_id"].as_str().unwrap());

    let edit = json!({ "expected_revision": 0, "ops": [add(1)] });
    let edited = reply(&l.app, &format!("mcp plan_edit {edit}")).await;
    let edited: Value = serde_json::from_str(&edited).expect(&edited);
    assert_eq!(edited["revision"], 1);

    let plan = l.app.storage.get_plan(&workflow).await.unwrap();
    assert_eq!(plan.title, "Login");
    let numbers: Vec<u32> = plan.tasks.iter().map(|t| t.content.number).collect();
    assert_eq!(numbers, [1]);
    let edits: Vec<_> = events(&l.app, &l.app.thread)
        .await
        .into_iter()
        .filter(|e| e.0 == "WorkflowEdited")
        .collect();
    assert_eq!(edits.len(), 1);
    assert_eq!(
        (edits[0].1.as_str(), edits[0].2.as_str()),
        ("Thread", l.app.thread.as_str())
    );
}

#[tokio::test]
async fn reopening_after_idle_issues_a_new_grant_and_revokes_the_old() {
    let l = listening_with(SessionsConfig {
        idle_after: Duration::from_millis(300),
        ..acp::test_config()
    })
    .await;
    let first = report(&l.app).await;
    let started = reply(&l.app, r#"mcp draft_start {"title":"Login","goal":"g"}"#).await;
    assert!(started.contains("workflow_id"), "{started}");

    for _ in 0..200 {
        if l.app.sessions.live_count().await == 0 {
            break;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    assert_eq!(l.app.sessions.live_count().await, 0, "the idle close ran");
    assert!(
        !live(&l.app.storage, &first["bearer_hash"]).await,
        "the closed adapter's grant is revoked"
    );

    let second = report(&l.app).await;
    assert_eq!(second["how"], "resume");
    assert_ne!(second["bearer_hash"], first["bearer_hash"]);
    assert!(live(&l.app.storage, &second["bearer_hash"]).await);
    let read = reply(&l.app, "mcp workflow_get {}").await;
    let read: Value = serde_json::from_str(&read).expect(&read);
    assert_eq!(read["title"], "Login");
}

#[tokio::test]
async fn restart_revokes_every_internal_grant() {
    let dir = tempfile::tempdir().unwrap();
    let l = listening_at(dir.path(), acp::test_config()).await;
    let storage = l.app.storage.clone();
    let held = report(&l.app).await["bearer_hash"].clone();
    let (project_grant, _) = issue_grant(&l.app, "project", &l.app.project, None).await;
    // A grant whose adapter died with its daemon: nothing closes it.
    issue_grant(&l.app, "thread", &l.app.project, Some(&l.app.thread)).await;

    shut_down_app(l.app).await;
    assert!(
        !live(&storage, &held).await,
        "stopping closes every adapter"
    );
    let left: Vec<_> = thread_grants(&storage).await;
    assert_eq!(left.iter().filter(|g| g.1.is_none()).count(), 1, "{left:?}");

    drop(serve(
        &dir.path().join("s.sqlite3"),
        shadows_core::testing::tree_probe_path().to_str().unwrap(),
    ));
    let thread: Vec<_> = thread_grants(&storage).await;
    assert_eq!(thread.len(), 2);
    assert!(thread.iter().all(|g| g.1.is_some()), "{thread:?}");
    let revoked: Option<String> =
        sqlx::query_scalar("SELECT revoked_at FROM mcp_grant WHERE id = ?")
            .bind(project_grant.as_str())
            .fetch_one(storage.reader())
            .await
            .unwrap();
    assert_eq!(revoked, None, "a project grant is the person's");
}

/// `serve` (not the in-process app) wires the address it bound into every
/// session it opens: a turn on the real daemon calls its own `/mcp`.
#[tokio::test]
async fn the_daemon_opens_sessions_with_its_own_mcp_address() {
    let tmp = tempfile::tempdir().unwrap();
    let project_dir = tmp.path().join("project");
    std::fs::create_dir(&project_dir).unwrap();
    let daemon = serve(
        &tmp.path().join("s.sqlite3"),
        shadows_core::testing::fake_acp_path().to_str().unwrap(),
    );
    let http = reqwest::Client::new();
    let post = async |path: &str, body: Value| -> Value {
        let url = format!("{}{path}", daemon.base);
        let response = http.post(url).json(&body).send().await.unwrap();
        assert!(
            response.status().is_success(),
            "{path}: {}",
            response.status()
        );
        response.json().await.unwrap()
    };
    let dir = project_dir.to_string_lossy();
    let project = post(
        "/api/projects",
        json!({ "command_id": "p1", "slug": "demo", "name": "Demo", "directory": dir }),
    )
    .await;
    let path = format!("/api/projects/{}/threads", project["id"].as_str().unwrap());
    let thread = post(&path, json!({ "command_id": "t1", "title": "T" })).await;
    let turns = format!("/api/threads/{}/turns", thread["id"].as_str().unwrap());
    let entries = format!(
        "{}/api/threads/{}/entries",
        daemon.base,
        thread["id"].as_str().unwrap()
    );
    let mut replies = Vec::new();
    for (n, prompt) in ["report", r#"mcp draft_start {"title":"Login","goal":"g"}"#]
        .iter()
        .enumerate()
    {
        let mut body = default_settings();
        body["command_id"] = json!(format!("c{n}"));
        body["prompt"] = json!(prompt);
        post(&turns, body).await;
        let reply = tokio::time::timeout(Duration::from_secs(10), async {
            loop {
                let list: Vec<Value> = http
                    .get(&entries)
                    .send()
                    .await
                    .unwrap()
                    .json()
                    .await
                    .unwrap();
                let agent: Vec<&Value> = list
                    .iter()
                    .filter(|e| e["kind"] == "AgentMessage")
                    .collect();
                if agent.len() > n {
                    break agent[n]["body"].as_str().unwrap().to_string();
                }
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
        })
        .await
        .expect("daemon turn did not produce an AgentMessage");
        replies.push(reply);
    }
    let r: Value = serde_json::from_str(&replies[0]).unwrap();
    assert_eq!(r["mcp"]["url"], format!("{}/mcp", daemon.base));
    assert!(replies[1].contains("workflow_id"), "{}", replies[1]);
}

#[tokio::test]
async fn changed_instructions_reach_the_next_turn_once_as_a_context_block() {
    let l = listening_app().await;
    let running = start_settled(&l.app, "wait-for-release").await;
    for _ in 0..200 {
        let op = l.app.storage.get_operation(&running).await.unwrap();
        if op.status_kind == "Running" {
            break;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    let body = "Name every task in Arabic.";
    save_instructions(&l.app, "i1", body).await;
    std::fs::write(project_dir(&l.app).await.join("release"), "").unwrap();
    assert_eq!(
        wait_terminal(&l.app, &running).await.status_kind,
        "Completed"
    );

    let next = report(&l.app).await;
    let sent = blocks(&next);
    assert_eq!(sent.len(), 2, "{sent:?}");
    assert_eq!(sent[0], "report", "the person's text is first");
    assert!(sent[1].starts_with(CHANGED), "{}", sent[1]);
    assert!(sent[1].ends_with(&format!("\n\n{body}")), "{}", sent[1]);

    let after = report(&l.app).await;
    assert_eq!(blocks(&after), ["report"], "sent once");
    assert_eq!(after["bearer_hash"], next["bearer_hash"], "the grant stays");
    assert_eq!(
        (&next["resumes"], &after["resumes"]),
        (&json!(0), &json!(0)),
        "a change never resumes the session"
    );
    assert_eq!(thread_grants(&l.app.storage).await.len(), 1);
}

#[tokio::test]
async fn a_thread_from_before_this_milestone_gets_shadows_instructions_once() {
    let l = listening_app().await;
    report(&l.app).await;
    // What a Milestone 1 turn recorded: no versions, and a session created
    // without Shadows' instructions.
    sqlx::query(
        "UPDATE agent_invocation SET prompt_version = NULL, planner_instructions_version_id = NULL",
    )
    .execute(l.app.storage.reader())
    .await
    .unwrap();
    l.app.sessions.terminate(&l.app.thread).await.unwrap();

    let next = report(&l.app).await;
    assert_eq!(next["how"], "resume");
    let sent = blocks(&next);
    assert_eq!(sent.len(), 2, "{sent:?}");
    assert_eq!(sent[0], "report");
    assert!(sent[1].starts_with(OURS), "{}", sent[1]);
    assert!(sent[1].contains(PROMPT.trim_end()));
    assert!(
        !sent[1].contains(CHANGED),
        "the project has no instructions"
    );

    assert_eq!(blocks(&report(&l.app).await), ["report"]);
}

#[tokio::test]
async fn a_new_threads_first_turn_has_no_context_block() {
    let l = listening_app().await;
    let body = "  Keep every task under a day.  ";
    save_instructions(&l.app, "i1", body).await;

    let first = report(&l.app).await;
    assert_eq!(blocks(&first), ["report"]);
    let append = first["append"].as_str().unwrap();
    assert!(append.starts_with(PROMPT.trim_end()), "{append}");
    assert!(
        append.ends_with(&format!("\n\n## Project instructions\n\n{body}")),
        "{append}"
    );
}

#[tokio::test]
async fn instructions_saved_after_the_session_opened_reach_its_first_turn() {
    let l = listening_app().await;
    let path = format!("/api/threads/{}/session", l.app.thread);
    let (status, opened) = post(&l.app, &path, json!({})).await;
    assert_eq!(status, 200, "{opened}");
    let body = "Keep every task under a day.";
    save_instructions(&l.app, "i1", body).await;

    let first = report(&l.app).await;
    assert!(!first["append"].as_str().unwrap().contains(body));
    let sent = blocks(&first);
    assert_eq!(sent.len(), 2, "{sent:?}");
    assert!(
        sent[1].starts_with(CHANGED) && sent[1].ends_with(body),
        "{sent:?}"
    );
    assert_eq!(blocks(&report(&l.app).await), ["report"]);
}

#[tokio::test]
async fn a_forks_first_turn_gets_both_parts() {
    let l = listening_app().await;
    let body = "Keep every task under a day.";
    save_instructions(&l.app, "i1", body).await;
    report(&l.app).await;
    let last = app::entries(&l.app).await.last().unwrap().id.to_string();
    let path = format!("/api/threads/{}/fork", l.app.thread);
    let (status, fork) = post(
        &l.app,
        &path,
        json!({ "command_id": "f1", "at_entry_id": last }),
    )
    .await;
    assert_eq!(status, 201, "{fork}");
    let fork = fork["id"].as_str().unwrap();

    let done = start_and_finish_on(&l.app, fork, "report", default_settings()).await;
    assert_eq!(done.status_kind, "Completed", "{done:?}");
    let first: Value = serde_json::from_str(&last_agent_entry_on(&l.app, fork).await.body).unwrap();
    assert_eq!(first["how"], "fork");
    let sent = blocks(&first);
    assert_eq!(sent.len(), 2, "{sent:?}");
    assert!(
        sent[1].starts_with(OURS),
        "Shadows' instructions come first"
    );
    assert!(
        sent[1].contains(CHANGED) && sent[1].ends_with(body),
        "{}",
        sent[1]
    );
}

#[tokio::test]
async fn an_invocation_records_the_prompt_and_instructions_versions() {
    let l = listening_app().await;
    save_instructions(&l.app, "i1", "Plan small.").await;
    let current = l
        .app
        .storage
        .current_planner_instructions(&l.app.project)
        .await
        .unwrap()
        .unwrap();
    report(&l.app).await;

    let recorded: (Option<String>, Option<String>) = sqlx::query_as(
        "SELECT prompt_version, planner_instructions_version_id FROM agent_invocation",
    )
    .fetch_one(l.app.storage.reader())
    .await
    .unwrap();
    assert_eq!(
        recorded,
        (Some(prompt_version().to_string()), Some(current.id))
    );
    assert_eq!(prompt_version().len(), 16);
    assert_eq!(
        l.app
            .storage
            .latest_invocation_versions(&l.app.thread)
            .await
            .unwrap(),
        Some(recorded)
    );
}
