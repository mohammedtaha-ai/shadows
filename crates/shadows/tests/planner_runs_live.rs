//! The Planner and its executors on the real adapter, on the person's signed-in
//! Claude Code subscription, reaching Shadows' `/mcp` on a real port. CI skips
//! these; run one signed in, with `-- --ignored --test-threads=1`.

use std::path::{Path, PathBuf};

use shadows_core::ThreadEntryKind;
use shadows_core::testing::acp;

#[path = "fixtures/app.rs"]
mod app;
#[path = "fixtures/listening.rs"]
mod listening;
#[path = "fixtures/live.rs"]
mod live;
#[path = "fixtures/plan.rs"]
mod plan;

fn git(root: &Path, args: &[&str]) -> String {
    let out = std::process::Command::new("git")
        .current_dir(root)
        .args(args)
        .output()
        .unwrap();
    assert!(out.status.success(), "git {args:?}: {:?}", out);
    String::from_utf8(out.stdout).unwrap()
}

/// The project folder as a person's repository: one commit, the daemon's
/// database ignored.
async fn repository(app: &app::App) -> PathBuf {
    let project = app.core.projects().get(&app.project).await.unwrap();
    let root = PathBuf::from(project.directory.unwrap());
    git(&root, &["init", "-q"]);
    git(&root, &["config", "user.email", "planner@example.invalid"]);
    git(&root, &["config", "user.name", "Planner"]);
    git(&root, &["config", "core.autocrlf", "false"]);
    std::fs::write(root.join(".gitignore"), "*.sqlite3*\n").unwrap();
    git(&root, &["add", ".gitignore"]);
    git(&root, &["commit", "-q", "-m", "start"]);
    root
}

/// An approved v1 whose T1 writes `src/**`.
async fn approved_task(app: &app::App) -> shadows_core::WorkflowId {
    let v1 = plan::draft(app).await.workflow_id;
    let mut t1 = plan::task(1, "Add t1");
    t1.goal = "Create src/t1.rs containing `pub fn t1() {}` with the Write tool.".into();
    t1.writes = vec!["src/**".into()];
    plan::edit(app, &v1, 0, &[shadows_core::PlanOp::TaskAdd { task: t1 }]).await;
    shadows_core::AppCore::plans_for_tests(app.storage.clone())
        .approve_plan_for_test(&app::ctx("approve-1", "PlanApprove"), &v1, 1)
        .await
        .unwrap();
    v1
}

/// Asked to run T1, the Planner calls `task_start`: one attempt runs on an
/// executor thread, on the model it named.
#[tokio::test]
#[ignore = "live: needs the signed-in Claude Code subscription"]
async fn live_the_planner_starts_the_task_it_was_asked_to_run() {
    let live = live::subscription_adapter();
    let tmp = tempfile::tempdir().unwrap();
    let l = listening::listening_on(tmp.path(), live.adapter.clone()).await;
    let app = &l.app;
    repository(app).await;
    let v1 = approved_task(app).await;
    let prompt =
        format!("Run T1 of plan version {v1} with task_start, with model sonnet. Do nothing else.");
    let done = live::planner_turn_on(app, "live-start", &prompt, live::SUBSCRIPTION_MODEL).await;
    assert_eq!(done.status_kind, "Completed", "{done:?}");
    let entries = app::entries(app).await;
    let called = entries
        .iter()
        .any(|e| e.kind == ThreadEntryKind::ToolCall && format!("{e:?}").contains("task_start"));
    assert!(called, "task_start was not called: {entries:?}");
    let attempts = app.core.runs().attempts(&v1, 1, 1).await.unwrap();
    assert_eq!(attempts.len(), 1, "{attempts:?}");
    let attempt = &attempts[0];
    assert_eq!(attempt.model, "sonnet");
    let kind: String = sqlx::query_scalar("SELECT kind FROM planning_thread WHERE id = ?")
        .bind(attempt.thread_id.as_str())
        .fetch_one(app.storage.reader())
        .await
        .unwrap();
    assert_eq!(kind, "executor");
    let stopped = app.core.runs().stop(&attempt.id).await.unwrap();
    println!("live start: {:?}", stopped.state);
    app.core
        .shut_down(std::time::Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
}
