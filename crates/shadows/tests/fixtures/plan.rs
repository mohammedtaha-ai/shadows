//! Plans in tests: starting, editing and approving them as a person would,
//! reading what the journal recorded, and the grant rows the MCP server will
//! issue. Include beside `app.rs`:
//!
//! ```ignore
//! #[path = "fixtures/plan.rs"] mod plan;
//! ```
//!
//! Grants are issued through storage's own API. Revoking and draft refs stay
//! direct SQL: a test needs those rows in states the API does not make on
//! demand, such as a ref that expired an hour ago.

#![allow(dead_code)]

use serde_json::{Value, json};
use shadows_core::GrantId;
use shadows_core::ProjectId;
use shadows_core::ThreadId;
use shadows_core::testing::{Anchor, derived_id};
use shadows_core::testing::{CommandContext, Writer, fingerprint};
use shadows_core::{
    AcceptanceItem, DraftStarted, Link, LinkKind, PlanId, PlanOp, TaskContent, WorkflowId,
};

use super::app::{App, ctx};

/// A command by `writer` with an explicit id, fingerprinting `params`.
pub fn writer_ctx(
    writer: &Writer,
    command_id: &str,
    kind: &str,
    params: serde_json::Value,
) -> CommandContext {
    let (principal_kind, principal_id) = writer.principal();
    CommandContext {
        principal_kind: principal_kind.into(),
        principal_id,
        command_id: command_id.into(),
        command_kind: kind.into(),
        command_schema_ver: 1,
        request_fingerprint: fingerprint(kind, &params),
    }
}

/// A `plan_edit` as a caller that names no command id builds it (§13.5): the
/// id derived from the expected revision and the request's fingerprint.
pub fn edit_ctx(
    writer: &Writer,
    workflow: &WorkflowId,
    expected: i64,
    ops: &[PlanOp],
) -> CommandContext {
    let params = json!({ "workflow": workflow, "expected_revision": expected, "ops": ops });
    let fp = fingerprint("PlanEdit", &params);
    let (principal_kind, principal_id) = writer.principal();
    CommandContext {
        principal_kind: principal_kind.into(),
        principal_id,
        command_id: derived_id(Anchor::Revision(expected), &fp),
        command_kind: "PlanEdit".into(),
        command_schema_ver: 1,
        request_fingerprint: fp,
    }
}

/// A task with a goal, one acceptance item and one path it writes: enough to
/// approve.
pub fn task(n: u32, title: &str) -> TaskContent {
    TaskContent {
        number: n,
        title: title.into(),
        goal: format!("goal {n}"),
        reads: vec![format!("docs/t{n}.md")],
        writes: vec![format!("src/t{n}.rs")],
        acceptance: vec![AcceptanceItem {
            number: 1,
            text: format!("T{n} works"),
        }],
    }
}

pub fn add(n: u32) -> PlanOp {
    PlanOp::TaskAdd {
        task: task(n, &format!("task {n}")),
    }
}

pub fn needs(task: u32, after: u32) -> PlanOp {
    PlanOp::LinkPut {
        link: Link {
            task,
            after: after.into(),
            kind: LinkKind::Needs,
            label: "api".into(),
            waiting_items: vec![],
        },
    }
}

/// A grant of `kind` (`thread` or `project`), issued as the daemon issues
/// one; answers its id. A thread grant takes its thread's own project.
pub async fn insert_grant(
    app: &App,
    kind: &str,
    project: &ProjectId,
    thread: Option<&ThreadId>,
) -> GrantId {
    issue_grant(app, kind, project, thread).await.0
}

/// As `insert_grant`, with the bearer token the grant answers to: what an MCP
/// client sends.
pub async fn issue_grant(
    app: &App,
    kind: &str,
    project: &ProjectId,
    thread: Option<&ThreadId>,
) -> (GrantId, String) {
    let (grant, token) = match (kind, thread) {
        ("thread", Some(thread)) => app.storage.issue_thread_grant(thread).await.unwrap(),
        ("project", None) => {
            let command = format!("grant-{}", uuid::Uuid::new_v4());
            let c = writer_ctx(&Writer::Person, &command, "McpGrantIssue", json!({}));
            let issued = app.storage.issue_project_grant(&c, project).await.unwrap();
            (
                issued.grant,
                issued.token.expect("a new grant shows its token"),
            )
        }
        other => panic!("no such grant: {other:?}"),
    };
    assert_eq!(&grant.project_id, project);
    (grant.id, token.as_str().to_string())
}

pub async fn revoke_grant(app: &App, grant: &GrantId) {
    sqlx::query("UPDATE mcp_grant SET revoked_at = '2026-09-25T00:00:01Z' WHERE id = ?")
        .bind(grant.as_str())
        .execute(app.storage.reader())
        .await
        .unwrap();
}

/// A `draft_intent` row held by `grant`, unused, expiring at `expires_at`
/// (RFC 3339); answers the ref.
pub async fn insert_draft_ref(app: &App, grant: &GrantId, expires_at: &str) -> String {
    let draft_ref = format!("dr-{}", uuid::Uuid::new_v4());
    sqlx::query(
        "INSERT INTO draft_intent (draft_ref, grant_id, created_at, expires_at)
         VALUES (?,?, '2026-09-25T00:00:00Z', ?)",
    )
    .bind(&draft_ref)
    .bind(grant.as_str())
    .bind(expires_at)
    .execute(app.storage.reader())
    .await
    .unwrap();
    draft_ref
}

/// The plan a `draft_intent` row is bound to, if any.
pub async fn draft_ref_binding(app: &App, draft_ref: &str) -> Option<String> {
    sqlx::query_scalar("SELECT workflow_id FROM draft_intent WHERE draft_ref = ?")
        .bind(draft_ref)
        .fetch_one(app.storage.reader())
        .await
        .unwrap()
}

/// An hour from now, and an hour ago, as the daemon writes times.
pub fn in_an_hour() -> String {
    offset(time::Duration::HOUR)
}

pub fn an_hour_ago() -> String {
    offset(-time::Duration::HOUR)
}

fn offset(by: time::Duration) -> String {
    (time::OffsetDateTime::now_utc() + by)
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap()
}

/// `(kind, actor_kind, actor_id, payload)` of every event on `thread`, in order.
pub async fn events(app: &App, thread: &ThreadId) -> Vec<(String, String, String, Value)> {
    let rows: Vec<(String, String, String, String)> = sqlx::query_as(
        "SELECT kind, actor_kind, actor_id, payload_json FROM durable_event
          WHERE thread_id = ? ORDER BY seq",
    )
    .bind(thread.as_str())
    .fetch_all(app.storage.reader())
    .await
    .unwrap();
    rows.into_iter()
        .map(|(k, ak, ai, p)| (k, ak, ai, serde_json::from_str(&p).unwrap()))
        .collect()
}

/// The payloads of every `kind` event of `project`, in order: a plan event
/// names a thread only when a Planner wrote it (§16.3).
pub async fn project_events_of(app: &App, project: &ProjectId, kind: &str) -> Vec<Value> {
    let rows: Vec<String> = sqlx::query_scalar(
        "SELECT payload_json FROM durable_event WHERE project_id = ? AND kind = ? ORDER BY seq",
    )
    .bind(project.as_str())
    .bind(kind)
    .fetch_all(app.storage.reader())
    .await
    .unwrap();
    rows.iter()
        .map(|p| serde_json::from_str(p).unwrap())
        .collect()
}

pub async fn events_of(app: &App, thread: &ThreadId, kind: &str) -> Vec<Value> {
    events(app, thread)
        .await
        .into_iter()
        .filter(|e| e.0 == kind)
        .map(|e| e.3)
        .collect()
}

/// v1 of the app's thread, started by its Planner.
pub async fn draft(app: &App) -> DraftStarted {
    draft_on(app, &app.thread, "start-1").await
}

/// `thread`'s Planner, under a new thread grant: since §16.3 a version is
/// written by a Planner's thread or an external agent's grant, never by a
/// person.
pub async fn planner(app: &App, thread: &ThreadId) -> Writer {
    let grant = insert_grant(app, "thread", &project_of(app, thread).await, Some(thread)).await;
    Writer::Planner {
        thread: thread.clone(),
        grant,
    }
}

pub async fn project_of(app: &App, thread: &ThreadId) -> ProjectId {
    let project: String = sqlx::query_scalar("SELECT project_id FROM planning_thread WHERE id = ?")
        .bind(thread.as_str())
        .fetch_one(app.storage.reader())
        .await
        .unwrap();
    ProjectId::from_literal(&project)
}

/// The plan `thread` wrote, if any.
pub async fn plan_of(app: &App, thread: &ThreadId) -> Option<PlanId> {
    let latest = app.storage.thread_plan(thread).await.unwrap()?;
    Some(app.storage.get_plan(&latest).await.unwrap().plan_id)
}

/// The thread's Planner starts a draft outside a turn: v1 when the thread has
/// no plan, else that plan's Draft or its next version, with a reason
/// (§16.3).
pub async fn draft_on(app: &App, thread: &ThreadId, command: &str) -> DraftStarted {
    let writer = planner(app, thread).await;
    let plan = plan_of(app, thread).await;
    app.storage
        .start_draft(
            &writer_ctx(&writer, command, "DraftStart", json!({ "t": thread })),
            &writer,
            &project_of(app, thread).await,
            plan.as_ref(),
            None,
            Some(("Login", "people can log in")),
            Some("the plan changed"),
            None,
            None,
        )
        .await
        .unwrap()
}

pub async fn edit(app: &App, workflow: &WorkflowId, expected: i64, ops: &[PlanOp]) -> i64 {
    app.storage
        .edit_plan(
            &edit_ctx(&Writer::Person, workflow, expected, ops),
            &Writer::Person,
            None,
            workflow,
            expected,
            ops,
        )
        .await
        .unwrap()
        .revision
}

/// A v1 with T1 and T2 needing T1, approved.
pub async fn approved_v1(app: &App) -> WorkflowId {
    let v1 = draft(app).await.workflow_id;
    edit(app, &v1, 0, &[add(1), add(2), needs(2, 1)]).await;
    app.storage
        .approve_plan(&ctx("approve-1", "PlanApprove"), &v1, 1)
        .await
        .unwrap();
    v1
}

pub async fn revision(app: &App, workflow: &WorkflowId) -> i64 {
    app.storage.get_plan(workflow).await.unwrap().revision
}
