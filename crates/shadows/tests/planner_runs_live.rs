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

/// The Planner reads the committed change with its own tools before judging it.
#[tokio::test]
#[ignore = "live: needs the signed-in Claude Code subscription"]
async fn live_the_planner_reads_the_commit_before_it_judges() {
    let live = live::subscription_adapter();
    let tmp = tempfile::tempdir().unwrap();
    let l = listening::listening_on(tmp.path(), live.adapter.clone()).await;
    let app = &l.app;
    repository(app).await;
    let v1 = approved_task(app).await;
    let prompt = format!(
        "Run T1 of plan version {v1} with task_start, model sonnet. When Shadows tells you \
         T1's attempt committed, judge it as your instructions say: task_done, or task_amend."
    );
    let first = live::planner_turn_on(app, "live-review", &prompt, live::SUBSCRIPTION_MODEL).await;
    assert_eq!(first.status_kind, "Completed", "{first:?}");
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(900);
    let (t1, attempts) = loop {
        let t1 = app.core.runs().task(&v1, 1, 1).await.unwrap();
        let attempts = app.core.runs().attempts(&v1, 1, 1).await.unwrap();
        if t1.done || attempts.len() > 1 {
            break (t1, attempts);
        }
        assert!(
            tokio::time::Instant::now() < deadline,
            "no judgment: {attempts:?}"
        );
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    };
    let notice = planner_entry(app, &["attempt 1 committed", "task_diff"]).await;
    let entries = app::entries(app).await;
    let ordinal = entries
        .iter()
        .find(|entry| entry.body == notice)
        .unwrap()
        .ordinal;
    assert!(
        entries.iter().any(|entry| {
            let text = format!("{entry:?}");
            entry.ordinal > ordinal
                && entry.kind == ThreadEntryKind::ToolCall
                && (text.contains("task_diff") || text.contains("task_file"))
        }),
        "the Planner did not read the commit: {entries:?}"
    );
    println!("live review: done={} attempts={}", t1.done, attempts.len());
    app.core
        .shut_down(std::time::Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
}

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
    planner_notice(app, texts).await.body
}

/// `planner_entry`, the whole entry.
async fn planner_notice(app: &app::App, texts: &[&str]) -> shadows_core::ThreadEntry {
    for _ in 0..1800 {
        let entries = app::entries(app).await;
        if let Some(entry) = entries
            .into_iter()
            .find(|e| texts.iter().all(|t| e.body.contains(t)))
        {
            return entry;
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    panic!("the Planner was not told: {texts:?}");
}

/// The notice is the executor's message of the attempt, with its card; the
/// card is printed and answered.
fn notice_card(
    notice: &shadows_core::ThreadEntry,
    attempt: &shadows_core::Attempt,
    outcome: &str,
) -> serde_json::Value {
    assert_eq!(notice.kind, ThreadEntryKind::UserMessage);
    assert_eq!(notice.author.kind, "Executor");
    assert_eq!(notice.author.id, attempt.thread_id.as_str());
    let card = notice.card.clone().expect("the notice carries its card");
    assert_eq!(
        (
            &card["task"],
            &card["attempt"],
            &card["model"],
            &card["outcome"]
        ),
        (
            &serde_json::json!(1),
            &serde_json::json!(1),
            &serde_json::json!("sonnet"),
            &serde_json::json!(outcome)
        ),
        "{card}"
    );
    println!("live notice card: {card}");
    card
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
    let entry = planner_notice(app, &[&sha, "attempt 1 committed"]).await;
    let card = notice_card(&entry, &now, "committed");
    assert_eq!(card["commit"], sha.as_str());
    assert!(
        !card["report"].as_str().unwrap_or_default().is_empty(),
        "{card}"
    );
    let notice = entry.body;
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
    let entry = planner_notice(
        app,
        &[
            "attempt 1 failed: ended without a commit",
            "The executor's last message:",
        ],
    )
    .await;
    let card = notice_card(&entry, &now, "failed");
    assert!(
        !card["reason"].as_str().unwrap_or_default().is_empty(),
        "{card}"
    );
    let notice = entry.body;
    let (_, message) = notice
        .split_once("The executor's last message:\n")
        .expect("the failed notice carries the executor's last message");
    let entries = app.core.threads().entries(&now.thread_id).await.unwrap();
    let last = entries
        .iter()
        .rev()
        .find(|entry| {
            entry.kind == ThreadEntryKind::AgentMessage
                && entry.operation_id.as_deref() == now.operation_id.as_ref().map(|id| id.as_str())
        })
        .expect("the executor's last message for this attempt");
    assert!(
        !last.body.trim().is_empty(),
        "the executor must leave an answer"
    );
    let mut expected: String = last.body.chars().take(4_000).collect();
    if last.body.chars().count() > 4_000 {
        expected.push_str("[cut]");
    }
    assert!(
        message.starts_with(&format!("{expected}\n")),
        "the notice must carry this attempt's last message: {notice}"
    );
    println!("live notice: {}", notice.lines().next().unwrap_or_default());
    println!(
        "live last message: {}",
        message.lines().next().unwrap_or_default()
    );
    let t1 = app.core.runs().task(&v1, 1, 1).await.unwrap();
    assert!(t1.start.allowed, "{:?}", t1.start);
    app.core
        .shut_down(std::time::Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
}

/// The latest plan version the app's Planner thread wrote.
async fn planner_plan(app: &app::App) -> shadows_core::Plan {
    let plans = shadows_core::AppCore::plans_for_tests(app.storage.clone());
    let Some(latest) = plans.thread_plan_for_test(&app.thread).await.unwrap() else {
        let entries = app::entries(app).await;
        let said: Vec<&str> = entries.iter().map(|e| e.body.as_str()).collect();
        panic!("the Planner wrote no plan; its thread: {said:#?}");
    };
    plans.get_plan_for_test(&latest).await.unwrap()
}

/// The required layers of `kind` in the base standards, each placed under `at`.
fn layers(kind: &str, at: &str) -> Vec<shadows_core::PartLayer> {
    shadows_core::Standards::base()
        .parts
        .iter()
        .find(|part| part.name == kind)
        .unwrap()
        .layers
        .iter()
        .filter(|layer| layer.required)
        .map(|layer| shadows_core::PartLayer {
            layer: layer.id.clone(),
            place: format!("{at}{}", layer.id),
            not_used: None,
        })
        .collect()
}

/// Gives the app's project what a person approves before plans: a whole
/// vision, a map whose parts have kinds (the Go backend under internal/ of a
/// module at the folder's root, its PostgreSQL database), and one roadmap
/// outcome over both, as the
/// person's own design edit writes them. The app's seed already approved the
/// map and roadmap revisions.
async fn approved_roadmap(app: &app::App) {
    use shadows_core::{DesignOp, OutcomeContent, OutcomeId, PartContent, PartId, VisionField};
    let backend = app.part.clone().expect("the app seeds a part");
    let database = PartId::generate();
    let outcome = OutcomeId::generate();
    let mut ops = vec![
        DesignOp::PartPut {
            id: backend.clone(),
            content: PartContent {
                title: "Shop backend".into(),
                responsibility: "The shop's HTTP service, a Go module at the folder's root".into(),
                kind: Some("backend".into()),
                folder: "internal".into(),
                layers: layers("backend", "internal/"),
                ..Default::default()
            },
        },
        DesignOp::PartCreate {
            id: database.clone(),
            parent: None,
            before: None,
            content: PartContent {
                title: "Shop database".into(),
                responsibility: "PostgreSQL 16, run by docker compose".into(),
                kind: Some("database".into()),
                folder: "db".into(),
                layers: layers("database", "db/"),
                ..Default::default()
            },
        },
        DesignOp::OutcomeCreate {
            id: outcome.clone(),
            parent: None,
            before: None,
            content: OutcomeContent {
                title: "The shop runs".into(),
                intended_result: "The shop's service starts against its database".into(),
                acceptance: vec!["The service answers on its health endpoint".into()],
            },
        },
        DesignOp::OutcomePartPut {
            outcome: outcome.clone(),
            part: backend,
        },
        DesignOp::OutcomePartPut {
            outcome,
            part: database,
        },
    ];
    for (field, text) in [
        (VisionField::Purpose, "An online shop"),
        (VisionField::Users, "People who buy from the shop"),
        (VisionField::Goals, "Customers can browse and order"),
        (VisionField::Boundaries, "No payments in the first release"),
        (
            VisionField::TechnicalDirection,
            "A Go service on PostgreSQL 16",
        ),
    ] {
        ops.push(DesignOp::VisionItemAdd {
            field,
            text: text.into(),
        });
    }
    app.core
        .plans()
        .edit_design("live-setup-design".into(), &app.project, 0, ops)
        .await
        .unwrap();
    let stage = app.core.design().stage(&app.project).await.unwrap();
    assert_eq!(format!("{:?}", stage.stage), "Plans", "{stage:?}");
}

/// Whether the Planner's thread holds a call of `tool`.
fn called(entries: &[shadows_core::ThreadEntry], tool: &str) -> bool {
    entries
        .iter()
        .any(|e| e.kind == ThreadEntryKind::ToolCall && format!("{e:?}").contains(tool))
}

/// The slice's acceptance: on an empty folder that is no repository, the
/// Planner writes a Setup plan, runs it once approved (its first commit made by
/// project_git_init), and plans the next feature on the files that appeared.
#[tokio::test]
#[ignore = "live: needs the signed-in Claude Code subscription and Docker"]
async fn live_the_planner_sets_up_an_empty_folder() {
    let live = live::subscription_adapter();
    let tmp = tempfile::tempdir().unwrap();
    let l = listening::listening_on(tmp.path(), live.adapter.clone()).await;
    let app = &l.app;
    let project = app.core.projects().get(&app.project).await.unwrap();
    let root = PathBuf::from(project.directory.unwrap());
    // The test app keeps its database in the project folder: the one file a
    // person's folder would not hold, so it is ignored as the other tests do.
    std::fs::write(root.join(".gitignore"), "*.sqlite3*\n").unwrap();
    assert!(!root.join(".git").exists());
    approved_roadmap(app).await;

    let ask = "The roadmap is approved. Set this project up: a Go module named \
               example.com/shop in the folder's root, and a docker-compose.yml that runs \
               PostgreSQL 16 on port 5432. Write the Setup plan now; do not run it yet.";
    let first = live::planner_turn_on(app, "live-setup-1", ask, live::SUBSCRIPTION_MODEL).await;
    assert_eq!(first.status_kind, "Completed", "{first:?}");
    let setup = planner_plan(app).await;
    assert_eq!(setup.title.trim(), "Setup", "{setup:?}");
    assert_eq!(format!("{:?}", setup.state), "Draft", "{setup:?}");
    assert!(!setup.tasks.is_empty(), "{setup:?}");
    plan::tie_to_part(app, &setup.id).await;
    shadows_core::AppCore::plans_for_tests(app.storage.clone())
        .approve_plan_for_test(
            &app::ctx("approve-setup", "PlanApprove"),
            &setup.id,
            setup.revision,
        )
        .await
        .unwrap();

    let run = "The Setup plan is approved. Run it.";
    let second = live::planner_turn_on(app, "live-setup-2", run, live::SUBSCRIPTION_MODEL).await;
    assert_eq!(second.status_kind, "Completed", "{second:?}");
    let numbers: Vec<u32> = setup.tasks.iter().map(|t| t.content.number).collect();
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(1200);
    let mut seen: Vec<String> = Vec::new();
    loop {
        let mut all_done = true;
        let mut states = Vec::new();
        for n in &numbers {
            let task = app
                .core
                .runs()
                .task(&setup.id, setup.version, *n)
                .await
                .unwrap();
            all_done &= task.done;
            let state = task.latest.map(|a| format!("{:?}#{}", a.state, a.number));
            states.push(format!("T{n}={}", state.unwrap_or_else(|| "-".into())));
        }
        let line = states.join(" ");
        if seen.last() != Some(&line) {
            println!("live setup states: {line}");
            seen.push(line);
        }
        if all_done {
            break;
        }
        assert!(
            tokio::time::Instant::now() < deadline,
            "the setup was not done: {seen:?}"
        );
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    }

    assert!(!git(&root, &["rev-parse", "HEAD"]).trim().is_empty());
    let authors = git(&root, &["log", "--format=%an"]);
    assert!(authors.lines().any(|a| a == "Shadows"), "{authors}");
    assert!(authors.lines().any(|a| a != "Shadows"), "{authors}");
    let status = git(&root, &["status", "--porcelain"]);
    assert_eq!(status.trim(), "", "uncommitted after the setup: {status}");
    let mut commits = Vec::new();
    let mut committed = String::new();
    for n in &numbers {
        let attempts = app
            .core
            .runs()
            .attempts(&setup.id, setup.version, *n)
            .await
            .unwrap();
        if let Some(sha) = attempts.first().and_then(|a| a.commit.clone()) {
            committed.push_str(&git(&root, &["show", "--name-only", "--format=", &sha]));
            commits.push(sha);
        }
    }
    for file in ["go.mod", "docker-compose.yml"] {
        assert!(root.join(file).exists(), "{file} is missing");
        assert!(
            committed.lines().any(|l| l.trim() == file),
            "{file} is in no task's commit: {committed}"
        );
    }
    let entries = app::entries(app).await;
    for tool in ["project_git_init", "task_start", "task_done"] {
        assert!(called(&entries, tool), "{tool} was not called");
    }

    // The turn that marked the last task done may still be ending.
    for _ in 0..1800 {
        let operations = app.core.turns().operations(&app.thread).await.unwrap();
        if operations.iter().all(|op| op.finished_at.is_some()) {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    let next = "Now plan the first feature: an HTTP health endpoint. Write the plan.";
    let third = live::planner_turn_on(app, "live-setup-3", next, live::SUBSCRIPTION_MODEL).await;
    assert_eq!(third.status_kind, "Completed", "{third:?}");
    // The feature plan is the project's newest version of a plan other than the setup's.
    let feature: Option<String> = sqlx::query_scalar(
        "SELECT w.id FROM workflow w JOIN plan p ON p.id = w.plan_id
         WHERE p.project_id = ? AND w.plan_id != ?
         ORDER BY w.created_at DESC, w.id DESC LIMIT 1",
    )
    .bind(app.project.as_str())
    .bind(setup.plan_id.as_str())
    .fetch_optional(app.storage.reader())
    .await
    .unwrap();
    let Some(feature) = feature else {
        let last = app::last_agent_entry(app).await;
        panic!("the Planner wrote no feature plan; it said: {}", last.body);
    };
    let feature = shadows_core::AppCore::plans_for_tests(app.storage.clone())
        .get_plan_for_test(&shadows_core::WorkflowId::from_literal(feature))
        .await
        .unwrap();
    let created_dirs: Vec<String> = committed
        .lines()
        .filter_map(|l| l.trim().rsplit_once('/').map(|(d, _)| format!("{d}/")))
        .collect();
    // A path some task of the plan creates may be read by a later one.
    let created: Vec<String> = feature
        .tasks
        .iter()
        .flat_map(|t| t.content.contract.iter().flat_map(|c| c.files.iter()))
        .filter(|f| {
            let action = serde_json::to_value(f.action).unwrap();
            action
                .as_str()
                .is_some_and(|a| a.eq_ignore_ascii_case("create"))
        })
        .map(|f| f.path.clone())
        .collect();
    let mut on_setup = false;
    for task in &feature.tasks {
        let value = serde_json::to_value(&task.content).unwrap();
        let mut named: Vec<String> = task.content.reads.clone();
        named.extend(task.content.writes.iter().cloned());
        on_setup |= named.iter().any(|p| {
            p.trim_start_matches("./") == "go.mod"
                || created_dirs.iter().any(|d| p.starts_with(d.as_str()))
        });
        let files = value["contract"]["files"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        let is_create = |f: &&serde_json::Value| {
            f["action"]
                .as_str()
                .is_some_and(|a| a.eq_ignore_ascii_case("create"))
        };
        let changed = files
            .iter()
            .filter(|f| !is_create(f))
            .filter_map(|f| f["path"].as_str());
        let exact = task.content.reads.iter().map(String::as_str).chain(changed);
        for path in exact.filter(|p| !p.contains(['*', '?', '['])) {
            assert!(
                root.join(path).exists() || created.iter().any(|c| c == path),
                "T{} names {path}, which neither exists nor is created: {value}",
                task.content.number
            );
        }
    }
    assert!(
        on_setup,
        "the feature plan names no setup file: {feature:?}"
    );
    println!("live setup: {}", commits.join(" "));
    app.core
        .shut_down(std::time::Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
}
