//! Start's admission: the plan's state and `needs` marks are checked again
//! inside the attempt's write, and a task's required files must exist before
//! it starts. A refused Start writes no attempt and no executor thread. The
//! executor runs on `fake_acp`, held until a `release` file appears.

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
    AcceptanceItem, AppCore, ContractSource, ContractUse, CoreParts, FileAction, FileWork, Link,
    LinkKind, PlanOp, TaskContent, TaskContract, TaskParent, WorkflowId, Writer,
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

fn file(path: &str, action: FileAction) -> FileWork {
    FileWork {
        path: path.into(),
        action,
        purpose: "the work".into(),
    }
}

fn contract(files: Vec<FileWork>, existing: &[&str]) -> TaskContract {
    TaskContract {
        files,
        symbols: vec![],
        produces: vec![],
        uses: existing
            .iter()
            .map(|path| ContractUse {
                source: ContractSource::Existing {
                    path: (*path).into(),
                    symbol: String::new(),
                },
                purpose: "it calls this".into(),
            })
            .collect(),
        steps: vec![],
        invariants: vec![],
        forbidden: vec![],
    }
}

fn task(number: u32, reads: &[&str], writes: &str, contract: Option<TaskContract>) -> PlanOp {
    PlanOp::TaskAdd {
        task: TaskContent {
            phases: Vec::new(),
            contract,
            number,
            title: format!("Task {number}"),
            goal: "A usable result".into(),
            reads: reads.iter().map(|r| (*r).into()).collect(),
            writes: vec![writes.into()],
            acceptance: vec![AcceptanceItem {
                number: 1,
                text: "The result works".into(),
            }],
        },
    }
}

/// T1 modifies `lib/a.rs` and creates `lib/new.rs`, reading `notes/**`;
/// T2 reads `missing/x.rs`, modifies `app/main.rs` and uses `core/gone.rs`;
/// T3 reads `linked/a.rs`; T4 needs T5; T5 has no contract.
fn tasks() -> Vec<PlanOp> {
    vec![
        task(
            1,
            &["notes/**"],
            "lib/**",
            Some(contract(
                vec![
                    file("lib/a.rs", FileAction::Modify),
                    file("lib/new.rs", FileAction::Create),
                ],
                &[],
            )),
        ),
        task(
            2,
            &[],
            "app/**",
            Some(contract(
                vec![
                    file("missing/x.rs", FileAction::Read),
                    file("app/main.rs", FileAction::Modify),
                ],
                &["core/gone.rs"],
            )),
        ),
        task(
            3,
            &[],
            "out/**",
            Some(contract(vec![file("linked/a.rs", FileAction::Read)], &[])),
        ),
        task(4, &[], "web/**", None),
        task(5, &[], "docs/**", None),
        PlanOp::LinkPut {
            link: Link {
                task: 4,
                after: TaskParent::Local(5),
                kind: LinkKind::Needs,
                label: "the docs".into(),
                waiting_items: vec![],
            },
        },
    ]
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
    for path in ["README.md", "lib/a.rs", "app/main.rs"] {
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

    /// Starts `task` held at the admission gate; answers once it waits there.
    async fn gated_start(
        &self,
        command: &str,
        task: u32,
    ) -> (
        tokio::task::JoinHandle<Result<Attempt, RunsError>>,
        tokio::sync::oneshot::Sender<()>,
    ) {
        let (reached, release) = self.core.runs().gate_admission_for_tests();
        let (core, workflow, command) = (
            self.core.clone(),
            self.workflow.clone(),
            command.to_string(),
        );
        let started = tokio::spawn(async move {
            core.runs()
                .start(command, &workflow, 1, task, "fake-large".into())
                .await
        });
        reached.await.unwrap();
        (started, release)
    }

    /// Attempts and executor threads written so far.
    async fn written(&self) -> (i64, i64) {
        let attempts: i64 = sqlx::query_scalar("SELECT count(*) FROM run_attempts")
            .fetch_one(self.storage.reader())
            .await
            .unwrap();
        let threads: i64 =
            sqlx::query_scalar("SELECT count(*) FROM planning_thread WHERE kind = 'executor'")
                .fetch_one(self.storage.reader())
                .await
                .unwrap();
        (attempts, threads)
    }

    /// A corrective Draft after version 1, as a Planner's next version is.
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

    async fn finish(self) {
        std::fs::write(self.root.join("release"), b"").unwrap();
        self.sessions.close_all().await.unwrap();
        self.storage.close().await.unwrap();
    }
}

#[tokio::test]
async fn admission_rechecks_a_new_draft() {
    let f = fixture().await;
    let first = f.start("s1", 1).await.unwrap();
    let (started, release) = f.gated_start("s5", 5).await;
    f.corrective_draft().await;
    release.send(()).unwrap();
    let refused = started.await.unwrap();
    assert!(matches!(refused, Err(RunsError::Superseded)), "{refused:?}");
    assert_eq!(f.written().await, (1, 1), "the refused Start wrote nothing");
    // A replay answers the first attempt, whatever the plan became.
    assert_eq!(f.start("s1", 1).await.unwrap().id, first.id);
    assert_eq!(f.written().await, (1, 1));
    f.finish().await;
}

#[tokio::test]
async fn admission_rechecks_needs() {
    let f = fixture().await;
    let runs = f.core.runs();
    runs.mark_done("m1".into(), &f.workflow, 1, 5, true)
        .await
        .unwrap();
    let (started, release) = f.gated_start("s4", 4).await;
    runs.mark_done("m2".into(), &f.workflow, 1, 5, false)
        .await
        .unwrap();
    release.send(()).unwrap();
    let refused = started.await.unwrap();
    let five = vec![TaskParent::Local(5)];
    assert!(
        matches!(&refused, Err(RunsError::Waiting { tasks }) if tasks == &five),
        "{refused:?}"
    );
    assert_eq!(f.written().await, (0, 0));
    f.finish().await;
}

#[tokio::test]
async fn a_create_target_need_not_exist() {
    let f = fixture().await;
    assert!(!f.root.join("lib/new.rs").exists());
    let started = f.start("s1", 1).await.unwrap();
    assert_eq!(started.state, AttemptState::Running);
    f.finish().await;
}

#[tokio::test]
async fn missing_existing_inputs_refuse_start() {
    let f = fixture().await;
    let refused = f.start("s2", 2).await;
    let missing = vec!["core/gone.rs".to_string(), "missing/x.rs".to_string()];
    assert!(
        matches!(&refused, Err(RunsError::MissingInput { paths }) if paths == &missing),
        "{refused:?}"
    );
    assert_eq!(refused.as_ref().unwrap_err().code(), "missing_input");
    let answer = f.core.runs().task(&f.workflow, 1, 2).await.unwrap();
    assert_eq!(answer.start.code.as_deref(), Some("missing_input"));
    assert_eq!(f.written().await, (0, 0));
    for path in ["core/gone.rs", "missing/x.rs"] {
        let at = f.root.join(path);
        std::fs::create_dir_all(at.parent().unwrap()).unwrap();
        std::fs::write(at, "here\n").unwrap();
    }
    assert_eq!(
        f.start("s2b", 2).await.unwrap().state,
        AttemptState::Running
    );
    f.finish().await;
}

#[tokio::test]
async fn a_reads_glob_is_not_a_required_file() {
    let f = fixture().await;
    assert!(!f.root.join("notes").exists());
    assert_eq!(f.start("s1", 1).await.unwrap().state, AttemptState::Running);
    f.finish().await;
}

#[cfg(unix)]
#[tokio::test]
async fn an_input_junction_out_is_refused() {
    let f = fixture().await;
    let away = f._tmp.path().join("away");
    std::fs::create_dir(&away).unwrap();
    std::fs::write(away.join("a.rs"), "secret\n").unwrap();
    std::os::unix::fs::symlink(&away, f.root.join("linked")).unwrap();
    let refused = f.start("s3", 3).await;
    assert!(
        matches!(&refused, Err(RunsError::InputUnavailable(_))),
        "{refused:?}"
    );
    assert_eq!(refused.unwrap_err().code(), "input_unavailable");
    // Absent under the link out is refused too, never answered as missing.
    std::fs::remove_file(away.join("a.rs")).unwrap();
    let refused = f.start("s3b", 3).await;
    assert!(
        matches!(&refused, Err(RunsError::InputUnavailable(_))),
        "{refused:?}"
    );
    assert_eq!(f.written().await, (0, 0));
    f.finish().await;
}

#[tokio::test]
async fn unrelated_dirty_files_do_not_refuse_start() {
    let f = fixture().await;
    std::fs::write(f.root.join("README.md"), "changed\n").unwrap();
    std::fs::write(f.root.join("scratch.txt"), "new\n").unwrap();
    assert_eq!(f.start("s1", 1).await.unwrap().state, AttemptState::Running);
    // A change inside the task's writes still refuses it.
    std::fs::write(f.root.join("app/main.rs"), "changed\n").unwrap();
    std::fs::create_dir_all(f.root.join("missing")).unwrap();
    std::fs::write(f.root.join("missing/x.rs"), "x\n").unwrap();
    std::fs::create_dir_all(f.root.join("core")).unwrap();
    std::fs::write(f.root.join("core/gone.rs"), "x\n").unwrap();
    let dirty = f.start("s2", 2).await;
    assert!(
        matches!(dirty, Err(RunsError::DirtyScope { .. })),
        "{dirty:?}"
    );
    f.finish().await;
}

#[tokio::test]
async fn old_work_reports_after_a_corrective_draft() {
    let f = fixture().await;
    let attempt = f.start("s1", 1).await.unwrap();
    f.corrective_draft().await;
    std::fs::write(f.root.join("lib/a.rs"), "done\n").unwrap();
    git(&f.root, &["add", "--", "lib/a.rs"]);
    git(&f.root, &["commit", "-q", "-m", "T1"]);
    let sha = git(&f.root, &["rev-parse", "HEAD"]).trim().to_string();
    let grant = shadows_core::Grant {
        id: shadows_core::GrantId::generate(),
        kind: shadows_core::GrantKind::Executor,
        project_id: f.project.clone(),
        thread_id: Some(attempt.thread_id.clone()),
        created_at: "2026-11-14T00:00:00Z".into(),
        revoked_at: None,
        scope: Some(shadows_core::testing::TaskScope {
            workflow: SOURCE.into(),
            version: 1,
            task: 1,
        }),
    };
    let reported = f
        .core
        .runs()
        .report(&grant, &sha, "Changed lib/a.rs.".into())
        .await
        .unwrap();
    assert_eq!(reported.state, AttemptState::Committed);
    assert_eq!(reported.commit.as_deref(), Some(sha.as_str()));
    // New work on the superseded version is refused.
    let refused = f.start("s5", 5).await;
    assert!(matches!(refused, Err(RunsError::Superseded)), "{refused:?}");
    f.finish().await;
}
