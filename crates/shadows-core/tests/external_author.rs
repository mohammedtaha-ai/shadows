//! An attempt of a plan an external agent wrote keeps its report for that author: one row,
//! addressed to the grant that wrote the attempt's own version, and no thread notice.

#[allow(dead_code)]
#[path = "fixtures/cross_plan.rs"]
mod cross_plan;

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use core_runs::{AttemptId, AttemptState};
use cross_plan::{SOURCE, command};
use serde_json::{Value, json};
use shadows_core::testing::{CommandContext, Harness, LiveHandles, ProjectDirectory, Storage};
use shadows_core::testing::{TaskScope, acp};
use shadows_core::{AcceptanceItem, AppCore, CoreParts, Grant, PlanOp, TaskContent};
use shadows_core::{ThreadId, WorkflowId, Writer};

struct Fixture {
    core: Arc<AppCore>,
    storage: Arc<Storage>,
    sessions: Arc<Harness>,
    workflow: WorkflowId,
    root: PathBuf,
    project: shadows_core::ProjectId,
    planner: ThreadId,
    /// Connection A, which wrote v1 when the plan is external.
    a: Grant,
    /// Connection B, another external agent of the same project.
    b: Grant,
    _tmp: tempfile::TempDir,
}

/// One report row as kept.
#[derive(Debug)]
struct Report {
    report_id: String,
    attempt_id: String,
    kind: String,
    grant_id: String,
    body: String,
    acked: bool,
}

fn git(root: &Path, args: &[&str]) -> String {
    let out = std::process::Command::new("git")
        .current_dir(root)
        .args(args)
        .output()
        .unwrap();
    assert!(out.status.success(), "git {args:?}: {out:?}");
    String::from_utf8(out.stdout).unwrap()
}

fn cmd(args: &[&str]) -> Vec<String> {
    args.iter().map(|s| s.to_string()).collect()
}

/// T1 writes `backend/app/**`; its contract carries `checks` when there are any.
fn task_one(checks: &[Vec<String>]) -> PlanOp {
    let contract = (!checks.is_empty()).then(|| {
        let checks: Vec<_> = checks
            .iter()
            .enumerate()
            .map(|(i, run)| json!({"id": format!("c{}", i + 1), "run": run, "timeout_secs": 10}))
            .collect();
        serde_json::from_value(json!({
            "files": [{"path": "backend/app/a.php", "action": "create", "purpose": "Count"}],
            "checks": checks
        }))
        .unwrap()
    });
    PlanOp::TaskAdd {
        task: TaskContent {
            phases: Vec::new(),
            contract,
            number: 1,
            title: "Task 1".into(),
            goal: "A usable result".into(),
            reads: vec![],
            writes: vec!["backend/app/**".into()],
            acceptance: vec![AcceptanceItem {
                number: 1,
                text: "The result works".into(),
            }],
        },
    }
}

/// An approved plan of one task; `external` makes connection A its v1's author, otherwise
/// the Planner thread wrote it.
async fn fixture(external: bool, checks: &[Vec<String>]) -> Fixture {
    let tmp = tempfile::tempdir().unwrap();
    let storage = Arc::new(Storage::open(&tmp.path().join("s.sqlite3")).await.unwrap());
    let root = tmp.path().join("project");
    std::fs::create_dir(&root).unwrap();
    git(&root, &["init", "-q"]);
    git(&root, &["config", "user.email", "runs@example.invalid"]);
    git(&root, &["config", "user.name", "Runs"]);
    git(&root, &["config", "core.autocrlf", "false"]);
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
    let planner = shadows_core::Threads::create_planning_thread_for_test(
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
    .bind(planner.id.as_str())
    .execute(storage.reader())
    .await
    .unwrap();
    cross_plan::approve_roadmap(&storage, project.id.as_str(), &["source-plan"]).await;
    let (runtime, _) = AppCore::runtime_for_tests(storage.clone()).await.unwrap();
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
    let mut grants = Vec::new();
    for name in ["connection-a", "connection-b"] {
        let issued = core
            .grants()
            .issue_external(name.into(), None, vec![project.id.clone()])
            .await
            .unwrap();
        grants.push(
            core.grants()
                .external_project(&issued.connection.id, &project.id)
                .await
                .unwrap(),
        );
    }
    let b = grants.pop().unwrap();
    let a = grants.pop().unwrap();
    let workflow = WorkflowId::from_literal(SOURCE);
    core.plans()
        .edit_plan_for_test(
            &command("tasks"),
            &Writer::Person,
            None,
            &workflow,
            0,
            &[task_one(checks)],
        )
        .await
        .unwrap();
    if external {
        sqlx::query(
            "UPDATE workflow SET written_by_thread = NULL, written_by_operation = NULL,
               written_by_grant = ? WHERE id = ?",
        )
        .bind(a.id.as_str())
        .bind(SOURCE)
        .execute(storage.reader())
        .await
        .unwrap();
    }
    core.plans()
        .approve_plan_for_test(&command("approve"), &workflow, 1)
        .await
        .unwrap();
    core.runs()
        .choose_executor_for_test(&workflow, 1, "claude-code", "fake-large", Some("high"))
        .await
        .unwrap();
    Fixture {
        core,
        storage,
        sessions,
        workflow,
        root,
        project: project.id,
        planner: planner.id,
        a,
        b,
        _tmp: tmp,
    }
}

impl Fixture {
    fn head(&self) -> String {
        git(&self.root, &["rev-parse", "HEAD"]).trim().to_string()
    }

    /// T1 running on an executor thread, as Start leaves it, with its context captured.
    async fn begin(&self) -> (AttemptId, Grant) {
        let (project, workflow, head) = (self.project.clone(), self.workflow.clone(), self.head());
        let grant = self
            .storage
            .write_txn(move |conn| {
                Box::pin(async move {
                    let thread = shadows_core::Threads::create_executor_in(
                        conn,
                        &project,
                        "T1".into(),
                        "claude-code",
                    )
                    .await?;
                    sqlx::query(
                        "INSERT INTO run_attempts
                           (id, project_id, workflow_id, version, task, number, thread_id,
                            model, state, writes_json, started_at, command_id, fingerprint,
                            start_head)
                         VALUES ('attempt-1', ?, ?, 1, 1, 1, ?, 'fake-large', 'running',
                                 '[\"backend/app/**\"]', '2026-10-05T00:00:00Z', 'attempt-1',
                                 'f', ?)",
                    )
                    .bind(project.as_str())
                    .bind(workflow.as_str())
                    .bind(thread.as_str())
                    .bind(&head)
                    .execute(&mut *conn)
                    .await?;
                    let scope = TaskScope {
                        workflow: workflow.as_str().to_string(),
                        version: 1,
                        task: 1,
                    };
                    let (grant, _) = shadows_core::Grants::issue_executor_grant_in(
                        conn, &project, &thread, scope,
                    )
                    .await?;
                    Ok(grant)
                })
            })
            .await
            .unwrap();
        let context = self
            .core
            .plans()
            .task_context(&grant, Some(&self.workflow), 1, None)
            .await
            .unwrap();
        sqlx::query(
            "INSERT INTO run_attempt_context (attempt_id, context_json, captured_at)
             VALUES ('attempt-1', ?, '2026-11-15T00:00:00Z')",
        )
        .bind(serde_json::to_string(&context).unwrap())
        .execute(self.storage.reader())
        .await
        .unwrap();
        (AttemptId::from_recorded("attempt-1".into()), grant)
    }

    /// The executor commits and reports; recovery then confirms the committed attempt.
    async fn commit_and_report(&self, grant: &Grant, checks: usize) {
        let path = self.root.join("backend/app/a.php");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, "one\n").unwrap();
        git(&self.root, &["add", "--", "backend/app/a.php"]);
        git(&self.root, &["commit", "-q", "-m", "build"]);
        let reports: Vec<_> = (1..=checks)
            .map(|i| json!({"id": format!("c{i}"), "outcome": "passed"}))
            .collect();
        self.core
            .runs()
            .report(
                grant,
                &self.head(),
                "Built.".into(),
                serde_json::from_value(json!(reports)).unwrap(),
            )
            .await
            .unwrap();
        self.core.runs().recover().await.unwrap();
    }

    async fn reports(&self) -> Vec<Report> {
        let rows: Vec<(String, String, String, String, String, bool)> = sqlx::query_as(
            "SELECT report_id, attempt_id, kind, grant_id, body, acked_at IS NOT NULL
             FROM run_author_report ORDER BY report_id",
        )
        .fetch_all(self.storage.reader())
        .await
        .unwrap();
        rows.into_iter()
            .map(
                |(report_id, attempt_id, kind, grant_id, body, acked)| Report {
                    report_id,
                    attempt_id,
                    kind,
                    grant_id,
                    body,
                    acked,
                },
            )
            .collect()
    }

    /// Waits for the first kept report.
    async fn reported(&self) -> Vec<Report> {
        for _ in 0..800 {
            let reports = self.reports().await;
            if !reports.is_empty() {
                return reports;
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
        panic!("no report was kept");
    }

    async fn planner_entries(&self) -> i64 {
        sqlx::query_scalar("SELECT COUNT(*) FROM thread_entry WHERE thread_id = ?")
            .bind(self.planner.as_str())
            .fetch_one(self.storage.reader())
            .await
            .unwrap()
    }

    async fn ended(&self, attempt: &AttemptId) {
        for _ in 0..400 {
            let now = self.core.runs().attempt(attempt).await.unwrap();
            if now.state != AttemptState::Running {
                return;
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
        panic!("the attempt stayed running");
    }

    async fn finish(self) {
        std::fs::write(self.root.join("release"), b"").unwrap();
        self.sessions.close_all().await.unwrap();
        self.storage.close().await.unwrap();
    }
}

#[tokio::test]
async fn an_ended_attempt_of_an_external_plan_is_kept_for_its_author() {
    let f = fixture(true, &[]).await;
    let attempt = f
        .core
        .runs()
        .start("s1".into(), &f.workflow, 1, 1)
        .await
        .unwrap();
    let reports = f.reported().await;
    assert_eq!(reports.len(), 1, "{reports:?}");
    let report = &reports[0];
    assert_eq!(report.report_id, format!("{}:ended", attempt.id));
    assert_eq!(report.attempt_id, attempt.id.as_str());
    assert_eq!(report.kind, "ended");
    assert_eq!(report.grant_id, f.a.id.as_str());
    assert!(!report.acked);
    assert!(
        report.body.contains("failed: ended without a commit"),
        "{}",
        report.body
    );
    assert_eq!(f.planner_entries().await, 0);
    f.finish().await;
}

#[tokio::test]
async fn the_report_is_kept_only_after_the_checks_ran() {
    let slow = cmd(&["git", "-c", "alias.pause=!sleep 2", "pause"]);
    let f = fixture(true, std::slice::from_ref(&slow)).await;
    let (id, grant) = f.begin().await;
    f.commit_and_report(&grant, 1).await;
    for _ in 0..600 {
        let rows = f.core.runs().check_results(&id).await.unwrap();
        if rows.first().is_some_and(|r| r.started_at.is_some()) {
            break;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    let rows = f.core.runs().check_results(&id).await.unwrap();
    assert!(rows[0].finished_at.is_none(), "the check ended too soon");
    assert!(f.reports().await.is_empty());
    let reports = f.reported().await;
    assert_eq!(reports.len(), 1, "{reports:?}");
    assert!(reports[0].body.contains("c1 passed"), "{}", reports[0].body);
    assert!(
        f.core.runs().check_results(&id).await.unwrap()[0]
            .finished_at
            .is_some()
    );
    f.finish().await;
}

#[tokio::test]
async fn a_planner_plan_keeps_its_thread_notice() {
    let f = fixture(false, &[]).await;
    let (id, grant) = f.begin().await;
    f.commit_and_report(&grant, 0).await;
    for _ in 0..200 {
        if !f.core.runs().check_results(&id).await.unwrap().is_empty() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    tokio::time::sleep(Duration::from_millis(500)).await;
    assert!(f.reports().await.is_empty());
    f.finish().await;
}

#[tokio::test]
async fn a_repeated_end_keeps_one_report() {
    let f = fixture(true, &[]).await;
    let (_, grant) = f.begin().await;
    f.commit_and_report(&grant, 0).await;
    let first = f.reported().await;
    sqlx::query("UPDATE run_attempts SET notice_sent_at = NULL")
        .execute(f.storage.reader())
        .await
        .unwrap();
    f.core.runs().recover().await.unwrap();
    tokio::time::sleep(Duration::from_millis(500)).await;
    let again = f.reports().await;
    assert_eq!(again.len(), 1, "{again:?}");
    assert_eq!(again[0].body, first[0].body);
    f.finish().await;
}

#[tokio::test]
async fn the_report_goes_to_the_author_of_the_attempts_own_version() {
    let f = fixture(true, &[]).await;
    let (_, grant) = f.begin().await;
    sqlx::query(
        "INSERT INTO workflow
           (id, plan_id, state, version, revision, title, goal, written_by_grant,
            created_at, updated_at)
         VALUES ('v2-by-b', 'source-plan', 'Draft', 2, 0, 'Web', 'A usable web app', ?,
           '2026-10-06T00:00:00Z', '2026-10-06T00:00:00Z')",
    )
    .bind(f.b.id.as_str())
    .execute(f.storage.reader())
    .await
    .unwrap();
    f.commit_and_report(&grant, 0).await;
    let reports = f.reported().await;
    assert_eq!(reports.len(), 1, "{reports:?}");
    assert_eq!(reports[0].grant_id, f.a.id.as_str());
    f.finish().await;
}

#[tokio::test]
async fn a_check_returned_to_the_executor_reports_once_its_follow_up_settles() {
    let failing = cmd(&["git", "rev-parse", "no-such-ref"]);
    let f = fixture(true, std::slice::from_ref(&failing)).await;
    let (id, grant) = f.begin().await;
    f.commit_and_report(&grant, 1).await;
    let follow_up = loop_follow_up(&f).await;
    f.ended(&follow_up).await;
    let reports = f.reported().await;
    tokio::time::sleep(Duration::from_millis(500)).await;
    let reports_now = f.reports().await;
    assert_eq!(reports_now.len(), 1, "{reports_now:?}");
    assert_eq!(reports[0].kind, "ended");
    assert_eq!(reports[0].attempt_id, follow_up.as_str());
    assert_ne!(reports[0].attempt_id, id.as_str());
    f.finish().await;
}

/// The attempt the failed checks returned T1 to.
async fn loop_follow_up(f: &Fixture) -> AttemptId {
    for _ in 0..400 {
        let attempts = f.core.runs().attempts(&f.workflow, 1, 1).await.unwrap();
        if attempts.len() > 1 {
            assert_eq!(attempts[0].origin, core_runs::AttemptOrigin::Checks);
            return attempts[0].id.clone();
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    panic!("the failed checks never returned to the executor");
}
