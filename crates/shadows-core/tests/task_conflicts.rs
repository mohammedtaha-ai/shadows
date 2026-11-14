//! Contract conflicts: an executor reports that its task cannot be built as
//! its contract says. The conflict is recorded on its own attempt, the
//! Planner hears it once while the attempt may still run, the existing
//! watcher settles the attempt only when its turn has ended, and the Planner
//! stops affected attempts explicitly. A held scope is never freed while the
//! executor may write. The executor runs on `fake_acp`: `hang` holds its turn
//! until it is stopped, `wait-for-release` until a `release` file appears.

#[allow(dead_code)]
#[path = "fixtures/cross_plan.rs"]
mod cross_plan;

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use core_runs::{Attempt, AttemptState, RunsError};
use cross_plan::{SOURCE, command};
use serde_json::Value;
use shadows_core::testing::acp;
use shadows_core::testing::{CommandContext, Harness, LiveHandles, ProjectDirectory, Storage};
use shadows_core::{
    AcceptanceItem, AppCore, ConflictDelivery, ConflictEvidence, ConflictInput, CoreParts, Grant,
    GrantId, GrantKind, OperationId, PlanOp, QueuedMessage, TaskConflict, TaskContent, ThreadId,
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
    std::fs::write(root.join(".gitignore"), "release\n").unwrap();
    std::fs::create_dir_all(root.join("lib")).unwrap();
    std::fs::write(root.join("lib/find.rs"), "pub fn find() {}\n").unwrap();
    git(root, &["add", ".gitignore", "lib/find.rs"]);
    git(root, &["commit", "-q", "-m", "start"]);
}

/// T1 `lib/**`, T2 `web/**`, T3 `docs/**`, approved, written by a Planner.
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
    let ops = vec![task(1, "lib/**"), task(2, "web/**"), task(3, "docs/**")];
    plans
        .edit_plan_for_test(&command("tasks"), &Writer::Person, None, &workflow, 0, &ops)
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

fn input(id: &str) -> ConflictInput {
    ConflictInput {
        command_id: id.into(),
        expected: " find answers the customer by id ".into(),
        observed: "the customers table has no id column".into(),
        evidence: vec![
            ConflictEvidence {
                path: "./lib/find.rs".into(),
                line: Some(1),
                summary: "find takes no argument".into(),
            },
            ConflictEvidence {
                path: "lib/missing.rs".into(),
                line: None,
                summary: "the producer's file does not exist".into(),
            },
        ],
    }
}

impl Fixture {
    /// Starts `task` with its executor's first turn scripted as `prompt`.
    async fn start(&self, command: &str, task: u32, prompt: &str) -> Attempt {
        self.core.runs().set_first_prompt_for_tests(prompt);
        let attempt = self
            .core
            .runs()
            .start(command.into(), &self.workflow, 1, task, "fake-large".into())
            .await
            .unwrap();
        assert_eq!(attempt.state, AttemptState::Running, "{attempt:?}");
        attempt
    }

    /// A stored executor grant for `attempt`'s thread and task, as Harness
    /// issues one when the session opens (the fake session issues none).
    async fn executor(&self, attempt: &Attempt) -> Grant {
        let (project, thread) = (self.project.clone(), attempt.thread_id.clone());
        let scope = shadows_core::testing::TaskScope {
            workflow: SOURCE.into(),
            version: 1,
            task: attempt.task,
        };
        let (grant, _token) = self
            .storage
            .write_txn(move |conn| {
                Box::pin(async move {
                    shadows_core::Grants::issue_executor_grant_in(conn, &project, &thread, scope)
                        .await
                })
            })
            .await
            .unwrap();
        grant
    }

    async fn planner_grant(&self) -> Grant {
        shadows_core::Grants::issue_thread_grant_for_test(&self.storage, &self.planner)
            .await
            .unwrap()
            .0
    }

    /// One ended turn of the Planner asking for `model`: the settings a
    /// notice is sent with.
    async fn planner_turn(&self, id: &str, model: &str) {
        use shadows_core::testing::turn::{new_turn, turn_command};
        let runtime = shadows_core::testing::Runtime::start(self.storage.clone())
            .await
            .unwrap();
        let settings = shadows_agent::TurnSettings {
            model: model.into(),
            mode: "default".into(),
            effort: None,
        };
        let turn = new_turn(&self.planner, &runtime, "plan it", &settings);
        let started = shadows_core::Turns::start_turn_for_test(
            &self.storage,
            &turn_command(id, &self.planner, "plan it", &settings),
            turn,
        )
        .await
        .unwrap();
        shadows_core::Turns::mark_operation_failed_for_test(
            &self.storage,
            &started.operation_id,
            shadows_core::testing::FailureStage::Prepare,
            "recorded only",
        )
        .await
        .unwrap();
    }

    /// The Planner is in a turn: a notice waits in its queue.
    async fn busy_planner(&self) -> OperationId {
        let runtime = shadows_core::testing::Runtime::start(self.storage.clone())
            .await
            .unwrap();
        shadows_core::Turns::create_pending_operation_for_test(
            &self.storage,
            &self.planner,
            runtime.instance_id(),
        )
        .await
        .unwrap()
    }

    /// The executors' notices waiting on the Planner.
    async fn notices(&self) -> Vec<QueuedMessage> {
        let queued = self.core.turns().queued(&self.planner).await.unwrap();
        queued.into_iter().filter(|m| m.notice.is_some()).collect()
    }

    async fn held(&self, attempt: &Attempt) -> bool {
        let released: Option<String> =
            sqlx::query_scalar("SELECT released_at FROM run_attempts WHERE id = ?")
                .bind(attempt.id.as_str())
                .fetch_one(self.storage.reader())
                .await
                .unwrap();
        released.is_none()
    }

    async fn conflict(&self, attempt: &Attempt) -> TaskConflict {
        self.core
            .runs()
            .conflict(&attempt.id)
            .await
            .unwrap()
            .expect("the attempt has a conflict")
    }

    /// Waits until `attempt`'s conflict satisfies `done`.
    async fn conflict_until(
        &self,
        attempt: &Attempt,
        done: impl Fn(&TaskConflict) -> bool,
    ) -> TaskConflict {
        for _ in 0..400 {
            let now = self.conflict(attempt).await;
            if done(&now) {
                return now;
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
        panic!(
            "the conflict never settled: {:?}",
            self.conflict(attempt).await
        );
    }

    /// Commits `path` (inside some task's writes) and answers its sha.
    fn commit(&self, path: &str) -> String {
        let at = self.root.join(path);
        std::fs::create_dir_all(at.parent().unwrap()).unwrap();
        std::fs::write(&at, "pub fn work() {}\n").unwrap();
        git(&self.root, &["add", "--", path]);
        git(&self.root, &["commit", "-q", "-m", path]);
        git(&self.root, &["rev-parse", "HEAD"]).trim().to_string()
    }

    fn release(&self) {
        std::fs::write(self.root.join("release"), b"").unwrap();
    }

    async fn finish(self) {
        self.release();
        self.sessions.close_all().await.unwrap();
        self.storage.close().await.unwrap();
    }
}

/// A conflict and a commit report on one attempt: whichever is written first
/// wins, and the other is refused without changing it.
#[tokio::test]
async fn conflict_and_report_admit_one_outcome() {
    let f = fixture().await;
    let runs = f.core.runs();
    let first = f.start("s1", 1, "hang").await;
    let grant = f.executor(&first).await;
    let accepted = runs.conflict_for(&grant, input("c1")).await.unwrap();
    // The commit is valid; only the accepted conflict refuses it, in the
    // report's own write after every preflight check passed.
    let sha = f.commit("lib/one.rs");
    let late = runs.report(&grant, &sha, "Built it.".into()).await;
    assert!(matches!(late, Err(RunsError::ContractConflict)), "{late:?}");
    assert_eq!(late.unwrap_err().code(), "contract_conflict");
    let now = runs.attempt(&first.id).await.unwrap();
    assert_eq!((now.state, now.commit), (AttemptState::Running, None));
    // One conflict per attempt: another command is refused, the same one
    // replays, and the same id with other input is a command conflict.
    let other = runs.conflict_for(&grant, input("c2")).await;
    assert!(
        matches!(other, Err(RunsError::ContractConflict)),
        "{other:?}"
    );
    let again = runs.conflict_for(&grant, input("c1")).await.unwrap();
    assert_eq!(again.created_at, accepted.created_at);
    assert_eq!(again.expected, "find answers the customer by id");
    let mut changed = input("c1");
    changed.observed = "something else".into();
    let changed = runs.conflict_for(&grant, changed).await;
    assert!(
        matches!(changed, Err(RunsError::CommandConflict)),
        "{changed:?}"
    );

    // A report written first keeps its commit: a late conflict cannot
    // overwrite it.
    let second = f.start("s2", 2, "hang").await;
    let grant = f.executor(&second).await;
    let sha = f.commit("web/two.rs");
    let reported = runs.report(&grant, &sha, "Built it.".into()).await.unwrap();
    assert_eq!(reported.state, AttemptState::Committed);
    let late = runs.conflict_for(&grant, input("c3")).await;
    assert!(matches!(late, Err(RunsError::AlreadyCommitted)), "{late:?}");
    assert!(runs.conflict(&second.id).await.unwrap().is_none());
    f.finish().await;
}

/// Acknowledging a conflict is not a stop: the attempt still runs, holds its
/// scope and its session, until its turn's end is confirmed.
#[tokio::test]
async fn conflict_ack_is_not_a_confirmed_stop() {
    let f = fixture().await;
    let attempt = f.start("s1", 1, "hang").await;
    let grant = f.executor(&attempt).await;
    let acked = f
        .core
        .runs()
        .conflict_for(&grant, input("c1"))
        .await
        .unwrap();
    assert_eq!(acked.attempt_state, AttemptState::Running);
    assert!(acked.scope_held);
    assert_eq!(
        (acked.attempt_id.clone(), acked.task, acked.version),
        (attempt.id.clone(), 1, 1)
    );
    assert_eq!(acked.project_id, f.project);
    assert_eq!(acked.workflow_id, f.workflow);
    // Paths are normalized, evidence keeps its order.
    let paths: Vec<_> = acked.evidence.iter().map(|e| e.path.as_str()).collect();
    assert_eq!(paths, ["lib/find.rs", "lib/missing.rs"]);
    let now = f.core.runs().attempt(&attempt.id).await.unwrap();
    assert_eq!((now.state, now.ended_at), (AttemptState::Running, None));
    assert!(f.held(&attempt).await);
    assert_eq!(f.sessions.live_count().await, 1);
    assert_eq!(f.conflict(&attempt).await.created_at, acked.created_at);
    f.finish().await;
}

/// The Planner hears the conflict at once, in the executor's name, while
/// the executor may still be running.
#[tokio::test]
async fn conflict_notifies_while_its_executor_is_running() {
    let f = fixture().await;
    f.planner_turn("p1", "fake-large").await;
    f.busy_planner().await;
    let attempt = f.start("s1", 1, "hang").await;
    let grant = f.executor(&attempt).await;
    let acked = f
        .core
        .runs()
        .conflict_for(&grant, input("c1"))
        .await
        .unwrap();
    assert_eq!(acked.delivery, ConflictDelivery::Delivered);
    assert_eq!(acked.attempt_state, AttemptState::Running);
    let notices = f.notices().await;
    assert_eq!(notices.len(), 1);
    let notice = notices[0].notice.clone().unwrap();
    assert_eq!(notice.executor.as_ref(), Some(&attempt.thread_id));
    assert_eq!(notice.card["outcome"], "running");
    assert_eq!(notice.card["conflict"]["observed"], acked.observed);
    assert!(
        notices[0].prompt.contains("task_stop"),
        "{}",
        notices[0].prompt
    );
    let principal: (String, String) = sqlx::query_as(
        "SELECT principal_kind, principal_id FROM command_record WHERE command_id = ?",
    )
    .bind(format!("notify-{}", attempt.id))
    .fetch_one(f.storage.reader())
    .await
    .unwrap();
    assert_eq!(
        principal,
        ("Executor".into(), attempt.thread_id.as_str().to_string())
    );
    f.finish().await;
}

/// The watcher settles the conflicted attempt once its turn ended: Stopped,
/// its session closed and its scope freed. Its sibling keeps running, and the
/// Planner hears no second, generic notice.
#[tokio::test]
async fn watch_settles_conflict_without_stopping_siblings() {
    let f = fixture().await;
    f.planner_turn("p1", "fake-large").await;
    f.busy_planner().await;
    let sibling = f.start("s1", 2, "hang").await;
    let attempt = f.start("s2", 1, "wait-for-release").await;
    let grant = f.executor(&attempt).await;
    f.core
        .runs()
        .conflict_for(&grant, input("c1"))
        .await
        .unwrap();
    f.release();
    let settled = f
        .conflict_until(&attempt, |c| {
            c.attempt_state != AttemptState::Running && !c.scope_held
        })
        .await;
    assert_eq!(settled.attempt_state, AttemptState::Stopped);
    let now = f.core.runs().attempt(&attempt.id).await.unwrap();
    assert_eq!(now.reason.as_deref(), Some("contract conflict"));
    let sibling_now = f.core.runs().attempt(&sibling.id).await.unwrap();
    assert_eq!(sibling_now.state, AttemptState::Running);
    assert!(f.held(&sibling).await);
    assert_eq!(f.sessions.live_count().await, 1);
    assert_eq!(f.notices().await.len(), 1);
    f.finish().await;
}

/// A Stop or a close that could not terminate the executor keeps its scope:
/// it may still write.
#[tokio::test]
async fn failed_stop_or_close_keeps_the_scope() {
    let f = fixture().await;
    let attempt = f.start("s1", 1, "hang").await;
    let grant = f.executor(&attempt).await;
    f.core
        .runs()
        .conflict_for(&grant, input("c1"))
        .await
        .unwrap();
    assert!(
        f.sessions
            .force_termination_failure(&attempt.thread_id)
            .await
    );
    let planner = f.planner_grant().await;
    let stopped = f
        .core
        .runs()
        .stop_for(&planner, "stop-1".into(), &attempt.id)
        .await;
    assert!(stopped.is_err(), "{stopped:?}");
    assert!(f.held(&attempt).await);
    assert!(f.conflict(&attempt).await.scope_held);
    f.release();
    let _ = f.sessions.close_all().await;
    f.storage.close().await.unwrap();
}

/// A notice not delivered before a restart is delivered by recovery, once;
/// a crash between its enqueue and its mark replays it without a second one.
#[tokio::test]
async fn restart_replays_pending_conflict_once() {
    let f = fixture().await;
    let attempt = f.start("s1", 1, "hang").await;
    let grant = f.executor(&attempt).await;
    // The Planner has no turn yet to take settings from: the notice waits.
    let acked = f
        .core
        .runs()
        .conflict_for(&grant, input("c1"))
        .await
        .unwrap();
    assert_eq!(acked.delivery, ConflictDelivery::Pending);
    f.planner_turn("p1", "fake-large").await;
    f.busy_planner().await;
    let report = f.core.runs().recover().await.unwrap();
    assert_eq!(report.interrupted, 1);
    let delivered = f
        .conflict_until(&attempt, |c| c.delivery == ConflictDelivery::Delivered)
        .await;
    assert_eq!(delivered.attempt_state, AttemptState::Interrupted);
    assert_eq!(delivered.expected, acked.expected);
    assert_eq!(f.notices().await.len(), 1);
    sqlx::query("UPDATE run_attempts SET conflict_delivery = 'pending' WHERE id = ?")
        .bind(attempt.id.as_str())
        .execute(f.storage.reader())
        .await
        .unwrap();
    f.core.runs().recover().await.unwrap();
    f.conflict_until(&attempt, |c| c.delivery == ConflictDelivery::Delivered)
        .await;
    assert_eq!(f.notices().await.len(), 1);
    f.finish().await;
}

/// A retried notice is the one first resolved: changed Planner settings and
/// the attempt's later state do not change it.
#[tokio::test]
async fn changed_planner_settings_keep_original_notice() {
    let f = fixture().await;
    f.planner_turn("p1", "fake-large").await;
    let busy = f.busy_planner().await;
    let attempt = f.start("s1", 1, "hang").await;
    let grant = f.executor(&attempt).await;
    let acked = f
        .core
        .runs()
        .conflict_for(&grant, input("c1"))
        .await
        .unwrap();
    assert_eq!(acked.delivery, ConflictDelivery::Delivered);
    shadows_core::Turns::mark_operation_failed_for_test(
        &f.storage,
        &busy,
        shadows_core::testing::FailureStage::Prepare,
        "ended",
    )
    .await
    .unwrap();
    f.planner_turn("p2", "fake-small").await;
    f.busy_planner().await;
    sqlx::query("UPDATE run_attempts SET conflict_delivery = 'pending' WHERE id = ?")
        .bind(attempt.id.as_str())
        .execute(f.storage.reader())
        .await
        .unwrap();
    let planner = f.planner_grant().await;
    let stopped = f
        .core
        .runs()
        .stop_for(&planner, "stop-1".into(), &attempt.id)
        .await
        .unwrap();
    assert_eq!(stopped.state, AttemptState::Stopped);
    let now = f.conflict(&attempt).await;
    assert_eq!(now.delivery, ConflictDelivery::Delivered);
    let notices = f.notices().await;
    assert_eq!(notices.len(), 1);
    assert_eq!(notices[0].model, "fake-large");
    assert_eq!(
        notices[0].notice.as_ref().unwrap().card["outcome"],
        "running"
    );
    f.finish().await;
}

/// An executor that never ends its turn is stopped by the Planner: a
/// confirmed Stop that closes it and frees its scope. A replay answers the
/// same; an executor cannot stop anything.
#[tokio::test]
async fn planner_can_stop_a_nonending_executor() {
    let f = fixture().await;
    let attempt = f.start("s1", 1, "hang").await;
    let grant = f.executor(&attempt).await;
    f.core
        .runs()
        .conflict_for(&grant, input("c1"))
        .await
        .unwrap();
    tokio::time::sleep(Duration::from_millis(300)).await;
    assert_eq!(
        f.core.runs().attempt(&attempt.id).await.unwrap().state,
        AttemptState::Running
    );
    let by_executor = f
        .core
        .runs()
        .stop_for(&grant, "stop-x".into(), &attempt.id)
        .await;
    assert!(
        matches!(by_executor, Err(RunsError::NotFound)),
        "{by_executor:?}"
    );
    let planner = f.planner_grant().await;
    let stopped = f
        .core
        .runs()
        .stop_for(&planner, "stop-1".into(), &attempt.id)
        .await
        .unwrap();
    assert_eq!(stopped.state, AttemptState::Stopped);
    assert!(!f.held(&attempt).await);
    assert_eq!(f.sessions.live_count().await, 0);
    let replay = f
        .core
        .runs()
        .stop_for(&planner, "stop-1".into(), &attempt.id)
        .await
        .unwrap();
    assert_eq!(replay.state, AttemptState::Stopped);
    let conflict = f.conflict(&attempt).await;
    assert_eq!(conflict.attempt_state, AttemptState::Stopped);
    assert!(!conflict.scope_held);
    f.finish().await;
}

/// The executor's uncommitted work stays in the folder after its conflict
/// stop: nothing is reset or committed, and the next Start sees it.
#[tokio::test]
async fn conflict_keeps_uncommitted_files() {
    let f = fixture().await;
    let attempt = f.start("s1", 1, "wait-for-release").await;
    std::fs::write(f.root.join("lib/wip.rs"), "half done\n").unwrap();
    let grant = f.executor(&attempt).await;
    f.core
        .runs()
        .conflict_for(&grant, input("c1"))
        .await
        .unwrap();
    f.release();
    f.conflict_until(&attempt, |c| !c.scope_held).await;
    assert_eq!(
        std::fs::read_to_string(f.root.join("lib/wip.rs")).unwrap(),
        "half done\n"
    );
    let status = git(&f.root, &["status", "--porcelain"]);
    assert!(status.contains("?? lib/wip.rs"), "{status}");
    assert_eq!(git(&f.root, &["rev-list", "--count", "HEAD"]).trim(), "1");
    let again = f
        .core
        .runs()
        .start("s2".into(), &f.workflow, 1, 1, "fake-large".into())
        .await;
    assert!(
        matches!(again, Err(RunsError::DirtyScope { .. })),
        "{again:?}"
    );
    f.finish().await;
}

/// A conflict is the executor's report about its own attempt, never an
/// approval: it changes no plan or mark, and only an executor grant files it.
#[tokio::test]
async fn executor_conflicts_cannot_approve() {
    let f = fixture().await;
    let attempt = f.start("s1", 1, "hang").await;
    let grant = f.executor(&attempt).await;
    let planner = f.planner_grant().await;
    let by_planner = f.core.runs().conflict_for(&planner, input("c0")).await;
    assert!(
        matches!(by_planner, Err(RunsError::NotFound)),
        "{by_planner:?}"
    );
    let external = Grant {
        id: GrantId::generate(),
        kind: GrantKind::Project,
        project_id: f.project.clone(),
        thread_id: None,
        created_at: "2026-11-14T00:00:00Z".into(),
        revoked_at: None,
        scope: None,
    };
    let by_external = f.core.runs().conflict_for(&external, input("c0")).await;
    assert!(
        matches!(by_external, Err(RunsError::NotFound)),
        "{by_external:?}"
    );
    f.core
        .runs()
        .conflict_for(&grant, input("c1"))
        .await
        .unwrap();
    let recorded: (String, String, String, String) = sqlx::query_as(
        "SELECT principal_kind, principal_id, command_kind, command_scope_key
         FROM command_record WHERE command_kind = 'run.conflict'",
    )
    .fetch_one(f.storage.reader())
    .await
    .unwrap();
    assert_eq!(
        recorded,
        (
            "Executor".into(),
            attempt.thread_id.as_str().to_string(),
            "run.conflict".into(),
            attempt.id.as_str().to_string()
        )
    );
    let state: (String, i64) = sqlx::query_as("SELECT state, version FROM workflow WHERE id = ?")
        .bind(SOURCE)
        .fetch_one(f.storage.reader())
        .await
        .unwrap();
    assert_eq!(state, ("Frozen".into(), 1));
    let versions: i64 = sqlx::query_scalar("SELECT count(*) FROM workflow")
        .fetch_one(f.storage.reader())
        .await
        .unwrap();
    assert_eq!(versions, 1);
    let run = f.core.runs().task(&f.workflow, 1, 1).await.unwrap();
    assert!(!run.done);
    f.finish().await;
}

/// Bounds and paths: an invalid conflict is `invalid_conflict`; a secret or
/// escaping path stays the code service's refusal. Nothing is recorded.
#[tokio::test]
async fn invalid_conflicts_are_refused() {
    let f = fixture().await;
    let attempt = f.start("s1", 1, "hang").await;
    let grant = f.executor(&attempt).await;
    let mut empty = input("c1");
    empty.expected = "   ".into();
    let mut many = input("c2");
    many.evidence = vec![many.evidence[0].clone(); 11];
    let mut line = input("c3");
    line.evidence[0].line = Some(0);
    let mut none = input("c4");
    none.evidence.clear();
    let mut long = input("c5");
    long.observed = "x".repeat(2_001);
    for bad in [empty, many, line, none, long] {
        let refused = f.core.runs().conflict_for(&grant, bad).await;
        let Err(error) = refused else {
            panic!("accepted: {refused:?}")
        };
        assert_eq!(error.code(), "invalid_conflict", "{error}");
    }
    for path in [".env", "../outside.rs", "/etc/passwd"] {
        let mut bad = input("c6");
        bad.evidence[0].path = path.into();
        let refused = f.core.runs().conflict_for(&grant, bad).await;
        assert!(
            matches!(refused, Err(RunsError::Code(_))),
            "{path}: {refused:?}"
        );
    }
    assert!(f.core.runs().conflict(&attempt.id).await.unwrap().is_none());
    f.finish().await;
}
