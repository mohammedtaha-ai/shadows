//! The brief each attempt receives: the task context Plans assembles is saved
//! inside the attempt's own write, read back unchanged however the project's
//! map or standards move on, and never invented for an attempt that has none.
//! The executor runs on `fake_acp`, held until a `release` file appears.

#[allow(dead_code)]
#[path = "fixtures/cross_plan.rs"]
mod cross_plan;

use std::path::{Path, PathBuf};
use std::sync::Arc;

use core_runs::{Attempt, AttemptState, RunsError};
use cross_plan::{SOURCE, command};
use serde_json::Value;
use shadows_core::testing::acp;
use shadows_core::testing::{CommandContext, Harness, LiveHandles, ProjectDirectory, Storage};
use shadows_core::{
    AcceptanceItem, AppCore, CoreParts, Grant, GrantId, GrantKind, PlanOp, TaskContent, ThreadId,
    WorkflowId, Writer,
};

struct Fixture {
    core: Arc<AppCore>,
    storage: Arc<Storage>,
    sessions: Arc<Harness>,
    workflow: WorkflowId,
    root: PathBuf,
    project: shadows_core::ProjectId,
    planner: ThreadId,
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

fn git(root: &Path, args: &[&str]) -> String {
    let out = std::process::Command::new("git")
        .current_dir(root)
        .args(args)
        .output()
        .unwrap();
    assert!(out.status.success(), "git {args:?}: {:?}", out);
    String::from_utf8(out.stdout).unwrap()
}

fn repository(root: &Path) {
    git(root, &["init", "-q"]);
    git(root, &["config", "user.email", "runs@example.invalid"]);
    git(root, &["config", "user.name", "Runs"]);
    git(root, &["config", "core.autocrlf", "false"]);
    for path in ["README.md", "lib/a.rs", "docs/a.md"] {
        let at = root.join(path);
        std::fs::create_dir_all(at.parent().unwrap()).unwrap();
        std::fs::write(at, "demo\n").unwrap();
        git(root, &["add", path]);
    }
    git(root, &["commit", "-q", "-m", "start"]);
}

async fn fixture() -> Fixture {
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
            &[task(1, "lib/**"), task(2, "docs/**")],
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
        planner: writer.id,
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

    fn executor(&self, attempt: &Attempt) -> Grant {
        Grant {
            id: GrantId::generate(),
            kind: GrantKind::Executor,
            project_id: self.project.clone(),
            thread_id: Some(attempt.thread_id.clone()),
            created_at: "2026-11-14T00:00:00Z".into(),
            revoked_at: None,
            scope: Some(shadows_core::testing::TaskScope {
                workflow: SOURCE.into(),
                version: 1,
                task: attempt.task,
            }),
        }
    }

    fn planner(&self) -> Grant {
        Grant {
            id: GrantId::generate(),
            kind: GrantKind::Thread,
            project_id: self.project.clone(),
            thread_id: Some(self.planner.clone()),
            created_at: "2026-11-14T00:00:00Z".into(),
            revoked_at: None,
            scope: None,
        }
    }

    /// Saved contexts, and attempts written so far.
    async fn written(&self) -> (i64, i64) {
        let contexts: i64 = sqlx::query_scalar("SELECT count(*) FROM run_attempt_context")
            .fetch_one(self.storage.reader())
            .await
            .unwrap();
        let attempts: i64 = sqlx::query_scalar("SELECT count(*) FROM run_attempts")
            .fetch_one(self.storage.reader())
            .await
            .unwrap();
        (contexts, attempts)
    }

    async fn saved_json(&self, attempt: &Attempt) -> String {
        sqlx::query_scalar("SELECT context_json FROM run_attempt_context WHERE attempt_id = ?")
            .bind(attempt.id.as_str())
            .fetch_one(self.storage.reader())
            .await
            .unwrap()
    }

    /// The project's map moves one revision on and a standards addition
    /// version `number` is recorded.
    async fn change_layout(&self, number: i64) {
        sqlx::query(
            "INSERT INTO design_workspace
               (project_id, revision, vision_revision, vision_content, map_revision)
             VALUES (?, 0, 0, '{}', 1)
             ON CONFLICT(project_id) DO UPDATE SET map_revision = map_revision + 1",
        )
        .bind(self.project.as_str())
        .execute(self.storage.reader())
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO standards_additions_version
               (id, project_id, number, content_json, created_at)
             VALUES (?, ?, ?, '{\"rules\":[],\"parts\":[]}', '2026-11-14T00:00:00Z')",
        )
        .bind(format!("additions-{number}"))
        .bind(self.project.as_str())
        .bind(number)
        .execute(self.storage.reader())
        .await
        .unwrap();
    }

    /// Commits a change to `lib/a.rs` and reports it for `attempt`.
    async fn commit_and_report(&self, attempt: &Attempt, text: &str) {
        std::fs::write(self.root.join("lib/a.rs"), text).unwrap();
        git(&self.root, &["add", "--", "lib/a.rs"]);
        git(&self.root, &["commit", "-q", "-m", "T1"]);
        let sha = git(&self.root, &["rev-parse", "HEAD"]).trim().to_string();
        let reported = self
            .core
            .runs()
            .report(&self.executor(attempt), &sha, "Changed lib/a.rs.".into())
            .await
            .unwrap();
        assert_eq!(reported.state, AttemptState::Committed);
    }

    async fn finish(self) {
        std::fs::write(self.root.join("release"), b"").unwrap();
        self.sessions.close_all().await.unwrap();
        self.storage.close().await.unwrap();
    }
}

#[tokio::test]
async fn brief_stays_fixed_after_layout_change() {
    let f = fixture().await;
    let attempt = f.start("s1", 1).await.unwrap();
    let first = f.core.runs().brief(&attempt.id).await.unwrap();
    let saved = f.saved_json(&attempt).await;
    assert_eq!(first.attempt.id, attempt.id);
    assert_eq!(first.context.task.number, 1);
    assert_eq!(first.context.plan.version, 1);
    let (map, additions) = (
        first.context.map_revision,
        first.context.standards.additions,
    );
    assert_eq!(additions, None);
    f.change_layout(1).await;
    // An attempt admitted now receives the new layout.
    let later = f.start("s2", 2).await.unwrap();
    let now = f.core.runs().brief(&later.id).await.unwrap().context;
    assert_eq!(now.map_revision, map + 1);
    assert_eq!(now.standards.additions, Some(1));
    let again = f.core.runs().brief(&attempt.id).await.unwrap();
    assert_eq!(again.context.map_revision, map);
    assert_eq!(again.context.standards.base, first.context.standards.base);
    assert_eq!(again.context.standards.additions, None);
    assert_eq!(again.captured_at, first.captured_at);
    assert_eq!(
        f.saved_json(&attempt).await,
        saved,
        "the saved JSON never changes"
    );
    // A saved context is never rewritten in place.
    let rewrite = sqlx::query("UPDATE run_attempt_context SET context_json = '{}'")
        .execute(f.storage.reader())
        .await;
    assert!(rewrite.is_err(), "{rewrite:?}");
    f.finish().await;
}

#[tokio::test]
async fn an_addition_captures_its_own_brief() {
    let f = fixture().await;
    let attempt = f.start("s1", 1).await.unwrap();
    let first = f.core.runs().brief(&attempt.id).await.unwrap();
    f.commit_and_report(&attempt, "one\n").await;
    f.change_layout(1).await;
    let next = f
        .core
        .runs()
        .amend(&f.planner(), &f.workflow, 1, "Also cover the edge.".into())
        .await
        .unwrap();
    assert_ne!(next.id, attempt.id);
    assert_eq!(next.addition.as_deref(), Some("Also cover the edge."));
    let second = f.core.runs().brief(&next.id).await.unwrap();
    assert_eq!(second.attempt.id, next.id);
    assert_eq!(second.context.map_revision, first.context.map_revision + 1);
    assert_eq!(second.context.standards.additions, Some(1));
    let old = f.core.runs().brief(&attempt.id).await.unwrap();
    assert_eq!(old.context.map_revision, first.context.map_revision);
    assert_eq!(old.context.standards.additions, None);
    assert_eq!(f.written().await, (2, 2));
    f.finish().await;
}

#[tokio::test]
async fn brief_replay_creates_one_context() {
    let f = fixture().await;
    let attempt = f.start("s1", 1).await.unwrap();
    let saved = f.saved_json(&attempt).await;
    f.change_layout(1).await;
    assert_eq!(f.start("s1", 1).await.unwrap().id, attempt.id);
    assert_eq!(
        f.written().await,
        (1, 1),
        "a replay captures no second context"
    );
    assert_eq!(f.saved_json(&attempt).await, saved);
    f.finish().await;
}

#[tokio::test]
async fn context_failure_rolls_back_admission() {
    let f = fixture().await;
    sqlx::query(
        "CREATE TRIGGER refuse_context BEFORE INSERT ON run_attempt_context
         BEGIN SELECT RAISE(ABORT, 'no context'); END",
    )
    .execute(f.storage.reader())
    .await
    .unwrap();
    let refused = f.start("s1", 1).await;
    assert!(refused.is_err(), "{refused:?}");
    assert_eq!(f.written().await, (0, 0), "no attempt without its context");
    let threads: i64 =
        sqlx::query_scalar("SELECT count(*) FROM planning_thread WHERE kind = 'executor'")
            .fetch_one(f.storage.reader())
            .await
            .unwrap();
    assert_eq!(threads, 0, "nor its executor thread");
    sqlx::query("DROP TRIGGER refuse_context")
        .execute(f.storage.reader())
        .await
        .unwrap();
    let attempt = f.start("s1b", 1).await.unwrap();
    assert_eq!(attempt.state, AttemptState::Running);
    assert_eq!(f.written().await, (1, 1));
    f.finish().await;
}

#[tokio::test]
async fn historical_context_is_not_invented() {
    let f = fixture().await;
    let attempt = f.start("s1", 1).await.unwrap();
    sqlx::query("DELETE FROM run_attempt_context WHERE attempt_id = ?")
        .bind(attempt.id.as_str())
        .execute(f.storage.reader())
        .await
        .unwrap();
    let missing = f.core.runs().brief(&attempt.id).await;
    assert!(
        matches!(missing, Err(RunsError::ContextNotRecorded)),
        "{missing:?}"
    );
    assert_eq!(missing.unwrap_err().code(), "context_not_recorded");
    let own = f.core.runs().brief_for(&f.executor(&attempt)).await;
    assert!(matches!(own, Err(RunsError::ContextNotRecorded)), "{own:?}");
    assert_eq!(f.written().await, (0, 1), "reading invents nothing");
    let unknown = f
        .core
        .runs()
        .brief(&shadows_core::AttemptId::generate())
        .await;
    assert!(matches!(unknown, Err(RunsError::NotFound)), "{unknown:?}");
    // An addition to such an attempt captures its own context normally.
    f.commit_and_report(&attempt, "one\n").await;
    let next = f
        .core
        .runs()
        .amend(&f.planner(), &f.workflow, 1, "More.".into())
        .await
        .unwrap();
    assert_eq!(
        f.core.runs().brief(&next.id).await.unwrap().attempt.id,
        next.id
    );
    assert!(matches!(
        f.core.runs().brief(&attempt.id).await,
        Err(RunsError::ContextNotRecorded)
    ));
    f.finish().await;
}

#[tokio::test]
async fn an_executor_reads_only_its_own_brief() {
    let f = fixture().await;
    let one = f.start("s1", 1).await.unwrap();
    let two = f.start("s2", 2).await.unwrap();
    let runs = f.core.runs();
    let own = runs.brief_for(&f.executor(&one)).await.unwrap();
    assert_eq!(own.attempt.id, one.id);
    assert_eq!(own.context.task.number, 1);
    let other = runs.brief_for(&f.executor(&two)).await.unwrap();
    assert_eq!(other.attempt.id, two.id);
    assert_eq!(other.context.task.number, 2);
    // A grant naming another task than its thread's attempt reads nothing.
    let mut crossed = f.executor(&one);
    crossed.scope.as_mut().unwrap().task = 2;
    let refused = runs.brief_for(&crossed).await;
    assert!(matches!(refused, Err(RunsError::NotFound)), "{refused:?}");
    // The Planner's grant is no executor's.
    let planner = runs.brief_for(&f.planner()).await;
    assert!(matches!(planner, Err(RunsError::NotFound)), "{planner:?}");
    f.finish().await;
}
