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
            "SELECT report_id, COALESCE(attempt_id, ''), kind, grant_id, body, acked_at IS NOT NULL
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
    assert_eq!(report.report_id, format!("ended:{}", attempt.id));
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

use core_runs::{AuthorWait, RunsError, WaitOutcome};

impl Fixture {
    async fn wait(&self, grant: &Grant, ack: &[&str], bound_ms: u64) -> AuthorWait {
        let ack: Vec<String> = ack.iter().map(|s| s.to_string()).collect();
        self.core
            .runs()
            .author_wait(grant, &ack, Duration::from_millis(bound_ms))
            .await
            .unwrap()
    }

    /// Spawns connection A's wait, bound by `bound_ms`.
    fn spawn_wait(&self, bound_ms: u64) -> tokio::task::JoinHandle<Result<AuthorWait, RunsError>> {
        let (core, grant) = (self.core.clone(), self.a.clone());
        tokio::spawn(async move {
            core.runs()
                .author_wait(&grant, &[], Duration::from_millis(bound_ms))
                .await
        })
    }

    /// Sets every report's claim to have lapsed long ago, or to lapse in a second.
    async fn set_claim(&self, lapsed: bool) {
        let sql = if lapsed {
            "UPDATE run_author_report SET claimed_until = '2000-01-01T00:00:00.000000000Z'"
        } else {
            "UPDATE run_author_report
             SET claimed_until = strftime('%Y-%m-%dT%H:%M:%f', 'now', '+1 seconds') || '000000Z'"
        };
        sqlx::query(sql)
            .execute(self.storage.reader())
            .await
            .unwrap();
    }

    async fn claim_open(&self) -> bool {
        sqlx::query_scalar(
            "SELECT COUNT(*) = 1 FROM run_author_report
             WHERE claimed_until IS NULL AND acked_at IS NULL",
        )
        .fetch_one(self.storage.reader())
        .await
        .unwrap()
    }

    /// A's attempt started on T1, ended at once, and its report kept.
    async fn one_report(&self) -> AttemptId {
        let attempt = self
            .core
            .runs()
            .start("s1".into(), &self.workflow, 1, 1)
            .await
            .unwrap();
        self.reported().await;
        attempt.id
    }
}

#[tokio::test]
async fn a_wait_answers_at_once_with_reports_left_while_no_one_waited() {
    let f = fixture(true, &[]).await;
    let attempt = f.one_report().await;
    let began = std::time::Instant::now();
    let wait = f.wait(&f.a, &[], 20_000).await;
    assert!(began.elapsed() < Duration::from_secs(2));
    assert_eq!(wait.outcome, WaitOutcome::Ended);
    assert_eq!(wait.reports.len(), 1, "{wait:?}");
    let report = &wait.reports[0];
    assert_eq!(report.report_id, format!("ended:{attempt}"));
    assert_eq!(report.attempt_id, Some(attempt));
    assert_eq!(report.workflow_id, Some(f.workflow.clone()));
    assert_eq!(report.task, Some(1));
    assert_eq!(report.kind, "ended");
    assert!(report.report.contains("failed: ended without a commit"));
    f.finish().await;
}

#[tokio::test]
async fn a_wait_with_nothing_pending_answers_none_running() {
    let f = fixture(true, &[]).await;
    let began = std::time::Instant::now();
    let wait = f.wait(&f.a, &[], 20_000).await;
    assert!(began.elapsed() < Duration::from_secs(2));
    assert_eq!(wait.outcome, WaitOutcome::NoneRunning);
    assert!(wait.reports.is_empty());
    f.finish().await;
}

#[tokio::test]
async fn a_wait_during_slow_checks_does_not_answer_none_running() {
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
    let wait = f.wait(&f.a, &[], 120_000).await;
    assert_eq!(wait.outcome, WaitOutcome::Ended, "{wait:?}");
    assert!(wait.reports[0].report.contains("c1 passed"));
    f.finish().await;
}

#[tokio::test]
async fn a_wait_ends_when_an_attempt_ends() {
    let f = fixture(true, &[]).await;
    let (id, grant) = f.begin().await;
    let waiting = f.spawn_wait(120_000);
    tokio::time::sleep(Duration::from_millis(300)).await;
    assert!(!waiting.is_finished());
    f.commit_and_report(&grant, 0).await;
    let wait = waiting.await.unwrap().unwrap();
    assert_eq!(wait.outcome, WaitOutcome::Ended);
    assert_eq!(wait.reports[0].attempt_id, Some(id));
    f.finish().await;
}

#[tokio::test]
async fn a_wait_passes_its_bound_with_nothing_yet() {
    let f = fixture(true, &[]).await;
    f.begin().await;
    let began = std::time::Instant::now();
    let wait = f.wait(&f.a, &[], 500).await;
    assert!(began.elapsed() >= Duration::from_millis(500));
    assert_eq!(wait.outcome, WaitOutcome::NothingYet);
    assert!(wait.reports.is_empty());
    f.finish().await;
}

#[tokio::test]
async fn two_waiters_never_hold_one_report_at_once() {
    let f = fixture(true, &[]).await;
    f.one_report().await;
    let (one, two) = tokio::join!(f.wait(&f.a, &[], 500), f.wait(&f.a, &[], 500));
    let ended = [&one, &two]
        .iter()
        .filter(|w| w.outcome == WaitOutcome::Ended)
        .count();
    let nothing = [&one, &two]
        .iter()
        .filter(|w| w.outcome == WaitOutcome::NothingYet)
        .count();
    assert_eq!((ended, nothing), (1, 1), "{one:?} {two:?}");
    assert_eq!(one.reports.len() + two.reports.len(), 1);
    f.finish().await;
}

#[tokio::test]
async fn an_unacknowledged_report_is_answered_again_after_its_lease() {
    let f = fixture(true, &[]).await;
    f.one_report().await;
    let first = f.wait(&f.a, &[], 500).await;
    f.set_claim(true).await;
    let again = f.wait(&f.a, &[], 500).await;
    assert_eq!(again.outcome, WaitOutcome::Ended);
    assert_eq!(again.reports[0].report_id, first.reports[0].report_id);
    f.finish().await;
}

#[tokio::test]
async fn an_acknowledged_report_is_not_answered_again() {
    let f = fixture(true, &[]).await;
    f.one_report().await;
    let first = f.wait(&f.a, &[], 500).await;
    let id = first.reports[0].report_id.clone();
    let next = f.wait(&f.a, &[&id], 500).await;
    assert_eq!(next.outcome, WaitOutcome::NoneRunning);
    assert!(next.reports.is_empty());
    f.set_claim(true).await;
    let later = f.wait(&f.a, &[], 500).await;
    assert_eq!(later.outcome, WaitOutcome::NoneRunning);
    assert!(f.reports().await[0].acked);
    f.finish().await;
}

#[tokio::test]
async fn an_ack_for_another_grants_report_is_ignored() {
    let f = fixture(true, &[]).await;
    f.one_report().await;
    let first = f.wait(&f.a, &[], 500).await;
    let id = first.reports[0].report_id.clone();
    let other = f.wait(&f.b, &[&id], 500).await;
    assert_eq!(other.outcome, WaitOutcome::NoneRunning);
    assert!(!f.reports().await[0].acked);
    f.finish().await;
}

#[tokio::test]
async fn a_wait_after_a_returned_check_answers_none_running_once_acknowledged() {
    let failing = cmd(&["git", "rev-parse", "no-such-ref"]);
    let f = fixture(true, std::slice::from_ref(&failing)).await;
    let (_, grant) = f.begin().await;
    f.commit_and_report(&grant, 1).await;
    let follow_up = loop_follow_up(&f).await;
    f.ended(&follow_up).await;
    let wait = f.wait(&f.a, &[], 120_000).await;
    assert_eq!(wait.outcome, WaitOutcome::Ended, "{wait:?}");
    assert_eq!(wait.reports.len(), 1);
    assert_eq!(wait.reports[0].attempt_id.as_ref(), Some(&follow_up));
    let id = wait.reports[0].report_id.clone();
    let next = f.wait(&f.a, &[&id], 120_000).await;
    assert_eq!(next.outcome, WaitOutcome::NoneRunning, "{next:?}");
    f.finish().await;
}

#[tokio::test]
async fn a_follow_up_that_fails_to_open_reports_to_its_external_author() {
    let failing = cmd(&["git", "rev-parse", "no-such-ref"]);
    let f = fixture(true, std::slice::from_ref(&failing)).await;
    let (id, grant) = f.begin().await;
    // A model that refuses the executor's mode: the follow-up's session cannot open.
    sqlx::query("UPDATE run_attempts SET model = 'fake-small' WHERE id = ?")
        .bind(id.as_str())
        .execute(f.storage.reader())
        .await
        .unwrap();
    f.commit_and_report(&grant, 1).await;
    let follow_up = loop_follow_up(&f).await;
    f.ended(&follow_up).await;
    let failed = f.core.runs().attempt(&follow_up).await.unwrap();
    assert_eq!(failed.state, AttemptState::Failed);
    let reason = failed.reason.as_deref().unwrap_or_default();
    assert!(reason.starts_with("session did not open"), "{reason}");
    let reports = f.reported().await;
    assert_eq!(reports.len(), 1, "{reports:?}");
    assert_eq!(reports[0].kind, "ended");
    assert_eq!(reports[0].attempt_id, follow_up.as_str());
    let wait = f.wait(&f.a, &[], 20_000).await;
    assert_eq!(wait.outcome, WaitOutcome::Ended, "{wait:?}");
    assert_eq!(wait.reports.len(), 1);
    let report = wait.reports[0].report_id.clone();
    let next = f.wait(&f.a, &[&report], 20_000).await;
    assert_eq!(next.outcome, WaitOutcome::NoneRunning, "{next:?}");
    f.finish().await;
}

#[tokio::test]
async fn a_start_that_fails_to_open_does_not_hold_the_wait() {
    let f = fixture(true, &[]).await;
    // A model the adapter refuses to switch to: Start's session cannot open.
    sqlx::query("UPDATE run_task_executors SET model = 'fake-locked'")
        .execute(f.storage.reader())
        .await
        .unwrap();
    let attempt = f
        .core
        .runs()
        .start("s1".into(), &f.workflow, 1, 1)
        .await
        .unwrap();
    assert_eq!(attempt.state, AttemptState::Failed);
    let reason = attempt.reason.as_deref().unwrap_or_default();
    assert!(reason.starts_with("session did not open"), "{reason}");
    let wait = f.wait(&f.a, &[], 3_000).await;
    assert_eq!(wait.outcome, WaitOutcome::NoneRunning, "{wait:?}");
    assert!(f.reports().await.is_empty());
    f.finish().await;
}

#[tokio::test]
async fn an_attempt_its_author_stops_does_not_hold_the_wait() {
    let f = fixture(true, &[]).await;
    let (id, _) = f.begin().await;
    let stopped = f
        .core
        .runs()
        .stop_for(&f.a, "stop-1".into(), &id)
        .await
        .unwrap();
    assert_eq!(stopped.state, AttemptState::Stopped);
    let began = std::time::Instant::now();
    let wait = f.wait(&f.a, &[], 3_000).await;
    assert!(began.elapsed() < Duration::from_secs(2));
    assert_eq!(wait.outcome, WaitOutcome::NoneRunning, "{wait:?}");
    assert!(f.reports().await.is_empty());
    f.finish().await;
}

#[tokio::test]
async fn a_waiting_call_wakes_when_a_claim_lapses() {
    let f = fixture(true, &[]).await;
    f.one_report().await;
    let first = f.wait(&f.a, &[], 500).await;
    f.set_claim(false).await;
    let began = std::time::Instant::now();
    let wait = f.spawn_wait(20_000).await.unwrap().unwrap();
    let took = began.elapsed();
    assert!(took >= Duration::from_millis(700), "{took:?}");
    assert!(took < Duration::from_secs(10), "{took:?}");
    assert_eq!(wait.outcome, WaitOutcome::Ended);
    assert_eq!(wait.reports[0].report_id, first.reports[0].report_id);
    assert_eq!(f.reports().await.len(), 1);
    f.finish().await;
}

#[tokio::test]
async fn a_grant_revoked_during_the_wait_is_refused_and_its_report_stays_open() {
    let f = fixture(true, &[]).await;
    let (_, grant) = f.begin().await;
    let waiting = f.spawn_wait(120_000);
    tokio::time::sleep(Duration::from_millis(300)).await;
    let mut a = None;
    for connection in f.core.grants().list_external().await.unwrap() {
        let project = f.core.grants().external_project(&connection.id, &f.project);
        if project.await.unwrap().id == f.a.id {
            a = Some(connection);
        }
    }
    let a = a.unwrap();
    f.core
        .grants()
        .update_external_projects("remove".into(), &a.id, a.project_revision, vec![])
        .await
        .unwrap();
    f.commit_and_report(&grant, 0).await;
    let refused = waiting.await.unwrap();
    let invalid = shadows_core::GrantsError::GrantInvalid.to_string();
    assert!(
        matches!(&refused, Err(RunsError::Grants(e)) if e.to_string() == invalid),
        "{refused:?}"
    );
    f.reported().await;
    assert!(f.claim_open().await);
    f.finish().await;
}

impl Fixture {
    async fn author_waiting(&self) -> bool {
        let run = self.core.runs().task(&self.workflow, 1, 1).await.unwrap();
        run.author_waiting
    }

    /// The TaskProgressChanged payloads journaled at `at`.
    async fn progress_at(&self, at: &str) -> Vec<serde_json::Value> {
        let rows: Vec<String> = sqlx::query_scalar(
            "SELECT payload_json FROM durable_event
             WHERE kind = 'TaskProgressChanged' AND created_at = ? ORDER BY seq",
        )
        .bind(at)
        .fetch_all(self.storage.reader())
        .await
        .unwrap();
        rows.iter()
            .map(|p| serde_json::from_str(p).unwrap())
            .collect()
    }
}

#[tokio::test]
async fn a_task_run_shows_its_result_waiting_for_its_author() {
    let f = fixture(true, &[]).await;
    assert!(!f.author_waiting().await);
    let attempt = f.one_report().await;
    assert!(f.author_waiting().await);
    let wait = f.wait(&f.a, &[], 2_000).await;
    assert_eq!(wait.reports.len(), 1);
    assert!(f.author_waiting().await, "a claimed report still waits");
    f.wait(&f.a, &[&format!("ended:{attempt}")], 50).await;
    assert!(!f.author_waiting().await);
    f.finish().await;
}

#[tokio::test]
async fn saving_and_acknowledging_a_report_journal_the_tasks_progress() {
    let f = fixture(true, &[]).await;
    let attempt = f.one_report().await;
    let report_id = format!("ended:{attempt}");
    f.wait(&f.a, &[], 2_000).await;
    f.wait(&f.a, &[&report_id], 50).await;
    let (created, acked): (String, String) =
        sqlx::query_as("SELECT created_at, acked_at FROM run_author_report WHERE report_id = ?")
            .bind(&report_id)
            .fetch_one(f.storage.reader())
            .await
            .unwrap();
    let expected = json!({
        "project_id": f.project.as_str(), "workflow_id": f.workflow.as_str(),
        "version": 1, "task": 1, "attempt_id": attempt.as_str(),
    });
    assert_eq!(f.progress_at(&created).await, vec![expected.clone()]);
    assert_eq!(f.progress_at(&acked).await, vec![expected]);
    f.finish().await;
}

#[tokio::test]
async fn report_ids_name_their_kind_and_source() {
    let f = fixture(true, &[]).await;
    let ended = f.one_report().await;
    assert_eq!(
        f.reports().await[0].report_id,
        format!("ended:{ended}"),
        "an ended report is named by its attempt"
    );
    f.finish().await;

    let f = fixture(true, &[]).await;
    let (attempt, grant) = f.begin().await;
    let input = shadows_core::ConflictInput {
        command_id: "c1".into(),
        expected: "the backend answers by id".into(),
        observed: "it has no id".into(),
        evidence: vec![shadows_core::ConflictEvidence {
            path: "README.md".into(),
            line: Some(1),
            summary: "no id here".into(),
        }],
    };
    f.core.runs().conflict_for(&grant, input).await.unwrap();
    f.core.runs().recover().await.unwrap();
    let reports = f.reported().await;
    assert_eq!(reports.len(), 1, "{reports:?}");
    assert_eq!(reports[0].kind, "conflict");
    assert_eq!(reports[0].report_id, format!("conflict:{attempt}"));
    f.finish().await;
}

impl Fixture {
    /// A question to the attempt's executor, in `state`, with `reply` when it was answered.
    async fn ask(&self, attempt: &AttemptId, state: &str, reply: Option<&str>) {
        let thread = self.core.runs().attempt(attempt).await.unwrap().thread_id;
        sqlx::query(
            "INSERT INTO run_agent_messages
               (id, attempt_id, thread_id, ordinal, message, state, reply, created_at)
             VALUES ('message-1', ?, ?, 1, 'Why?', ?, ?, '2026-11-28T00:00:00Z')",
        )
        .bind(attempt.as_str())
        .bind(thread.as_str())
        .bind(state)
        .bind(reply)
        .execute(self.storage.reader())
        .await
        .unwrap();
    }

    async fn replies(&self) -> Vec<Report> {
        let mut reports = self.reports().await;
        reports.retain(|r| r.kind == "reply");
        reports
    }
}

#[tokio::test]
async fn an_executors_reply_reaches_its_external_author() {
    let f = fixture(true, &[]).await;
    let (id, grant) = f.begin().await;
    f.commit_and_report(&grant, 0).await;
    f.reported().await;
    f.ask(&id, "replied", Some("Because.")).await;
    f.core.runs().recover().await.unwrap();
    let mut replies = Vec::new();
    for _ in 0..400 {
        replies = f.replies().await;
        if !replies.is_empty() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    assert_eq!(replies.len(), 1, "{replies:?}");
    assert_eq!(replies[0].report_id, "reply:message-1");
    assert_eq!(replies[0].grant_id, f.a.id.as_str());
    assert_eq!(
        replies[0].body,
        "[Shadows] Reply from task agent · T1 \"Task 1\":\nBecause."
    );
    let (state, delivered): (String, bool) = sqlx::query_as(
        "SELECT state, delivered_at IS NOT NULL FROM run_agent_messages WHERE id = 'message-1'",
    )
    .fetch_one(f.storage.reader())
    .await
    .unwrap();
    assert_eq!((state.as_str(), delivered), ("delivered", true));
    f.core.runs().recover().await.unwrap();
    tokio::time::sleep(Duration::from_millis(500)).await;
    assert_eq!(f.replies().await.len(), 1);
    assert_eq!(f.planner_entries().await, 0);
    f.finish().await;
}

#[tokio::test]
async fn a_wait_holds_while_a_message_awaits_its_reply() {
    let f = fixture(true, &[]).await;
    let (id, grant) = f.begin().await;
    f.commit_and_report(&grant, 0).await;
    let ended = f.wait(&f.a, &[], 20_000).await;
    assert_eq!(ended.outcome, WaitOutcome::Ended);
    let ack = ended.reports[0].report_id.clone();
    f.ask(&id, "sent", None).await;
    let held = f.wait(&f.a, &[&ack], 300).await;
    assert_eq!(held.outcome, WaitOutcome::NothingYet, "{held:?}");
    sqlx::query(
        "UPDATE run_agent_messages SET state = 'failed', reason = 'no reply',
           delivered_at = '2026-11-28T00:00:01Z'",
    )
    .execute(f.storage.reader())
    .await
    .unwrap();
    let free = f.wait(&f.a, &[], 20_000).await;
    assert_eq!(free.outcome, WaitOutcome::NoneRunning, "{free:?}");
    f.finish().await;
}
