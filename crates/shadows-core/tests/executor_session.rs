//! An executor session is built as the Planner's is: a fixed tool list, its
//! instructions, a grant that reads its one task only, mode `auto` with no
//! permission rules, and a thread no conversation list shows. `fake_acp`'s
//! `report` prompt answers with what its session was opened with.

#[allow(dead_code)]
#[path = "fixtures/cross_plan.rs"]
mod cross_plan;

use std::borrow::Cow;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

use core_grants::{Grants, TaskScope, Token};
use core_threads::Threads;
use cross_plan::{SOURCE, command, fixture, task};
use serde_json::Value;
use shadows_core::testing::{CommandContext, Harness, LiveHandles, ProjectDirectory, Storage};
use shadows_core::testing::{acp, turn};
use shadows_core::{AppCore, GrantKind, OperationId, PlansError, ProjectId, ThreadEntryKind};
use shadows_core::{ThreadId, WorkflowId, Writer};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

/// A grant row as stored: id, kind, thread, project, token hash, created, revoked.
type GrantRow = (
    String,
    String,
    Option<String>,
    String,
    String,
    String,
    Option<String>,
);

struct Fixture {
    runtime: Arc<shadows_core::testing::Runtime>,
    storage: Arc<Storage>,
    sessions: Arc<Harness>,
    handles: Arc<LiveHandles>,
    bus: tokio::sync::broadcast::Sender<(
        ThreadId,
        OperationId,
        shadows_agent::events::HarnessEvent,
    )>,
    project: ProjectId,
    root: std::path::PathBuf,
    _tmp: tempfile::TempDir,
}

fn scope() -> TaskScope {
    TaskScope {
        workflow: "w".into(),
        version: 1,
        task: 1,
    }
}

async fn fixture_with_session() -> Fixture {
    let tmp = tempfile::tempdir().unwrap();
    let storage = Arc::new(Storage::open(&tmp.path().join("s.sqlite3")).await.unwrap());
    let (runtime, _report) = AppCore::runtime_for_tests(storage.clone()).await.unwrap();
    let ctx = CommandContext {
        agent: None,
        principal_kind: "User".into(),
        principal_id: "local".into(),
        command_id: "c1".into(),
        command_kind: "project.create".into(),
        command_schema_ver: 1,
        request_fingerprint: CommandContext::fingerprint("project.create", &Value::Null),
    };
    let dir = ProjectDirectory::resolve(tmp.path()).unwrap();
    let project = shadows_core::testing::create_project(
        &storage,
        &ctx,
        "demo",
        "Demo",
        &dir,
        &shadows_agent::policy::default_modes(),
    )
    .await
    .unwrap();
    // An executor works in its project's folder, never one of its own.
    let root = std::path::PathBuf::from(dir.as_str());
    Fixture {
        sessions: acp::fake_sessions(storage.clone()),
        runtime: Arc::new(runtime),
        storage,
        handles: Arc::new(LiveHandles::default()),
        bus: tokio::sync::broadcast::channel(16).0,
        project: project.id,
        root,
        _tmp: tmp,
    }
}

impl Fixture {
    async fn executor_thread(&self) -> ThreadId {
        let project = self.project.clone();
        self.storage
            .write_txn(move |conn| {
                Box::pin(async move {
                    Threads::create_executor_in(conn, &project, "T1".into(), "claude-code").await
                })
            })
            .await
            .unwrap()
    }

    /// On `fake-large`, the fake's one model that offers `auto`.
    async fn open(&self, thread: &ThreadId) {
        self.sessions
            .open_executor(thread, scope(), "fake-large".into())
            .await
            .unwrap();
    }

    async fn turn(&self, thread: &ThreadId, prompt: &str) -> OperationId {
        let op = turn::start_direct(
            &self.runtime,
            &self.handles,
            &self.sessions,
            &self.bus,
            thread,
            prompt,
        )
        .await
        .unwrap();
        for _ in 0..200 {
            let status =
                shadows_core::Turns::get_operation_for_test(&self.runtime.storage_for_test(), &op)
                    .await
                    .unwrap();
            if status.finished_at.is_some() {
                return op;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        panic!("the turn did not end");
    }

    async fn entries(&self, thread: &ThreadId) -> Vec<shadows_core::ThreadEntry> {
        Threads::list_thread_entries_for_test(&self.runtime.storage_for_test(), thread)
            .await
            .unwrap()
    }
}

#[tokio::test]
async fn an_executor_session_opens_with_its_instructions_tools_and_rules() {
    let f = fixture_with_session().await;
    let thread = f.executor_thread().await;
    f.open(&thread).await;

    let offered = f.sessions.offered(&thread).await.unwrap();
    assert_eq!(offered.current.mode, "auto");
    assert_eq!(offered.current.model, "fake-large");

    f.turn(&thread, "report").await;
    let entries = f.entries(&thread).await;
    let reply = entries
        .iter()
        .rev()
        .find(|e| e.kind == ThreadEntryKind::AgentMessage)
        .expect("an agent reply");
    let report: Value = serde_json::from_str(&reply.body).unwrap();
    let append = report["append"].as_str().unwrap();
    assert!(
        append.starts_with("[Shadows] You are an executor."),
        "{append}"
    );
    // The body of `executor.md`, without its front matter.
    assert!(append.contains("Never use Python"), "{append}");
    assert!(!append.contains("tools:"), "{append}");
    assert_eq!(
        report["cwd"].as_str().map(std::path::Path::new),
        Some(f.root.as_path())
    );
    let options = &report["options"];
    let tools: Vec<&str> = options["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t.as_str().unwrap())
        .collect();
    assert_eq!(
        tools,
        [
            "Read",
            "Grep",
            "Glob",
            "Edit",
            "Write",
            "MultiEdit",
            "Bash",
            "TodoWrite"
        ]
    );
    assert_eq!(options["settingSources"], serde_json::json!([]));
    assert_eq!(options["strictMcpConfig"], serde_json::json!(true));
    // In `auto` nothing is asked, so no rule is sent.
    assert!(options.get("settings").is_none(), "{options}");

    f.sessions.close_all().await.unwrap();
    f.storage.close().await.unwrap();
}

async fn executor_grant(storage: &Arc<Storage>, scope: TaskScope) -> (shadows_core::Grant, Token) {
    storage
        .write_txn(move |conn| {
            Box::pin(async move {
                Grants::issue_executor_grant_in(
                    conn,
                    &ProjectId::from_literal("p"),
                    &ThreadId::from_literal("writer"),
                    scope,
                )
                .await
            })
        })
        .await
        .unwrap()
}

#[tokio::test]
async fn an_executor_grant_reads_only_its_own_task() {
    let (_tmp, storage) = fixture().await;
    let plans = AppCore::plans_for_tests(storage.clone());
    let workflow = WorkflowId::from_literal(SOURCE);
    plans
        .edit_plan_for_test(
            &command("tasks"),
            &Writer::Person,
            None,
            &workflow,
            0,
            &[task(1), task(2)],
        )
        .await
        .unwrap();
    plans
        .approve_plan_for_test(&command("approve"), &workflow, 1)
        .await
        .unwrap();
    let own = |task, version| TaskScope {
        workflow: SOURCE.into(),
        version,
        task,
    };

    let (grant, _) = executor_grant(&storage, own(1, 1)).await;
    assert_eq!(grant.kind, GrantKind::Executor);
    let context = plans
        .task_context(&grant, Some(&workflow), 1, None)
        .await
        .unwrap();
    assert_eq!(context.task.number, 1);
    let other = plans.task_context(&grant, Some(&workflow), 2, None).await;
    assert!(matches!(other, Err(PlansError::OutsideGrant)), "{other:?}");
    // Naming no version reads the grant's own: an executor cannot list plans.
    let own_version = plans.task_context(&grant, None, 1, None).await.unwrap();
    assert_eq!(own_version.plan.workflow_id, workflow);
    assert_eq!(own_version.task.number, 1);
    let unnamed_other = plans.task_context(&grant, None, 2, None).await;
    assert!(
        matches!(unnamed_other, Err(PlansError::OutsideGrant)),
        "{unnamed_other:?}"
    );

    let (later, _) = executor_grant(&storage, own(1, 2)).await;
    let version = plans.task_context(&later, Some(&workflow), 1, None).await;
    assert!(
        matches!(version, Err(PlansError::OutsideGrant)),
        "{version:?}"
    );

    let (elsewhere, _) = executor_grant(
        &storage,
        TaskScope {
            workflow: "another-plan".into(),
            version: 1,
            task: 1,
        },
    )
    .await;
    let plan = plans
        .task_context(&elsewhere, Some(&workflow), 1, None)
        .await;
    assert!(matches!(plan, Err(PlansError::OutsideGrant)), "{plan:?}");
    storage.close().await.unwrap();
}

#[tokio::test]
async fn an_executor_grant_lists_only_its_tools() {
    assert_eq!(
        Grants::tools_for(GrantKind::Executor),
        ["task_brief", "task_question", "code_lsp", "task_report"]
    );
}

#[tokio::test]
async fn an_executor_grant_cannot_write_the_plan() {
    let tools = Grants::tools_for(GrantKind::Executor);
    for writer in [
        "plan_edit",
        "draft_start",
        "plan_approve",
        "plan_show",
        "task_get",
    ] {
        assert!(!tools.contains(&writer), "{writer} is listed");
    }
    // Were a call to reach Plans, the grant has no writer to act as.
    let (_tmp, storage) = fixture().await;
    let (grant, _) = executor_grant(&storage, scope()).await;
    assert!(Writer::from_grant(&grant, None).is_err());
    storage.close().await.unwrap();
}

#[tokio::test]
async fn an_executor_grant_from_before_a_restart_is_refused() {
    let (_tmp, storage) = fixture().await;
    let (grant, token) = executor_grant(&storage, scope()).await;
    let grants = Grants::new(storage.clone(), "http://127.0.0.1:0/mcp".into());
    assert_eq!(
        grants
            .authorize(token.as_str())
            .await
            .unwrap()
            .map(|g| g.id),
        Some(grant.id)
    );
    // The start of a new daemon.
    assert_eq!(grants.revoke_thread_grants().await.unwrap(), 1);
    assert!(grants.authorize(token.as_str()).await.unwrap().is_none());
    storage.close().await.unwrap();
}

#[tokio::test]
async fn existing_grants_survive_the_migration() {
    let tmp = tempfile::tempdir().unwrap();
    let url = format!(
        "sqlite://{}",
        tmp.path()
            .join("m.sqlite3")
            .to_string_lossy()
            .replace('\\', "/")
    );
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(
            SqliteConnectOptions::from_str(&url)
                .unwrap()
                .create_if_missing(true)
                .foreign_keys(true),
        )
        .await
        .unwrap();
    let mut before = sqlx::migrate!("../core-db/migrations");
    before.migrations = Cow::Owned(
        before
            .migrations
            .iter()
            .filter(|m| m.version <= 49)
            .cloned()
            .collect(),
    );
    before.run(&pool).await.unwrap();
    for sql in [
        "INSERT INTO project (id, slug, name, directory, created_at)
         VALUES ('P', 'p', 'P', 'C:/p', '2026-09-01T00:00:00Z')",
        "INSERT INTO planning_thread (id, project_id, title, status, created_at, title_source)
         VALUES ('T', 'P', 'T', 'Open', '2026-09-01T00:00:00Z', 'client')",
        "INSERT INTO mcp_grant (id, kind, thread_id, project_id, token_hash, created_at)
         VALUES ('G1', 'thread', 'T', 'P', 'h1', '2026-09-01T00:00:00Z'),
                ('G2', 'project', NULL, 'P', 'h2', '2026-09-01T00:00:00Z')",
        "INSERT INTO draft_intent (draft_ref, grant_id, created_at, expires_at)
         VALUES ('dr-1', 'G2', '2026-09-01T00:00:00Z', '2026-09-01T01:00:00Z')",
    ] {
        sqlx::query(sql).execute(&pool).await.unwrap();
    }
    let rows = "SELECT id, kind, thread_id, project_id, token_hash, created_at, revoked_at
                FROM mcp_grant ORDER BY id";
    let kept: Vec<GrantRow> = sqlx::query_as(rows).fetch_all(&pool).await.unwrap();

    sqlx::migrate!("../core-db/migrations")
        .run(&pool)
        .await
        .unwrap();

    let after: Vec<GrantRow> = sqlx::query_as(rows).fetch_all(&pool).await.unwrap();
    assert_eq!(kept.len(), 2);
    assert_eq!(kept, after);
    let draft: String = sqlx::query_scalar("SELECT grant_id FROM draft_intent")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(draft, "G2");
    let violations = sqlx::query("PRAGMA foreign_key_check")
        .fetch_all(&pool)
        .await
        .unwrap();
    assert!(violations.is_empty());
    sqlx::query(
        "INSERT INTO mcp_grant (id, kind, thread_id, project_id, token_hash, created_at,
                                scope_workflow, scope_version, scope_task)
         VALUES ('G3', 'executor', 'T', 'P', 'h3', '2026-09-01T00:00:00Z', 'w', 1, 1)",
    )
    .execute(&pool)
    .await
    .unwrap();
    pool.close().await;
}

#[tokio::test]
async fn executor_threads_are_not_listed_as_conversations() {
    let f = fixture_with_session().await;
    let executor = f.executor_thread().await;
    let planning = Threads::create_planning_thread_for_test(
        &f.storage,
        &CommandContext {
            agent: None,
            principal_kind: "User".into(),
            principal_id: "local".into(),
            command_id: "c2".into(),
            command_kind: "thread.create".into(),
            command_schema_ver: 1,
            request_fingerprint: CommandContext::fingerprint("thread.create", &Value::Null),
        },
        &f.project,
        "Plan",
        "claude-code",
        None,
    )
    .await
    .unwrap();

    let projects = shadows_core::Projects::new(f.storage.clone());
    let threads = Threads::new(f.storage.clone(), projects);
    let listed = threads.list(&f.project).await.unwrap();
    let ids: Vec<_> = listed.iter().map(|t| t.id.clone()).collect();
    assert_eq!(ids, vec![planning.id]);
    assert!(!ids.contains(&executor));
    f.storage.close().await.unwrap();
}
