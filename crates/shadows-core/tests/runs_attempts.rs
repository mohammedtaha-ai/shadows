//! Attempts at the tasks of an approved plan: starting one, stopping it,
//! what its session may still write, and what a restart finds. The executor
//! runs on `fake_acp`; `wait-for-release` holds its turn until a `release`
//! file appears in the project folder. `live` runs the real adapter.

#[allow(dead_code)]
#[path = "fixtures/cross_plan.rs"]
mod cross_plan;
#[path = "runs_attempts/init.rs"]
mod init;
#[path = "runs_attempts/live.rs"]
mod live;
#[path = "runs_attempts/report.rs"]
mod report;

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use core_runs::{Attempt, AttemptState, RunsError};
use cross_plan::{SOURCE, TARGET, TARGET_PLAN, command};
use serde_json::Value;
use shadows_agent::claude::ClaudeAdapter;
use shadows_core::testing::acp;
use shadows_core::testing::{CommandContext, Harness, LiveHandles, ProjectDirectory, Storage};
use shadows_core::{AcceptanceItem, AppCore, CoreParts, Link, LinkKind, PlanOp, TaskContent};
use shadows_core::{TaskParent, ThreadEntryKind, ThreadId, WorkflowId, Writer};

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

/// T1 `lib/**`, T3 `backend/app/**`, T4 `backend/app/Models/**`, T5 `web/**`,
/// T6 `docs/**` which needs T1, T7 `api/**` which needs T2 of another plan.
fn tasks() -> Vec<PlanOp> {
    vec![
        task(1, "lib/**"),
        task(3, "backend/app/**"),
        task(4, "backend/app/Models/**"),
        task(5, "web/**"),
        task(6, "docs/**"),
        task(7, "api/**"),
        PlanOp::LinkPut {
            link: Link {
                task: 6,
                after: TaskParent::Local(1),
                kind: LinkKind::Needs,
                label: "the library".into(),
                waiting_items: vec![],
            },
        },
        PlanOp::LinkPut {
            link: Link {
                task: 7,
                after: other_plan_task(),
                kind: LinkKind::Needs,
                label: "the backend".into(),
                waiting_items: vec![],
            },
        },
    ]
}

/// T2 of the other plan, in its latest version.
fn other_plan_task() -> TaskParent {
    TaskParent::Plan {
        plan_id: shadows_core::PlanId::from_literal(TARGET_PLAN),
        task: 2,
    }
}

fn git(root: &Path, args: &[&str]) -> String {
    let out = std::process::Command::new("git")
        .current_dir(root)
        .args(args)
        .output()
        .unwrap();
    assert!(out.status.success(), "git {args:?}: {:?}", out);
    String::from_utf8(out.stdout).unwrap()
}

/// A person's project: a git repository with one commit.
fn repository(root: &Path) {
    git(root, &["init", "-q"]);
    git(root, &["config", "user.email", "runs@example.invalid"]);
    git(root, &["config", "user.name", "Runs"]);
    git(root, &["config", "core.autocrlf", "false"]);
    std::fs::write(root.join("README.md"), "demo\n").unwrap();
    git(root, &["add", "README.md"]);
    git(root, &["commit", "-q", "-m", "start"]);
}

async fn fixture() -> Fixture {
    fixture_with(true, None).await
}

async fn fixture_with(approve: bool, adapter: Option<Arc<ClaudeAdapter>>) -> Fixture {
    let tmp = tempfile::tempdir().unwrap();
    let storage = Arc::new(Storage::open(&tmp.path().join("s.sqlite3")).await.unwrap());
    let root = tmp.path().join("project");
    std::fs::create_dir(&root).unwrap();
    repository(&root);
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
    let (project_id, thread_id) = (project.id.as_str(), writer.id.as_str());
    sqlx::query(
        "INSERT INTO plan (id, project_id, state, created_at)
         VALUES ('source-plan', ?, 'Active', '2026-10-04T00:00:00Z')",
    )
    .bind(project_id)
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
    .bind(thread_id)
    .execute(storage.reader())
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO plan (id, project_id, state, created_at)
         VALUES (?, ?, 'Active', '2026-10-04T00:00:00Z')",
    )
    .bind(TARGET_PLAN)
    .bind(project_id)
    .execute(storage.reader())
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO workflow
           (id, plan_id, state, version, revision, title, goal, written_by_thread,
            created_at, updated_at)
         VALUES (?, ?, 'Draft', 1, 0, 'Backend', 'A usable API', ?,
           '2026-10-04T00:00:00Z', '2026-10-04T00:00:00Z')",
    )
    .bind(TARGET)
    .bind(TARGET_PLAN)
    .bind(thread_id)
    .execute(storage.reader())
    .await
    .unwrap();
    cross_plan::approve_roadmap(&storage, project_id, &["source-plan", TARGET_PLAN]).await;
    let workflow = WorkflowId::from_literal(SOURCE);
    let plans = AppCore::plans_for_tests(storage.clone());
    plans
        .edit_plan_for_test(
            &command("target-tasks"),
            &Writer::Person,
            None,
            &WorkflowId::from_literal(TARGET),
            0,
            &[task(2, "server/**")],
        )
        .await
        .unwrap();
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
    if approve {
        plans
            .approve_plan_for_test(&command("approve"), &workflow, 1)
            .await
            .unwrap();
    }
    let (runtime, _report) = AppCore::runtime_for_tests(storage.clone()).await.unwrap();
    let runtime = Arc::new(runtime);
    let sessions = match adapter {
        Some(adapter) => AppCore::harness_for_tests(adapter, storage.clone(), acp::test_config()),
        None => acp::fake_sessions(storage.clone()),
    };
    let core = AppCore::assemble(CoreParts {
        operations: AppCore::operations_for_tests(storage.clone()),
        secrets: shadows_core::testing::secrets(storage.clone(), None),
        sessions: sessions.clone(),
        storage: storage.clone(),
        runtime: runtime.clone(),
        handles: Arc::new(LiveHandles::default()),
        bus: tokio::sync::broadcast::channel(64).0,
        ui: tokio::sync::broadcast::channel(16).0,
        mcp_url: acp::MCP_URL.to_string(),
    });
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
    /// The next attempts hold their turn until `release()`.
    fn hold(&self) {
        self.core
            .runs()
            .set_first_prompt_for_tests("wait-for-release");
    }

    fn release(&self) {
        std::fs::write(self.root.join("release"), b"").unwrap();
    }

    async fn start(&self, command: &str, task: u32) -> Result<Attempt, RunsError> {
        self.core
            .runs()
            .start(command.into(), &self.workflow, 1, task, "fake-large".into())
            .await
    }

    async fn state_of(&self, attempt: &Attempt) -> Attempt {
        self.core.runs().attempt(&attempt.id).await.unwrap()
    }

    /// Waits until the attempt leaves `Running`.
    async fn settled(&self, attempt: &Attempt) -> Attempt {
        for _ in 0..400 {
            let now = self.state_of(attempt).await;
            if now.state != AttemptState::Running {
                return now;
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
        panic!("the attempt stayed running");
    }

    async fn finish(self) {
        self.release();
        self.sessions.close_all().await.unwrap();
        self.storage.close().await.unwrap();
    }
}

/// The grant of the plan's Planner, the thread that wrote it.
async fn planner_grant(f: &Fixture) -> shadows_core::Grant {
    let thread: String = sqlx::query_scalar("SELECT written_by_thread FROM workflow WHERE id = ?")
        .bind(SOURCE)
        .fetch_one(f.storage.reader())
        .await
        .unwrap();
    let thread = ThreadId::from_literal(thread);
    shadows_core::Grants::issue_thread_grant_for_test(&f.storage, &thread)
        .await
        .unwrap()
        .0
}

/// One recorded turn of the plan's Planner on `harness`, asking for `model`,
/// committed by the turn command's own write and ended without an adapter.
async fn planner_turn(f: &Fixture, id: &str, harness: &str, model: &str) {
    use shadows_core::testing::turn::{new_turn, turn_command};
    let thread: String = sqlx::query_scalar("SELECT written_by_thread FROM workflow WHERE id = ?")
        .bind(SOURCE)
        .fetch_one(f.storage.reader())
        .await
        .unwrap();
    let thread = ThreadId::from_literal(thread);
    let runtime = shadows_core::testing::Runtime::start(f.storage.clone())
        .await
        .unwrap();
    let settings = shadows_agent::TurnSettings {
        model: model.into(),
        mode: "default".into(),
        effort: None,
    };
    let mut turn = new_turn(&thread, &runtime, "plan it", &settings);
    turn.harness_kind = harness;
    let started = shadows_core::Turns::start_turn_for_test(
        &f.storage,
        &turn_command(id, &thread, "plan it", &settings),
        turn,
    )
    .await
    .unwrap();
    shadows_core::Turns::mark_operation_failed_for_test(
        &f.storage,
        &started.operation_id,
        shadows_core::testing::FailureStage::Prepare,
        "recorded only",
    )
    .await
    .unwrap();
}

/// The suggestion is a model the executor's harness offers: a Codex
/// Planner's model is not, so the model last used on Claude is suggested.
#[tokio::test]
async fn the_suggested_model_is_one_the_executors_harness_offers() {
    let f = fixture().await;
    let suggested = |f: &Fixture| {
        let runs = f.core.runs().clone();
        let workflow = f.workflow.clone();
        async move { runs.task(&workflow, 1, 1).await.unwrap().suggested_model }
    };
    assert_eq!(suggested(&f).await, None);
    planner_turn(&f, "turn-1", "claude-code", "sonnet").await;
    assert_eq!(suggested(&f).await.as_deref(), Some("sonnet"));
    planner_turn(&f, "turn-2", "codex", "gpt-6-luna").await;
    let after_codex = suggested(&f).await;
    assert_ne!(after_codex.as_deref(), Some("gpt-6-luna"));
    assert_eq!(after_codex.as_deref(), Some("sonnet"), "{after_codex:?}");
    f.finish().await;
}

#[tokio::test]
async fn start_opens_one_attempt_on_an_executor_thread() {
    let f = fixture().await;
    f.hold();
    let attempt = f.start("s1", 3).await.unwrap();
    assert_eq!(attempt.state, AttemptState::Running);
    assert_eq!(attempt.number, 1);
    assert_eq!(attempt.model, "fake-large");
    assert!(attempt.operation_id.is_some());
    let kind: String = sqlx::query_scalar("SELECT kind FROM planning_thread WHERE id = ?")
        .bind(attempt.thread_id.as_str())
        .fetch_one(f.storage.reader())
        .await
        .unwrap();
    assert_eq!(kind, "executor");
    // A reported commit is judged new against the HEAD Start saw.
    let start_head: Option<String> =
        sqlx::query_scalar("SELECT start_head FROM run_attempts WHERE id = ?")
            .bind(attempt.id.as_str())
            .fetch_one(f.storage.reader())
            .await
            .unwrap();
    let head = git(&f.root, &["rev-parse", "HEAD"]);
    assert_eq!(start_head.as_deref(), Some(head.trim()));
    f.finish().await;
}

#[tokio::test]
async fn the_same_command_returns_the_same_attempt() {
    let f = fixture().await;
    f.hold();
    let first = f.start("s1", 3).await.unwrap();
    let second = f.start("s1", 3).await.unwrap();
    assert_eq!(first.id, second.id);
    let rows: i64 = sqlx::query_scalar("SELECT count(*) FROM run_attempts")
        .fetch_one(f.storage.reader())
        .await
        .unwrap();
    assert_eq!(rows, 1);
    let other = f
        .core
        .runs()
        .start("s1".into(), &f.workflow, 1, 3, "fake-small".into())
        .await;
    assert!(
        matches!(other, Err(RunsError::CommandConflict)),
        "{other:?}"
    );
    // A replay answers the first attempt even once the plan could not start it.
    sqlx::query("UPDATE plan SET state = 'Archived', archived_at = '2026-10-05T00:00:00Z'")
        .execute(f.storage.reader())
        .await
        .unwrap();
    assert_eq!(f.start("s1", 3).await.unwrap().id, first.id);
    f.finish().await;
}

#[tokio::test]
async fn a_second_start_while_running_is_busy() {
    let f = fixture().await;
    f.hold();
    f.start("s1", 3).await.unwrap();
    let again = f.start("s2", 3).await;
    assert!(matches!(again, Err(RunsError::TaskBusy)), "{again:?}");
    f.finish().await;
}

#[tokio::test]
async fn start_waits_for_needs_links() {
    let f = fixture().await;
    f.hold();
    let waiting = f.start("s1", 6).await;
    let one = vec![TaskParent::Local(1)];
    assert!(
        matches!(&waiting, Err(RunsError::Waiting { tasks }) if tasks == &one),
        "{waiting:?}"
    );
    let marked = f
        .core
        .runs()
        .mark_done("m1".into(), &f.workflow, 1, 1, true)
        .await
        .unwrap();
    assert!(marked.done && marked.done_at.is_some());
    assert_eq!(f.start("s2", 6).await.unwrap().state, AttemptState::Running);
    // A mark command is replayed, never applied twice, and its id is its own.
    let runs = f.core.runs();
    let other = runs.mark_done("m1".into(), &f.workflow, 1, 1, false).await;
    assert!(
        matches!(other, Err(RunsError::CommandConflict)),
        "{other:?}"
    );
    runs.mark_done("m2".into(), &f.workflow, 1, 1, false)
        .await
        .unwrap();
    let replayed = runs.mark_done("m1".into(), &f.workflow, 1, 1, true).await;
    assert!(
        !replayed.unwrap().done,
        "a replay applied an old mark again"
    );
    let elsewhere = runs.mark_done("m1".into(), &f.workflow, 1, 3, true).await;
    assert!(
        matches!(elsewhere, Err(RunsError::CommandConflict)),
        "{elsewhere:?}"
    );
    f.finish().await;
}

/// A `needs` link to another plan's task waits for that task's mark in the
/// version the link reaches. The adapter is missing, so an attempt that
/// passed every check is recorded `Failed` without any agent running.
#[tokio::test]
async fn start_waits_for_a_needs_link_to_another_plans_task() {
    let missing = acp::adapter_at(PathBuf::from("no-such-adapter"));
    let f = fixture_with(true, Some(missing)).await;
    let waiting = f.start("s1", 7).await;
    assert!(
        matches!(&waiting, Err(RunsError::Waiting { tasks }) if tasks == &vec![other_plan_task()]),
        "{waiting:?}"
    );
    let reason = waiting.unwrap_err().to_string();
    assert!(
        reason.contains(TARGET_PLAN) && reason.contains("T2"),
        "{reason}"
    );
    let target = WorkflowId::from_literal(TARGET);
    f.core
        .runs()
        .mark_done("m1".into(), &target, 1, 2, true)
        .await
        .unwrap();
    let started = f.start("s2", 7).await.unwrap();
    assert!(
        started
            .reason
            .as_deref()
            .is_some_and(|r| r.starts_with("session did not open")),
        "{started:?}"
    );
    f.storage.close().await.unwrap();
}

#[tokio::test]
async fn start_refuses_a_draft() {
    let f = fixture_with(false, None).await;
    let started = f.start("s1", 3).await;
    assert!(matches!(started, Err(RunsError::NotFrozen)), "{started:?}");
    f.storage.close().await.unwrap();
}

#[tokio::test]
async fn start_refuses_a_superseded_version() {
    let f = fixture().await;
    sqlx::query(
        "INSERT INTO workflow (id,plan_id,state,version,revision,title,goal,
         written_by_thread,previous_version_id,created_at,updated_at)
         SELECT 'next-draft',plan_id,'Draft',2,0,'Changed','Changed',
         written_by_thread,id,created_at,updated_at FROM workflow WHERE id=?",
    )
    .bind(SOURCE)
    .execute(f.storage.reader())
    .await
    .unwrap();
    let started = f.start("s1", 3).await;
    assert!(matches!(started, Err(RunsError::Superseded)), "{started:?}");
    f.storage.close().await.unwrap();
}

#[tokio::test]
async fn start_refuses_an_archived_plan() {
    let f = fixture().await;
    sqlx::query("UPDATE plan SET state = 'Archived', archived_at = '2026-10-05T00:00:00Z'")
        .execute(f.storage.reader())
        .await
        .unwrap();
    let started = f.start("s1", 3).await;
    assert!(matches!(started, Err(RunsError::Archived)), "{started:?}");
    f.storage.close().await.unwrap();
}

#[tokio::test]
async fn stop_ends_the_attempt_and_closes_its_session() {
    let f = fixture().await;
    f.hold();
    let attempt = f.start("s1", 3).await.unwrap();
    assert_eq!(f.sessions.live_count().await, 1);
    let stopped = f.core.runs().stop(&attempt.id).await.unwrap();
    assert_eq!(stopped.state, AttemptState::Stopped);
    assert!(stopped.ended_at.is_some());
    assert_eq!(f.sessions.live_count().await, 0);
    f.finish().await;
}

#[tokio::test]
async fn a_session_that_fails_to_open_fails_the_attempt() {
    let missing = acp::adapter_at(PathBuf::from("no-such-adapter"));
    let f = fixture_with(true, Some(missing)).await;
    let attempt = f.start("s1", 3).await.unwrap();
    assert_eq!(attempt.state, AttemptState::Failed);
    assert!(
        attempt
            .reason
            .as_deref()
            .unwrap()
            .starts_with("session did not open"),
        "{:?}",
        attempt.reason
    );
    let live: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM mcp_grant WHERE kind = 'executor' AND revoked_at IS NULL",
    )
    .fetch_one(f.storage.reader())
    .await
    .unwrap();
    assert_eq!(live, 0);
    // Its scope is free again.
    let again = f.start("s2", 3).await.unwrap();
    assert_eq!(again.number, 2);
    f.storage.close().await.unwrap();
}

#[tokio::test]
async fn a_turn_that_ends_without_a_commit_fails_the_attempt() {
    let f = fixture().await;
    let attempt = f.start("s1", 3).await.unwrap();
    let ended = f.settled(&attempt).await;
    assert_eq!(ended.state, AttemptState::Failed);
    assert_eq!(ended.reason.as_deref(), Some("ended without a commit"));
    assert!(ended.ended_at.is_some());
    f.finish().await;
}

#[tokio::test]
async fn recover_interrupts_running_attempts_without_restarting_them() {
    let f = fixture().await;
    sqlx::query(
        "INSERT INTO run_attempts
           (id, project_id, workflow_id, version, task, number, thread_id, model, state,
            writes_json, started_at, command_id, fingerprint)
         VALUES ('a1', ?, ?, 1, 3, 1, 'gone', 'fake-small', 'running', '[\"backend/**\"]',
                 '2026-10-05T00:00:00Z', 'old', 'f')",
    )
    .bind(f.project.as_str())
    .bind(SOURCE)
    .execute(f.storage.reader())
    .await
    .unwrap();
    let report = f.core.runs().recover().await.unwrap();
    assert_eq!(report.interrupted, 1);
    let attempt = f
        .core
        .runs()
        .attempt(&core_runs::AttemptId::from_recorded("a1".into()))
        .await
        .unwrap();
    assert_eq!(attempt.state, AttemptState::Interrupted);
    assert!(attempt.ended_at.is_some());
    // Nothing was restarted, and the scope it held is free.
    assert_eq!(f.sessions.live_count().await, 0);
    let held: i64 =
        sqlx::query_scalar("SELECT count(*) FROM run_attempts WHERE released_at IS NULL")
            .fetch_one(f.storage.reader())
            .await
            .unwrap();
    assert_eq!(held, 0);
    assert_eq!(f.core.runs().recover().await.unwrap().interrupted, 0);
    f.storage.close().await.unwrap();
}

#[tokio::test]
async fn overlapping_tasks_cannot_start_together() {
    let f = fixture().await;
    f.hold();
    f.start("s1", 3).await.unwrap();
    let overlap = f.start("s2", 4).await;
    assert!(
        matches!(overlap, Err(RunsError::Overlap { task: 3 })),
        "{overlap:?}"
    );
    f.finish().await;
}

#[tokio::test]
async fn disjoint_tasks_start_together() {
    let f = fixture().await;
    f.hold();
    let (a, b) = (f.start("s1", 3).await, f.start("s2", 5).await);
    assert_eq!(a.unwrap().state, AttemptState::Running);
    assert_eq!(b.unwrap().state, AttemptState::Running);
    f.finish().await;
}

#[tokio::test]
async fn a_committed_attempt_holds_its_scope_until_its_turn_ends() {
    let f = fixture().await;
    f.hold();
    let attempt = f.start("s1", 3).await.unwrap();
    sqlx::query("UPDATE run_attempts SET state = 'committed', ended_at = ? WHERE id = ?")
        .bind("2026-10-05T00:00:00Z")
        .bind(attempt.id.as_str())
        .execute(f.storage.reader())
        .await
        .unwrap();
    let held = f.start("s2", 4).await;
    assert!(
        matches!(held, Err(RunsError::Overlap { task: 3 })),
        "{held:?}"
    );
    f.release();
    for n in 0..400 {
        match f.start(&format!("retry-{n}"), 4).await {
            Ok(_) => return f.finish().await,
            Err(RunsError::Overlap { .. }) => tokio::time::sleep(Duration::from_millis(25)).await,
            Err(other) => panic!("{other:?}"),
        }
    }
    panic!("the scope was never released");
}

#[tokio::test]
async fn two_concurrent_starts_of_overlapping_tasks_admit_one() {
    let f = fixture().await;
    f.hold();
    let (a, b) = tokio::join!(f.start("s1", 3), f.start("s2", 4));
    let outcomes = [a, b];
    assert_eq!(outcomes.iter().filter(|o| o.is_ok()).count(), 1);
    assert_eq!(
        outcomes
            .iter()
            .filter(|o| matches!(o, Err(RunsError::Overlap { .. })))
            .count(),
        1
    );
    f.finish().await;
}

async fn reply_of(f: &Fixture, thread: &ThreadId) -> Value {
    let entries = shadows_core::Threads::list_thread_entries_for_test(&f.storage, thread)
        .await
        .unwrap();
    let reply = entries
        .iter()
        .rev()
        .find(|e| e.kind == ThreadEntryKind::AgentMessage)
        .expect("an agent reply");
    serde_json::from_str(&reply.body).unwrap()
}

#[tokio::test]
async fn a_released_executor_thread_does_not_open_again() {
    let f = fixture().await;
    f.core.runs().set_first_prompt_for_tests("report");
    let attempt = f.start("s1", 3).await.unwrap();
    f.settled(&attempt).await;
    let executor = reply_of(&f, &attempt.thread_id).await;
    assert!(
        executor["options"]["tools"]
            .as_array()
            .unwrap()
            .contains(&Value::from("Edit"))
    );
    // Once released, the thread opens neither as an executor nor as a
    // Planner, whose grant could write the plan.
    for _ in 0..100 {
        if f.sessions.live_count().await == 0 {
            break;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    let reopened = f.sessions.open(&attempt.thread_id).await;
    assert!(reopened.is_err(), "an executor thread reopened on its own");
    assert_eq!(f.sessions.live_count().await, 0);
    let live: i64 = sqlx::query_scalar("SELECT count(*) FROM mcp_grant WHERE revoked_at IS NULL")
        .fetch_one(f.storage.reader())
        .await
        .unwrap();
    assert_eq!(live, 0);
    f.finish().await;
}

#[tokio::test]
async fn an_executors_first_turn_hears_no_planner_instructions() {
    let f = fixture().await;
    f.core
        .instructions()
        .save("i1".into(), &f.project, "Always answer in French.")
        .await
        .unwrap();
    f.core.runs().set_first_prompt_for_tests("report");
    let attempt = f.start("s1", 3).await.unwrap();
    f.settled(&attempt).await;
    let report = reply_of(&f, &attempt.thread_id).await;
    let blocks = report["blocks"].to_string();
    assert!(!blocks.contains("Planner instructions"), "{blocks}");
    assert!(!blocks.contains("French"), "{blocks}");
    // Nor the Planner's planning context: the executor reads its task, not the plan.
    assert!(!blocks.contains("Planning context"), "{blocks}");
    assert!(!blocks.contains("Stage:"), "{blocks}");
    let carried: i64 =
        sqlx::query_scalar("SELECT count(*) FROM invocation_context WHERE thread_id = ?")
            .bind(attempt.thread_id.as_str())
            .fetch_one(f.storage.reader())
            .await
            .unwrap();
    assert_eq!(carried, 0, "planning context was recorded for an executor");
    f.finish().await;
}
