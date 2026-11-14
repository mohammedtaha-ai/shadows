//! The Planner's ready group: tasks of one approved version that may start now
//! and whose writes meet neither a held scope nor each other. A group is an
//! observation; every Start checks admission again. The executor runs on
//! `fake_acp`, held until a `release` file appears.

#[allow(dead_code)]
#[path = "fixtures/cross_plan.rs"]
mod cross_plan;

use std::path::PathBuf;
use std::sync::Arc;

use core_runs::{Attempt, AttemptState, RunsError};
use cross_plan::{SOURCE, command};
use serde_json::Value;
use shadows_core::testing::acp;
use shadows_core::testing::{CommandContext, Harness, LiveHandles, ProjectDirectory, Storage};
use shadows_core::{
    AcceptanceItem, AppCore, CoreParts, Grant, Link, LinkKind, PlanOp, ReadyGroup, TaskContent,
    TaskParent, ThreadId, WorkflowId, Writer,
};

struct Fixture {
    core: Arc<AppCore>,
    storage: Arc<Storage>,
    sessions: Arc<Harness>,
    workflow: WorkflowId,
    root: PathBuf,
    project: shadows_core::ProjectId,
    _tmp: tempfile::TempDir,
}

fn task(number: u32, writes: &str) -> PlanOp {
    PlanOp::TaskAdd {
        task: TaskContent {
            phases: Vec::new(),
            contract: None,
            number,
            title: format!("Task {number}"),
            goal: "A usable result".into(),
            reads: vec![],
            writes: vec![writes.into()],
            acceptance: vec![AcceptanceItem {
                number: 1,
                text: "The result works".into(),
            }],
        },
    }
}

/// T1 `lib/**`, T2 `lib/x.rs` (inside T1's writes), T3 `web/**`, T4 `docs/**`
/// which needs T1.
fn tasks() -> Vec<PlanOp> {
    vec![
        task(4, "docs/**"),
        task(3, "web/**"),
        task(2, "lib/x.rs"),
        task(1, "lib/**"),
        PlanOp::LinkPut {
            link: Link {
                task: 4,
                after: TaskParent::Local(1),
                kind: LinkKind::Needs,
                label: "the library".into(),
                waiting_items: vec![],
            },
        },
    ]
}

fn git(root: &std::path::Path, args: &[&str]) {
    let out = std::process::Command::new("git")
        .current_dir(root)
        .args(args)
        .output()
        .unwrap();
    assert!(out.status.success(), "git {args:?}: {out:?}");
}

async fn fixture() -> Fixture {
    let tmp = tempfile::tempdir().unwrap();
    let storage = Arc::new(Storage::open(&tmp.path().join("s.sqlite3")).await.unwrap());
    let root = tmp.path().join("project");
    std::fs::create_dir(&root).unwrap();
    git(&root, &["init", "-q"]);
    git(&root, &["config", "user.email", "runs@example.invalid"]);
    git(&root, &["config", "user.name", "Runs"]);
    std::fs::write(root.join("README.md"), "demo\n").unwrap();
    git(&root, &["add", "README.md"]);
    git(&root, &["commit", "-q", "-m", "start"]);
    let ctx = |id: &str, kind: &str| CommandContext {
        agent: None,
        principal_kind: "User".into(),
        principal_id: "local".into(),
        command_id: id.into(),
        command_kind: kind.into(),
        command_schema_ver: 1,
        request_fingerprint: CommandContext::fingerprint(kind, &Value::Null),
    };
    let project = shadows_core::testing::create_project(
        &storage,
        &ctx("c1", "project.create"),
        "demo",
        "Demo",
        &ProjectDirectory::resolve(&root).unwrap(),
        &shadows_agent::policy::default_modes(),
    )
    .await
    .unwrap();
    let writer = shadows_core::Threads::create_planning_thread_for_test(
        &storage,
        &ctx("c2", "thread.create"),
        &project.id,
        "Planner",
        "claude-code",
        None,
    )
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO plan (id, project_id, state, created_at)
         VALUES ('source-plan', ?, 'Active', '2026-10-04T00:00:00Z')",
    )
    .bind(project.id.as_str())
    .execute(storage.reader())
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO workflow
           (id, plan_id, state, version, revision, title, goal, written_by_thread,
            created_at, updated_at)
         VALUES (?, 'source-plan', 'Draft', 1, 0, 'Web', 'A usable web app', ?,
           '2026-10-04T00:00:00Z', '2026-10-04T00:00:00Z')",
    )
    .bind(SOURCE)
    .bind(writer.id.as_str())
    .execute(storage.reader())
    .await
    .unwrap();
    cross_plan::approve_roadmap(&storage, project.id.as_str(), &["source-plan"]).await;
    let workflow = WorkflowId::from_literal(SOURCE);
    let plans = AppCore::plans_for_tests(storage.clone());
    plans
        .edit_plan_for_test(
            &command("tasks"),
            &Writer::Person,
            None,
            &workflow,
            0,
            &tasks(),
        )
        .await
        .unwrap();
    plans
        .approve_plan_for_test(&command("approve"), &workflow, 1)
        .await
        .unwrap();
    let (runtime, _report) = AppCore::runtime_for_tests(storage.clone()).await.unwrap();
    let sessions = acp::fake_sessions(storage.clone());
    let core = AppCore::assemble(CoreParts {
        operations: AppCore::operations_for_tests(storage.clone()),
        secrets: shadows_core::testing::secrets(storage.clone(), None),
        sessions: sessions.clone(),
        storage: storage.clone(),
        runtime: Arc::new(runtime),
        handles: Arc::new(LiveHandles::default()),
        bus: tokio::sync::broadcast::channel(64).0,
        ui: tokio::sync::broadcast::channel(16).0,
        mcp_url: acp::MCP_URL.to_string(),
    });
    core.runs().set_first_prompt_for_tests("wait-for-release");
    Fixture {
        core,
        storage,
        sessions,
        workflow,
        root,
        project: project.id,
        _tmp: tmp,
    }
}

impl Fixture {
    async fn start(&self, command: &str, task: u32) -> Result<Attempt, RunsError> {
        self.core
            .runs()
            .start(command.into(), &self.workflow, 1, task, "fake-large".into())
            .await
    }

    /// The grant of the plan's Planner, the thread that wrote it.
    async fn planner(&self) -> Grant {
        let thread: String =
            sqlx::query_scalar("SELECT written_by_thread FROM workflow WHERE id = ?")
                .bind(SOURCE)
                .fetch_one(self.storage.reader())
                .await
                .unwrap();
        shadows_core::Grants::issue_thread_grant_for_test(
            &self.storage,
            &ThreadId::from_literal(thread),
        )
        .await
        .unwrap()
        .0
    }

    /// The task numbers of the Planner's current ready group.
    async fn group(&self) -> Vec<u32> {
        let grant = self.planner().await;
        let group: ReadyGroup = self
            .core
            .runs()
            .ready_for(&grant, &self.workflow)
            .await
            .unwrap();
        assert_eq!(group.workflow_id, self.workflow);
        assert_eq!(group.version, 1);
        group.tasks.iter().map(|t| t.task).collect()
    }

    /// An ended attempt at `task` in `state`, holding its scope unless
    /// `released`, as a judged or failed attempt leaves one.
    async fn ended_attempt(&self, task: u32, state: &'static str, released: bool) {
        let (project, workflow) = (self.project.clone(), self.workflow.clone());
        let id = format!("{state}-{task}");
        let released = released.then_some("2026-10-05T00:01:00Z");
        self.storage
            .write_txn(move |conn| {
                Box::pin(async move {
                    let thread = shadows_core::Threads::create_executor_in(
                        conn,
                        &project,
                        format!("T{task}"),
                        "claude-code",
                    )
                    .await?;
                    sqlx::query(
                        "INSERT INTO run_attempts
                           (id, project_id, workflow_id, version, task, number, thread_id,
                            model, state, commit_sha, writes_json, started_at, ended_at,
                            released_at, command_id, fingerprint)
                         VALUES (?, ?, ?, 1, ?, 1, ?, 'fake-large', ?, NULL,
                                 '[\"lib/**\"]', '2026-10-05T00:00:00Z',
                                 '2026-10-05T00:01:00Z', ?, ?, 'f')",
                    )
                    .bind(&id)
                    .bind(project.as_str())
                    .bind(workflow.as_str())
                    .bind(i64::from(task))
                    .bind(thread.as_str())
                    .bind(state)
                    .bind(released)
                    .bind(&id)
                    .execute(&mut *conn)
                    .await?;
                    Ok(())
                })
            })
            .await
            .unwrap();
    }

    /// A corrective Draft after version 1.
    async fn corrective_draft(&self) {
        sqlx::query(
            "INSERT INTO workflow (id,plan_id,state,version,revision,title,goal,
             written_by_thread,previous_version_id,created_at,updated_at)
             SELECT 'next-draft',plan_id,'Draft',2,0,'Changed','Changed',
             written_by_thread,id,created_at,updated_at FROM workflow WHERE id=?",
        )
        .bind(SOURCE)
        .execute(self.storage.reader())
        .await
        .unwrap();
    }

    async fn attempts(&self) -> i64 {
        sqlx::query_scalar("SELECT count(*) FROM run_attempts")
            .fetch_one(self.storage.reader())
            .await
            .unwrap()
    }

    async fn finish(self) {
        std::fs::write(self.root.join("release"), b"").unwrap();
        self.sessions.close_all().await.unwrap();
        self.storage.close().await.unwrap();
    }
}

/// T2 may start on its own, yet it meets T1 inside the group, so only the
/// lower-numbered of the two is offered.
#[tokio::test]
async fn the_ready_group_is_pairwise_disjoint() {
    let f = fixture().await;
    let alone = f.core.runs().task(&f.workflow, 1, 2).await.unwrap();
    assert!(alone.start.allowed, "{:?}", alone.start);
    let group = f.group().await;
    assert!(group.contains(&1) && !group.contains(&2), "{group:?}");
    f.finish().await;
}

#[tokio::test]
async fn first_group_has_two_independent_tasks() {
    let f = fixture().await;
    let grant = f.planner().await;
    let group = f.core.runs().ready_for(&grant, &f.workflow).await.unwrap();
    let listed: Vec<(u32, &str)> = group
        .tasks
        .iter()
        .map(|t| (t.task, t.title.as_str()))
        .collect();
    assert_eq!(listed, [(1, "Task 1"), (3, "Task 3")], "T4 waits on T1");
    // Both start without waiting for one another.
    for task in [1, 3] {
        let started = f
            .core
            .runs()
            .start_for(&grant, &f.workflow, task, Some("fake-large".into()));
        assert_eq!(started.await.unwrap().state, AttemptState::Running);
    }
    f.finish().await;
}

#[tokio::test]
async fn held_commits_block_groups() {
    let f = fixture().await;
    f.ended_attempt(1, "committed", false).await;
    assert_eq!(f.group().await, [3], "T1's commit still holds lib/**");
    f.core
        .runs()
        .mark_done("m1".into(), &f.workflow, 1, 1, true)
        .await
        .unwrap();
    assert_eq!(f.group().await, [2, 3, 4], "the judged commit freed lib/**");
    f.finish().await;
}

#[tokio::test]
async fn a_stopped_task_is_not_automatically_retried() {
    let f = fixture().await;
    f.ended_attempt(1, "stopped", true).await;
    assert_eq!(f.group().await, [2, 3]);
    f.ended_attempt(3, "failed", true).await;
    assert_eq!(f.group().await, [2]);
    f.finish().await;
}

#[tokio::test]
async fn a_stop_unblocks_a_never_attempted_task() {
    let f = fixture().await;
    let running = f.start("s1", 1).await.unwrap();
    assert_eq!(f.group().await, [3], "T2 meets T1's held scope");
    let stopped = f.core.runs().stop(&running.id).await.unwrap();
    assert_eq!(stopped.state, AttemptState::Stopped);
    assert_eq!(f.group().await, [2, 3], "T1 is not offered again");
    f.finish().await;
}

#[tokio::test]
async fn a_stale_group_cannot_bypass_start() {
    let f = fixture().await;
    let grant = f.planner().await;
    assert_eq!(f.group().await, [1, 3]);
    // T2 starts outside the group before the Planner acts on it.
    f.start("s2", 2).await.unwrap();
    let stale = f
        .core
        .runs()
        .start_for(&grant, &f.workflow, 1, Some("fake-large".into()))
        .await;
    assert!(matches!(stale, Err(RunsError::Overlap { .. })), "{stale:?}");
    assert_eq!(f.attempts().await, 1, "the refused Start wrote nothing");
    assert_eq!(f.group().await, [3]);
    f.finish().await;
}

#[tokio::test]
async fn judging_old_work_returns_no_old_next() {
    let f = fixture().await;
    let grant = f.planner().await;
    f.ended_attempt(1, "committed", false).await;
    f.corrective_draft().await;
    let marked = f
        .core
        .runs()
        .mark_done_for(&grant, &f.workflow, 1, true)
        .await
        .unwrap();
    assert!(marked.task.done, "the old work is judged");
    assert!(marked.next.is_empty(), "{:?}", marked.next);
    let refused = f.core.runs().ready_for(&grant, &f.workflow).await;
    assert!(matches!(refused, Err(RunsError::Superseded)), "{refused:?}");
    assert_eq!(f.attempts().await, 1, "no old work started");
    f.finish().await;
}

#[tokio::test]
async fn non_planner_grants_cannot_request_groups() {
    let f = fixture().await;
    let project = shadows_core::Grants::issue_project_grant_for_test(
        &f.storage,
        &command("project-grant"),
        &f.project,
    )
    .await
    .unwrap()
    .grant;
    let executor = Grant {
        id: shadows_core::GrantId::generate(),
        kind: shadows_core::GrantKind::Executor,
        project_id: f.project.clone(),
        thread_id: None,
        created_at: "2026-11-14T00:00:00Z".into(),
        revoked_at: None,
        scope: Some(shadows_core::testing::TaskScope {
            workflow: SOURCE.into(),
            version: 1,
            task: 1,
        }),
    };
    for grant in [executor, project] {
        let refused = f.core.runs().ready_for(&grant, &f.workflow).await;
        assert!(matches!(refused, Err(RunsError::NotFound)), "{refused:?}");
    }
    f.finish().await;
}
