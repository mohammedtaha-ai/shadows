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

const T1_GOAL: &str = "Create src/t1.rs containing `pub fn t1() {}` with the Write tool.";

/// An approved v1 whose T1 writes `src/**`.
async fn approved_task(app: &app::App) -> shadows_core::WorkflowId {
    approved_task_with(app, T1_GOAL).await
}

/// An approved v1 whose T1 writes `src/**` and has `goal`.
async fn approved_task_with(app: &app::App, goal: &str) -> shadows_core::WorkflowId {
    let v1 = plan::draft(app).await.workflow_id;
    let mut t1 = plan::task(1, "Add t1");
    t1.goal = goal.into();
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

/// What a started run holds: the adapter, the app's folder and the app.
struct Run {
    _live: live::LiveAdapter,
    _tmp: tempfile::TempDir,
    l: listening::Listening,
}

/// A listening app on the subscription whose Planner has one recorded turn,
/// with T1 of an approved v1 started on the production first prompt.
async fn started(goal: &str) -> (Run, shadows_core::WorkflowId, shadows_core::Attempt) {
    let live = live::subscription_adapter();
    let tmp = tempfile::tempdir().unwrap();
    let l = listening::listening_on(tmp.path(), live.adapter.clone()).await;
    repository(&l.app).await;
    let v1 = approved_task_with(&l.app, goal).await;
    let ready = "Answer with the word ready.";
    let done = live::planner_turn_on(&l.app, "p1", ready, live::SUBSCRIPTION_MODEL).await;
    assert_eq!(done.status_kind, "Completed", "{done:?}");
    let attempt = l
        .app
        .core
        .runs()
        .start("live-t1".into(), &v1, 1, 1, live::SUBSCRIPTION_MODEL.into())
        .await
        .unwrap();
    let run = Run {
        _live: live,
        _tmp: tmp,
        l,
    };
    (run, v1, attempt)
}

/// Polls up to `seconds` for an attempt that left `Running` and whose turn ended.
async fn settled(app: &app::App, attempt: &shadows_core::Attempt, seconds: u64) {
    for _ in 0..seconds * 10 {
        let now = app.core.runs().attempt(&attempt.id).await.unwrap();
        let ended = app
            .core
            .turns()
            .operations(&attempt.thread_id)
            .await
            .unwrap()
            .iter()
            .any(|op| Some(&op.id) == attempt.operation_id.as_ref() && op.finished_at.is_some());
        if now.state != shadows_core::AttemptState::Running && ended {
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    panic!("the attempt did not settle");
}

/// The Planner's entry whose body holds every one of `texts`, waited for up
/// to three minutes.
async fn planner_entry(app: &app::App, texts: &[&str]) -> String {
    for _ in 0..1800 {
        let entries = app::entries(app).await;
        if let Some(entry) = entries
            .iter()
            .find(|e| texts.iter().all(|t| e.body.contains(t)))
        {
            return entry.body.clone();
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    panic!("the Planner was not told: {texts:?}");
}

/// The executor commits and reports; its turn ends and the attempt keeps its
/// scope, the Planner hears the commit, and the done mark frees the scope and
/// closes the executor's session.
#[tokio::test]
#[ignore = "live: needs the signed-in Claude Code subscription"]
async fn live_a_committed_attempt_reaches_its_planner_and_waits_for_its_judgment() {
    let (run, v1, attempt) = started(T1_GOAL).await;
    let app = &run.l.app;
    settled(app, &attempt, 600).await;
    let now = app.core.runs().attempt(&attempt.id).await.unwrap();
    assert_eq!(now.state, shadows_core::AttemptState::Committed, "{now:?}");
    let sha = now.commit.clone().unwrap();
    let t1 = app.core.runs().task(&v1, 1, 1).await.unwrap();
    assert_eq!(
        t1.start.code.as_deref(),
        Some("task_busy"),
        "{:?}",
        t1.start
    );
    let notice = planner_entry(app, &[&sha, "attempt 1 committed"]).await;
    println!("live notice: {}", notice.lines().next().unwrap_or_default());
    app.core
        .runs()
        .mark_done("m1".into(), &v1, 1, 1, true)
        .await
        .unwrap();
    let t1 = app.core.runs().task(&v1, 1, 1).await.unwrap();
    assert_ne!(
        t1.start.code.as_deref(),
        Some("task_busy"),
        "{:?}",
        t1.start
    );
    assert!(app.sessions.offered(&attempt.thread_id).await.is_none());
    app.core
        .shut_down(std::time::Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
}

/// The slice's acceptance: the Planner starts T1, hears it committed, sends
/// an addition the executor carries on with in its own session, hears the
/// second commit, judges it and marks T1 done.
#[tokio::test]
#[ignore = "live: needs the signed-in Claude Code subscription"]
async fn live_the_planner_runs_judges_and_amends_a_task() {
    let live = live::subscription_adapter();
    let tmp = tempfile::tempdir().unwrap();
    let l = listening::listening_on(tmp.path(), live.adapter.clone()).await;
    let app = &l.app;
    let root = repository(app).await;
    let v1 = approved_task(app).await;
    let prompt = format!(
        "Run T1 of plan version {v1} with task_start, model sonnet. When Shadows tells you \
         T1's attempt 1 committed, call task_amend for T1 with the addition: add the line \
         `// two` at the end of src/t1.rs. When Shadows tells you T1's attempt 2 committed, \
         read src/t1.rs and call task_done for T1 with done true."
    );
    let first = live::planner_turn_on(app, "live-journey", &prompt, live::SUBSCRIPTION_MODEL).await;
    assert_eq!(first.status_kind, "Completed", "{first:?}");
    let mut done = false;
    for _ in 0..9000 {
        if app.core.runs().task(&v1, 1, 1).await.unwrap().done {
            done = true;
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    let attempts = app.core.runs().attempts(&v1, 1, 1).await.unwrap();
    assert!(done, "T1 was not marked done: {attempts:?}");
    assert_eq!(attempts.len(), 2, "{attempts:?}");
    let (second, first) = (&attempts[0], &attempts[1]);
    for attempt in [first, second] {
        assert_eq!(
            attempt.state,
            shadows_core::AttemptState::Committed,
            "{attempt:?}"
        );
    }
    assert_eq!(first.thread_id, second.thread_id);
    assert!(
        second
            .addition
            .as_deref()
            .unwrap_or_default()
            .contains("// two"),
        "{second:?}"
    );
    let (sha1, sha2) = (
        first.commit.clone().unwrap(),
        second.commit.clone().unwrap(),
    );
    let written = git(&root, &["show", &format!("{sha2}:src/t1.rs")]);
    assert!(
        written.contains("pub fn t1") && written.contains("// two"),
        "{written}"
    );
    let entries = app::entries(app).await;
    for tool in ["task_start", "task_amend", "task_done"] {
        let called = entries
            .iter()
            .any(|e| e.kind == ThreadEntryKind::ToolCall && format!("{e:?}").contains(tool));
        assert!(called, "{tool} was not called");
    }
    println!("live journey: {sha1} {sha2}");
    app.core
        .shut_down(std::time::Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
}

/// The executor makes no commit: the attempt fails, its scope is freed, and
/// the Planner hears why.
#[tokio::test]
#[ignore = "live: needs the signed-in Claude Code subscription"]
async fn live_an_attempt_without_a_commit_reaches_its_planner() {
    let goal = "Change no file and make no commit: answer that there is nothing to do, and stop.";
    let (run, v1, attempt) = started(goal).await;
    let app = &run.l.app;
    settled(app, &attempt, 600).await;
    let now = app.core.runs().attempt(&attempt.id).await.unwrap();
    assert_eq!(now.state, shadows_core::AttemptState::Failed, "{now:?}");
    assert_eq!(now.reason.as_deref(), Some("ended without a commit"));
    let notice = planner_entry(app, &["attempt 1 failed: ended without a commit"]).await;
    println!("live notice: {}", notice.lines().next().unwrap_or_default());
    let t1 = app.core.runs().task(&v1, 1, 1).await.unwrap();
    assert!(t1.start.allowed, "{:?}", t1.start);
    app.core
        .shut_down(std::time::Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
}
