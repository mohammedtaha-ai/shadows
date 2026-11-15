//! Slice 5 through the signed-in Claude Code subscription and real MCP.
//! Run explicitly with `cargo test -p shadows --test checks_live -- --ignored --nocapture`.
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use serde_json::{Value, json};
use shadows_core::testing::acp;
use shadows_core::{AttemptOrigin, CheckOutcome, PlanOp, TaskContent, ThreadEntryKind};

#[path = "fixtures/app.rs"]
mod app;
#[path = "fixtures/listening.rs"]
mod listening;
#[path = "fixtures/live.rs"]
mod live;
#[path = "fixtures/plan.rs"]
mod plan;

fn git(root: &Path, args: &[&str]) {
    let result = std::process::Command::new("git")
        .current_dir(root)
        .args(args)
        .output()
        .unwrap();
    assert!(result.status.success(), "git {args:?}: {result:?}");
}

async fn repository(app: &app::App) -> PathBuf {
    let project = app.core.projects().get(&app.project).await.unwrap();
    let root = PathBuf::from(project.directory.unwrap());
    std::fs::write(root.join(".gitignore"), "*.sqlite3*\n").unwrap();
    git(&root, &["init", "-q"]);
    git(&root, &["config", "user.email", "checks@example.invalid"]);
    git(&root, &["config", "user.name", "Checks acceptance"]);
    git(&root, &["config", "core.autocrlf", "false"]);
    git(&root, &["add", ".gitignore"]);
    git(&root, &["commit", "-q", "-m", "start"]);
    root
}

fn task() -> TaskContent {
    serde_json::from_value(json!({
        "number":1, "title":"Add Count", "phases":[],
        "goal":"Create src/count.js exporting Count(items), returning items.length. Run both \
            checks and report each honestly. The external check is intentionally unavailable: \
            never create unavailable.txt or change its command. Report its failure, not a \
            contract conflict. When Shadows returns this known failure, keep the correct file \
            and report the same commit if no file needs changing.",
        "reads":[], "writes":["src/count.js"],
        "acceptance":[{"number":1,"text":"Count returns the length of its array argument"}],
        "contract":{
            "files":[{"path":"src/count.js","action":"create","purpose":"Exports Count"}],
            "symbols":[{"id":"s1","path":"src/count.js","kind":"function","name":"Count",
                "inputs":[{"name":"items","type_name":"Array","meaning":"items to count"}],
                "returns":{"name":"","type_name":"number","meaning":"array length"},
                "behavior":["Returns items.length"]}],
            "produces":[{"id":"count","symbol_id":"s1","acceptance":[1]}],
            "checks":[
                {"id":"count","run":["node","--input-type=module","-e",
                    "const {Count}=await import('./src/count.js'); \
                     if(Count([])!==0 || Count([1,2,3])!==3) process.exit(1); \
                     console.log('Count passed')"],
                    "proves":[1],"timeout_secs":10},
                {"id":"external","run":["git","cat-file","-e","HEAD:unavailable.txt"],
                    "proves":[1],"timeout_secs":10}
            ]
        }
    }))
    .unwrap()
}

async fn reply(app: &app::App, question: &str) -> Value {
    let deadline = Instant::now() + Duration::from_secs(600);
    loop {
        for entry in app::entries(app).await {
            if entry.kind == ThreadEntryKind::UserMessage
                && entry.author.kind == "Executor"
                && let Some(card) = entry.card
                && card["message"] == question
            {
                assert!(card["reason"].is_null(), "reply failed: {card}");
                assert!(card["reply"].as_str().is_some_and(|s| !s.trim().is_empty()));
                return card;
            }
        }
        assert!(Instant::now() < deadline, "no reply to {question}");
        tokio::time::sleep(Duration::from_millis(250)).await;
    }
}

#[tokio::test]
#[ignore = "live: needs the signed-in Claude Code subscription"]
async fn checks_returns_judgment_and_agent_replies_on_the_subscription() {
    let adapter = live::subscription_adapter();
    let tmp = tempfile::tempdir().unwrap();
    let l = listening::listening_on(tmp.path(), adapter.adapter.clone()).await;
    let app = &l.app;
    let root = repository(app).await;
    let workflow = plan::draft(app).await.workflow_id;
    let ops = serde_json::to_string(&[PlanOp::TaskAdd { task: task() }]).unwrap();
    let prompt = format!(
        "Use plan_edit for workflow_id {workflow}, expected_revision 0, ops {ops}. \
         Keep the two checks exactly as given. Do not run it yet or approve it. \
         This is an integration trial: when later notices arrive, read task_checks and \
         acknowledge them, but do not amend, mark done or start tasks. The person will judge."
    );
    let planned =
        live::planner_turn_on(app, "write-checks", &prompt, live::SUBSCRIPTION_MODEL).await;
    assert_eq!(planned.status_kind, "Completed", "{planned:?}");
    println!("Planner wrote the command checks through MCP");
    let plan = shadows_core::AppCore::plans_for_tests(app.storage.clone())
        .get_plan_for_test(&workflow)
        .await
        .unwrap();
    assert_eq!(plan.tasks.len(), 1);
    assert_eq!(
        plan.tasks[0]
            .content
            .contract
            .as_ref()
            .unwrap()
            .checks
            .len(),
        2
    );
    shadows_core::AppCore::plans_for_tests(app.storage.clone())
        .approve_plan_for_test(
            &app::ctx("approve", "PlanApprove"),
            &workflow,
            plan.revision,
        )
        .await
        .unwrap();
    let started = app
        .core
        .runs()
        .start(
            "start".into(),
            &workflow,
            1,
            1,
            live::SUBSCRIPTION_MODEL.into(),
        )
        .await
        .unwrap();
    let client = listening::thread_client(&l, &app.thread).await;
    let question = "Why does Count return the array length?";
    let queued = listening::ok(
        &client,
        "message_agent",
        json!({
            "workflow_id":workflow,"task_number":1,"message":question
        }),
    )
    .await;
    assert_eq!(queued["attempt_id"], started.id.as_str());
    assert_eq!(queued["state"], "waiting");
    reply(app, question).await;
    println!("The question queued while running reached its Planner as a reply");

    let deadline = Instant::now() + Duration::from_secs(900);
    let latest = loop {
        let attempts = app.core.runs().attempts(&workflow, 1, 1).await.unwrap();
        assert!(attempts.len() <= 3, "extra attempts: {attempts:?}");
        let latest = &attempts[0];
        let checks = app.core.runs().check_results(&latest.id).await.unwrap();
        if attempts.len() == 3
            && checks.len() == 2
            && checks.iter().all(|c| c.finished_at.is_some())
        {
            assert_eq!(latest.origin, AttemptOrigin::Checks);
            assert_eq!(checks[0].outcome, Some(CheckOutcome::Passed));
            assert_eq!(checks[1].outcome, Some(CheckOutcome::Failed));
            assert!(checks.iter().all(|c| c.reported.is_some()));
            break latest.clone();
        }
        assert!(
            Instant::now() < deadline,
            "confirmation stalled: {attempts:?}"
        );
        tokio::time::sleep(Duration::from_millis(250)).await;
    };
    let deadline = Instant::now() + Duration::from_secs(120);
    loop {
        let notice = app::entries(app).await.into_iter().find_map(|entry| {
            entry.card.filter(|card| {
                card["attempt_id"] == latest.id.as_str()
                    && card["checks"]
                        .as_array()
                        .is_some_and(|checks| checks.len() == 2)
            })
        });
        if let Some(card) = notice {
            assert_eq!(card["checks"][0]["outcome"], "passed");
            assert_eq!(card["checks"][1]["outcome"], "failed");
            assert!(card["checks"][0]["matches"].is_boolean());
            break;
        }
        assert!(
            Instant::now() < deadline,
            "the final results never reached the Planner"
        );
        tokio::time::sleep(Duration::from_millis(250)).await;
    }
    println!("Two checks confirmed on three attempts; the final notice retained both results");
    let checked = listening::ok(
        &client,
        "task_checks",
        json!({
            "workflow_id":workflow,"task_number":1
        }),
    )
    .await;
    assert_eq!(checked.as_array().unwrap().len(), 2);
    let error = listening::refused(
        &client,
        "task_done",
        json!({
            "workflow_id":workflow,"task_number":1,"done":true
        }),
    )
    .await;
    assert!(error.contains("checks_not_passed"), "{error}");
    let reason = "Count reviewed; the deliberately absent external fixture stays unverified.";
    let marked = listening::ok(
        &client,
        "task_done",
        json!({
            "workflow_id":workflow,"task_number":1,"done":true,"reason":reason
        }),
    )
    .await;
    assert_eq!(marked["task"]["done_reason"], reason);
    let question = "Which command remained unavailable, and why?";
    listening::ok(
        &client,
        "message_agent",
        json!({
            "workflow_id":workflow,"task_number":1,"message":question
        }),
    )
    .await;
    reply(app, question).await;
    assert_eq!(
        app.core
            .runs()
            .attempts(&workflow, 1, 1)
            .await
            .unwrap()
            .len(),
        3
    );
    assert!(root.join("src/count.js").is_file());
    println!(
        "subscription acceptance: 3 attempts, 2 results, done reason and 2 replies; {}",
        latest.id
    );
    app.core
        .shut_down(Duration::from_secs(10), std::future::pending())
        .await
        .unwrap();
}
