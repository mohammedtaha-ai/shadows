//! External project grants use the same task lifecycle as Planner grants.

#[allow(dead_code)]
#[path = "fixtures/cross_plan.rs"]
mod cross_plan;

use std::path::Path;
use std::sync::Arc;

use core_runs::{AttemptState, RunsError};
use cross_plan::{SOURCE, command};
use shadows_core::testing::{LiveHandles, Storage, acp};
use shadows_core::{AppCore, CoreParts, Grant, GrantKind, ProjectId, WorkflowId, Writer};

struct Fixture {
    core: Arc<AppCore>,
    storage: Arc<Storage>,
    workflow: WorkflowId,
    grant: Grant,
    _temp: tempfile::TempDir,
}

fn git(root: &Path, args: &[&str]) {
    let output = std::process::Command::new("git")
        .current_dir(root)
        .args(args)
        .output()
        .unwrap();
    assert!(output.status.success(), "git {args:?}: {output:?}");
}

async fn fixture() -> Fixture {
    let (temp, storage) = cross_plan::fixture().await;
    let root = temp.path().join("project");
    std::fs::create_dir(&root).unwrap();
    git(&root, &["init", "-q"]);
    git(
        &root,
        &[
            "-c",
            "user.name=Runs",
            "-c",
            "user.email=runs@example.invalid",
            "commit",
            "-q",
            "--allow-empty",
            "-m",
            "start",
        ],
    );
    sqlx::query("UPDATE project SET directory = ? WHERE id = 'p'")
        .bind(root.to_str().unwrap())
        .execute(storage.reader())
        .await
        .unwrap();
    let workflow = WorkflowId::from_literal(SOURCE);
    let plans = AppCore::plans_for_tests(storage.clone());
    plans
        .edit_plan_for_test(
            &command("external-runs-tasks"),
            &Writer::Person,
            None,
            &workflow,
            0,
            &[cross_plan::task(1), cross_plan::task(2)],
        )
        .await
        .unwrap();
    plans
        .approve_plan_for_test(&command("external-runs-approve"), &workflow, 1)
        .await
        .unwrap();
    let (runtime, _) = AppCore::runtime_for_tests(storage.clone()).await.unwrap();
    let core = AppCore::assemble(CoreParts {
        operations: AppCore::operations_for_tests(storage.clone()),
        secrets: shadows_core::testing::secrets(storage.clone(), None),
        sessions: acp::fake_sessions(storage.clone()),
        storage: storage.clone(),
        runtime: Arc::new(runtime),
        handles: Arc::new(LiveHandles::default()),
        bus: tokio::sync::broadcast::channel(64).0,
        ui: tokio::sync::broadcast::channel(16).0,
        mcp_url: acp::MCP_URL.to_string(),
    });
    let project = ProjectId::from_literal("p");
    let issued = core
        .grants()
        .issue_external("external-connection".into(), None, vec![project.clone()])
        .await
        .unwrap();
    let grant = core
        .grants()
        .external_project(&issued.connection.id, &project)
        .await
        .unwrap();
    for task in [1, 2] {
        core.runs()
            .choose_executor_for_test(&workflow, task, "claude-code", "fake-large", Some("high"))
            .await
            .unwrap();
    }
    core.runs().set_first_prompt_for_tests("wait-for-release");
    Fixture {
        core,
        storage,
        workflow,
        grant,
        _temp: temp,
    }
}

#[tokio::test]
async fn an_external_grant_starts_and_marks_a_task_of_its_project() {
    let f = fixture().await;
    let runs = f.core.runs();
    let attempt = runs.start_for(&f.grant, &f.workflow, 1).await.unwrap();
    assert_eq!(attempt.state, AttemptState::Running);
    runs.stop(&attempt.id).await.unwrap();
    let marked = runs
        .mark_done_for(&f.grant, &f.workflow, 1, true, None)
        .await
        .unwrap();
    assert!(marked.task.done);
    assert_eq!(
        marked.next.iter().map(|task| task.task).collect::<Vec<_>>(),
        [2]
    );
}

#[tokio::test]
async fn an_external_grant_of_another_project_is_refused() {
    let f = fixture().await;
    let mut other = f.grant;
    other.project_id = ProjectId::from_literal("other-project");
    let result = f.core.runs().start_for(&other, &f.workflow, 1).await;
    assert!(matches!(result, Err(RunsError::NotFound)), "{result:?}");
}

#[tokio::test]
async fn an_external_stop_is_recorded_under_its_grant() {
    let f = fixture().await;
    let runs = f.core.runs();
    let attempt = runs
        .start("person-start".into(), &f.workflow, 1, 1)
        .await
        .unwrap();
    assert_eq!(attempt.state, AttemptState::Running);
    let stopped = runs
        .stop_for(&f.grant, "external-stop".into(), &attempt.id)
        .await
        .unwrap();
    assert_eq!(stopped.state, AttemptState::Stopped);
    let (kind, id): (String, String) = sqlx::query_as(
        "SELECT principal_kind, principal_id FROM command_record WHERE command_id = ?",
    )
    .bind("external-stop")
    .fetch_one(f.storage.reader())
    .await
    .unwrap();
    assert_eq!(kind, "Grant");
    assert_eq!(id, f.grant.id.as_str());
}

#[tokio::test]
async fn an_executor_grant_is_still_refused() {
    let f = fixture().await;
    let mut executor = f.grant;
    executor.kind = GrantKind::Executor;
    let runs = f.core.runs();
    let start = runs.start_for(&executor, &f.workflow, 1).await;
    assert!(matches!(start, Err(RunsError::NotFound)), "{start:?}");
    let mark = runs
        .mark_done_for(&executor, &f.workflow, 1, true, None)
        .await;
    assert!(matches!(mark, Err(RunsError::NotFound)), "{mark:?}");
}

#[tokio::test]
async fn an_external_grant_reaches_git_init() {
    let f = fixture().await;
    let result = f.core.runs().init_git(&f.grant).await;
    assert!(matches!(result, Err(RunsError::AlreadyGit)), "{result:?}");
}
