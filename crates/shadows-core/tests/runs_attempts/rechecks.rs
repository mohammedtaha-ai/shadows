//! Done tasks retain their marks while their recorded checks run again.
use super::*;

async fn done_task(failing: bool) -> Fixture {
    let run = if failing {
        vec!["git".into(), "not-a-command".into()]
    } else {
        vec!["git".into(), "--version".into()]
    };
    let (f, id, _, _) = super::checks::candidate(vec![run]).await;
    sqlx::query("UPDATE run_attempts SET notice_sent_at=? WHERE id=?")
        .bind("2026-11-16T00:00:00Z")
        .bind(id.as_str())
        .execute(f.storage.reader())
        .await
        .unwrap();
    f.core
        .runs()
        .mark_done(
            "done-three".into(),
            &f.workflow,
            1,
            3,
            true,
            Some("Verified by the person".into()),
        )
        .await
        .unwrap();
    f
}

/// The person asks for a recheck of the fixture plan's project under `command`.
async fn asked(f: &Fixture, workflow: &WorkflowId, command: &str) -> Option<String> {
    f.core
        .runs()
        .recheck_project(command.into(), workflow, None)
        .await
        .unwrap()
        .recheck_id
}

async fn cause(f: &Fixture) {
    asked(f, &f.workflow, "cause").await;
}

async fn batches(f: &Fixture) -> i64 {
    sqlx::query_scalar("SELECT COUNT(*) FROM run_rechecks")
        .fetch_one(f.storage.reader())
        .await
        .unwrap()
}

#[tokio::test]
async fn a_mark_that_does_not_complete_its_plan_starts_no_recheck() {
    let f = done_task(false).await;
    f.core
        .runs()
        .mark_done("one".into(), &f.workflow, 1, 1, true, None)
        .await
        .unwrap();
    assert_eq!(batches(&f).await, 0);
    f.finish().await;
}

#[tokio::test]
async fn the_mark_that_completes_its_plan_starts_one_recheck() {
    let f = done_task(false).await;
    for task in [1, 4, 5, 6] {
        f.core
            .runs()
            .mark_done(format!("done-{task}"), &f.workflow, 1, task, true, None)
            .await
            .unwrap();
        assert_eq!(batches(&f).await, 0, "T{task} is not the plan's last");
    }
    f.core
        .runs()
        .mark_done("done-7".into(), &f.workflow, 1, 7, true, None)
        .await
        .unwrap();
    finished(&f).await;
    assert_eq!(batches(&f).await, 1);
    let (task, cause, asked): (i64, i64, bool) = sqlx::query_as(
        "SELECT r.task,b.cause_task,b.asked FROM run_recheck_results r
         JOIN run_rechecks b ON b.id=r.recheck_id",
    )
    .fetch_one(f.storage.reader())
    .await
    .unwrap();
    assert_eq!((task, cause, asked), (3, 7, false));
    f.finish().await;
}

#[tokio::test]
async fn a_planner_asks_for_a_recheck_and_hears_it() {
    let f = done_task(true).await;
    let planner = super::checks::prepare_planner(&f).await;
    let grant = planner_grant(&f).await;
    let started = f
        .core
        .runs()
        .recheck_project_for(&grant, &f.workflow)
        .await
        .unwrap();
    assert!(started.recheck_id.is_some());
    assert_eq!(grant.thread_id.as_ref(), Some(&planner));
    let message = notified(&f, &planner).await;
    assert!(
        message.prompt.contains("As you asked"),
        "{}",
        message.prompt
    );
    assert!(
        message.prompt.contains("T3 c1 failed"),
        "{}",
        message.prompt
    );
    f.finish().await;
}

#[tokio::test]
async fn the_person_asks_for_a_recheck_and_the_plans_planner_hears_it() {
    let f = done_task(true).await;
    let planner = super::checks::prepare_planner(&f).await;
    cause(&f).await;
    let message = notified(&f, &planner).await;
    assert!(
        message.prompt.contains("As the person asked"),
        "{}",
        message.prompt
    );
    assert_eq!(message.notice.unwrap().card["cause_task"], 0);
    f.finish().await;
}

async fn finished(f: &Fixture) {
    for _ in 0..400 {
        let pending: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM run_rechecks WHERE finished_at IS NULL")
                .fetch_one(f.storage.reader())
                .await
                .unwrap();
        if pending == 0 {
            return;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    panic!("recheck did not finish");
}

#[tokio::test]
async fn a_done_mark_reruns_the_checks_of_the_plans_done_tasks() {
    let f = done_task(false).await;
    cause(&f).await;
    finished(&f).await;
    let (task, outcome, output): (i64, String, String) =
        sqlx::query_as("SELECT task,outcome,output FROM run_recheck_results")
            .fetch_one(f.storage.reader())
            .await
            .unwrap();
    assert_eq!(task, 3);
    assert_eq!(outcome, "passed");
    assert!(output.contains("git version"));
    f.finish().await;
}

#[tokio::test]
async fn a_replayed_ask_starts_no_second_recheck() {
    let f = done_task(false).await;
    cause(&f).await;
    cause(&f).await;
    finished(&f).await;
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM run_rechecks")
        .fetch_one(f.storage.reader())
        .await
        .unwrap();
    assert_eq!(count, 1);
    f.finish().await;
}

#[tokio::test]
async fn a_task_marked_done_again_stays_in_the_recheck() {
    let f = done_task(false).await;
    f.core
        .runs()
        .mark_done(
            "done-again".into(),
            &f.workflow,
            1,
            3,
            true,
            Some("Still verified".into()),
        )
        .await
        .unwrap();
    cause(&f).await;
    finished(&f).await;
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM run_recheck_results WHERE task=3")
        .fetch_one(f.storage.reader())
        .await
        .unwrap();
    assert_eq!(count, 1);
    f.finish().await;
}

#[tokio::test]
async fn a_done_task_with_no_commit_is_left_out() {
    let f = fixture().await;
    cause(&f).await;
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM run_rechecks")
        .fetch_one(f.storage.reader())
        .await
        .unwrap();
    assert_eq!(count, 0);
    f.finish().await;
}

async fn notified(f: &Fixture, planner: &ThreadId) -> shadows_core::QueuedMessage {
    for _ in 0..400 {
        if let Some(message) = f
            .core
            .turns()
            .queued(planner)
            .await
            .unwrap()
            .into_iter()
            .find(|message| {
                message
                    .notice
                    .as_ref()
                    .is_some_and(|notice| notice.card.get("recheck_id").is_some())
            })
        {
            return message;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    panic!("the recheck notice was not queued");
}

#[tokio::test]
async fn a_failing_recheck_tells_the_planner_once() {
    let f = done_task(true).await;
    let planner = super::checks::prepare_planner(&f).await;
    cause(&f).await;
    let message = notified(&f, &planner).await;
    let notice = message.notice.unwrap();
    assert!(notice.executor.is_none());
    assert_eq!(notice.card["failed"][0]["task"], 3);
    assert!(notice.card["failed"][0]["checks"][0]["output"].is_null());
    assert!(message.prompt.contains("T3 c1 failed"));
    assert!(f.core.runs().task(&f.workflow, 1, 3).await.unwrap().done);
    f.finish().await;
}

#[tokio::test]
async fn a_person_asked_recheck_with_a_failing_check_finishes() {
    let f = done_task(true).await;
    let runs = f.core.runs();
    let id = asked(&f, &f.workflow, "fails").await.unwrap();
    finished(&f).await;
    let last = runs.last_recheck(&f.workflow).await.unwrap().unwrap();
    assert_eq!(last.recheck_id, id);
    assert!(last.finished_at.is_some());
    assert_eq!(last.counts.failed, 1);
    let rows = f.core.runs().recheck_rows(&id).await.unwrap();
    let row = rows.iter().find(|row| row.check_id == "c1").unwrap();
    assert_eq!(row.outcome, Some(shadows_core::CheckOutcome::Failed));
    let mut watch = f.core.runs().watch_recheck(&id).await.unwrap();
    assert_eq!(watch.next().await, Some(shadows_core::RecheckOutput::End));
    f.finish().await;
}

#[tokio::test]
async fn a_passing_recheck_a_mark_began_sends_no_notice() {
    let f = done_task(false).await;
    let planner = super::checks::prepare_planner(&f).await;
    for task in [1, 4, 5, 6, 7] {
        f.core
            .runs()
            .mark_done(format!("done-{task}"), &f.workflow, 1, task, true, None)
            .await
            .unwrap();
    }
    finished(&f).await;
    assert_eq!(batches(&f).await, 1);
    assert!(f.core.turns().queued(&planner).await.unwrap().is_empty());
    f.finish().await;
}

#[tokio::test]
async fn an_asked_recheck_that_passes_says_every_check_passed() {
    let f = done_task(false).await;
    let planner = super::checks::prepare_planner(&f).await;
    cause(&f).await;
    let message = notified(&f, &planner).await;
    let said = "As the person asked, Shadows reran the checks of the project's done tasks. \
                Every check passed.";
    assert!(message.prompt.contains(said), "{}", message.prompt);
    assert_eq!(
        message.notice.unwrap().card["failed"],
        serde_json::json!([])
    );
    f.finish().await;
}

#[tokio::test]
async fn a_finished_batch_never_loses_its_notice() {
    let f = done_task(true).await;
    let planner = super::checks::prepare_planner(&f).await;
    cause(&f).await;
    notified(&f, &planner).await;
    let json: Option<String> = sqlx::query_scalar("SELECT notice_json FROM run_rechecks")
        .fetch_one(f.storage.reader())
        .await
        .unwrap();
    assert!(json.is_some());
    sqlx::query("UPDATE run_rechecks SET notice_sent_at=NULL")
        .execute(f.storage.reader())
        .await
        .unwrap();
    f.core.runs().recover().await.unwrap();
    for _ in 0..400 {
        let sent: Option<String> = sqlx::query_scalar("SELECT notice_sent_at FROM run_rechecks")
            .fetch_one(f.storage.reader())
            .await
            .unwrap();
        if sent.is_some() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    assert_eq!(f.core.turns().queued(&planner).await.unwrap().len(), 1);
    f.finish().await;
}

#[tokio::test]
async fn a_restart_runs_an_interrupted_recheck_again() {
    let f = done_task(false).await;
    cause(&f).await;
    finished(&f).await;
    // Reconstruct the durable state left by a daemon killed during this check.
    sqlx::query("UPDATE run_rechecks SET finished_at=NULL")
        .execute(f.storage.reader())
        .await
        .unwrap();
    sqlx::query(
        "UPDATE run_recheck_results SET finished_at=NULL,outcome=NULL,output=NULL,
         started_at='2026-11-23T00:00:00Z'",
    )
    .execute(f.storage.reader())
    .await
    .unwrap();
    f.core.runs().recover().await.unwrap();
    finished(&f).await;
    let (outcome, output, finished_at): (String, String, Option<String>) =
        sqlx::query_as("SELECT outcome,output,finished_at FROM run_recheck_results")
            .fetch_one(f.storage.reader())
            .await
            .unwrap();
    assert_eq!(outcome, "passed");
    assert!(output.contains("git version"), "{output}");
    assert!(finished_at.is_some());
    f.finish().await;
}

/// Plan A's T3, done while its check passed, then broken by plan B's T2 marked done, which
/// completes plan B.
async fn broken_by_another_plan() -> (Fixture, ThreadId) {
    let run = ["git", "config", "--get", "check.health"].map(String::from);
    let (f, id, _, _) = super::checks::candidate(vec![run.to_vec()]).await;
    f.git(&["config", "check.health", "ok"]);
    sqlx::query("UPDATE run_attempts SET notice_sent_at=? WHERE id=?")
        .bind("2026-11-16T00:00:00Z")
        .bind(id.as_str())
        .execute(f.storage.reader())
        .await
        .unwrap();
    f.core
        .runs()
        .mark_done(
            "done-three".into(),
            &f.workflow,
            1,
            3,
            true,
            Some("Verified by the person".into()),
        )
        .await
        .unwrap();
    let planner = super::checks::prepare_planner(&f).await;
    let other = WorkflowId::from_literal(TARGET);
    f.core
        .plans()
        .approve_plan_for_test(&command("approve-other"), &other, 1)
        .await
        .unwrap();
    f.git(&["config", "--unset", "check.health"]);
    f.core
        .runs()
        .mark_done("done-other".into(), &other, 1, 2, true, None)
        .await
        .unwrap();
    (f, planner)
}

async fn recheck_notices(f: &Fixture, planner: &ThreadId) -> usize {
    f.core
        .turns()
        .queued(planner)
        .await
        .unwrap()
        .iter()
        .filter(|message| {
            message
                .notice
                .as_ref()
                .is_some_and(|notice| notice.card.get("recheck_id").is_some())
        })
        .count()
}

#[tokio::test]
async fn a_done_mark_reruns_another_plans_done_tasks() {
    let (f, planner) = broken_by_another_plan().await;
    let message = notified(&f, &planner).await;
    let (workflow, outcome): (String, String) =
        sqlx::query_as("SELECT workflow_id,outcome FROM run_recheck_results")
            .fetch_one(f.storage.reader())
            .await
            .unwrap();
    assert_eq!(workflow, f.workflow.as_str());
    assert_eq!(outcome, "failed");
    let notice = message.notice.unwrap();
    assert_eq!(notice.card["failed"][0]["workflow_id"], f.workflow.as_str());
    assert_eq!(notice.card["failed"][0]["task"], 3);
    assert!(
        message.prompt.contains("the project's done tasks"),
        "{}",
        message.prompt
    );
    assert!(
        message
            .prompt
            .contains("Plan \"Web\" version 1: T3 c1 failed"),
        "{}",
        message.prompt
    );
    assert_eq!(recheck_notices(&f, &planner).await, 1);
    f.finish().await;
}

#[tokio::test]
async fn a_failure_still_failing_is_rerun_but_not_told_again() {
    let (f, planner) = broken_by_another_plan().await;
    notified(&f, &planner).await;
    cause(&f).await;
    finished(&f).await;
    tokio::time::sleep(Duration::from_millis(300)).await;
    let outcomes: Vec<String> = sqlx::query_scalar(
        "SELECT r.outcome FROM run_recheck_results r JOIN run_rechecks b ON b.id=r.recheck_id
         WHERE r.task=3 ORDER BY b.created_at,b.id",
    )
    .fetch_all(f.storage.reader())
    .await
    .unwrap();
    assert_eq!(outcomes, vec!["failed", "failed"]);
    let notice: Option<String> =
        sqlx::query_scalar("SELECT notice_json FROM run_rechecks WHERE cause_command='cause'")
            .fetch_one(f.storage.reader())
            .await
            .unwrap();
    assert!(notice.is_none(), "a failure already told is not told again");
    assert_eq!(recheck_notices(&f, &planner).await, 1);
    f.finish().await;
}

#[tokio::test]
async fn an_older_versions_done_task_is_rechecked() {
    let f = done_task(false).await;
    let grant = planner_grant(&f).await;
    let plans = f.core.plans();
    let plan = plans.get(&f.workflow).await.unwrap();
    let next = plans
        .start_draft_for_test(
            &command("next"),
            &Writer::Planner {
                thread: grant.thread_id.unwrap(),
                grant: grant.id,
            },
            &f.project,
            Some(&plan.plan_id),
            Some(&f.workflow),
            None,
            Some("Correction"),
            None,
            None,
            &[],
        )
        .await
        .unwrap();
    let draft = plans.get(&next.workflow_id).await.unwrap();
    plans
        .approve_plan_for_test(&command("approve-next"), &next.workflow_id, draft.revision)
        .await
        .unwrap();
    f.core
        .runs()
        .mark_done("done-next".into(), &next.workflow_id, 2, 1, true, None)
        .await
        .unwrap();
    asked(&f, &next.workflow_id, "ask-next").await;
    finished(&f).await;
    let (workflow, version): (String, i64) =
        sqlx::query_as("SELECT workflow_id,version FROM run_recheck_results")
            .fetch_one(f.storage.reader())
            .await
            .unwrap();
    assert_eq!(workflow, f.workflow.as_str());
    assert_eq!(version, 1);
    f.finish().await;
}

#[tokio::test]
async fn a_done_task_a_newer_version_marks_done_is_not_rechecked() {
    let f = done_task(false).await;
    let grant = planner_grant(&f).await;
    let plans = f.core.plans();
    let plan = plans.get(&f.workflow).await.unwrap();
    let next = plans
        .start_draft_for_test(
            &command("next"),
            &Writer::Planner {
                thread: grant.thread_id.unwrap(),
                grant: grant.id,
            },
            &f.project,
            Some(&plan.plan_id),
            Some(&f.workflow),
            None,
            Some("Correction"),
            None,
            None,
            &[],
        )
        .await
        .unwrap();
    let draft = plans.get(&next.workflow_id).await.unwrap();
    plans
        .approve_plan_for_test(&command("approve-next"), &next.workflow_id, draft.revision)
        .await
        .unwrap();
    // Version 2 marks T3 done too: its mark now stands for T3, so version 1's T3 is not
    // run again by this recheck or by the next one.
    for (command, task) in [("done-next-three", 3), ("done-next-one", 1)] {
        f.core
            .runs()
            .mark_done(command.into(), &next.workflow_id, 2, task, true, None)
            .await
            .unwrap();
        asked(&f, &next.workflow_id, &format!("ask-{command}")).await;
        finished(&f).await;
    }
    let rows: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM run_recheck_results")
        .fetch_one(f.storage.reader())
        .await
        .unwrap();
    assert_eq!(rows, 0, "version 1's T3 is superseded by version 2's mark");
    f.finish().await;
}

#[tokio::test]
async fn the_plan_page_reads_the_projects_last_recheck() {
    let f = done_task(false).await;
    let runs = f.core.runs();
    assert_eq!(runs.last_recheck(&f.workflow).await.unwrap(), None);
    let id = asked(&f, &f.workflow, "page").await.unwrap();
    finished(&f).await;
    let rows: u32 = sqlx::query_scalar("SELECT COUNT(*) FROM run_recheck_results")
        .fetch_one(f.storage.reader())
        .await
        .unwrap();
    assert!(rows > 0);
    let done = runs.last_recheck(&f.workflow).await.unwrap().unwrap();
    // The batch as it stood while running: no row finished yet.
    for sql in [
        "UPDATE run_rechecks SET finished_at=NULL",
        "UPDATE run_recheck_results SET finished_at=NULL",
    ] {
        sqlx::query(sql).execute(f.storage.reader()).await.unwrap();
    }
    let running = runs.last_recheck(&f.workflow).await.unwrap().unwrap();
    assert_eq!(running.recheck_id, id);
    assert_eq!(running.began, shadows_core::RecheckBegan::Person);
    assert_eq!(running.finished_at, None);
    assert_eq!(running.counts.unfinished, rows);
    assert_eq!(running.counts.passed, 0);
    assert_eq!(done.recheck_id, id);
    assert!(done.finished_at.is_some());
    assert_eq!(done.counts.unfinished, 0);
    assert_eq!(done.counts.passed, rows);
    assert_eq!(done.counts.failed + done.counts.not_run, 0);
    f.finish().await;
}

#[tokio::test]
async fn the_person_reads_a_finished_rechecks_rows_with_their_output() {
    let f = done_task(false).await;
    let runs = f.core.runs();
    let id = asked(&f, &f.workflow, "rows").await.unwrap();
    finished(&f).await;
    let rows = runs.recheck_rows(&id).await.unwrap();
    let row = rows.iter().find(|row| row.check_id == "c1").unwrap();
    assert_eq!(row.task, 3);
    assert_eq!(row.command, "git --version");
    assert_eq!(row.outcome, Some(shadows_core::CheckOutcome::Passed));
    assert!(row.finished_at.is_some());
    assert!(
        row.output.as_deref().unwrap().contains("git version"),
        "{:?}",
        row.output
    );
    assert!(matches!(
        runs.recheck_rows("no-such-batch").await,
        Err(shadows_core::RunsError::NotFound)
    ));
    f.finish().await;
}

#[tokio::test]
async fn the_person_watches_a_running_checks_output_live() {
    let run: Vec<String> = if cfg!(windows) {
        ["cmd", "/c", "echo printed-early& ping -n 4 127.0.0.1 >nul"]
    } else {
        ["sh", "-c", "echo printed-early; sleep 3"]
    }
    .iter()
    .map(|word| word.to_string())
    .collect();
    let (f, attempt, _, _) = super::checks::candidate(vec![run]).await;
    sqlx::query("UPDATE run_attempts SET notice_sent_at=? WHERE id=?")
        .bind("2026-11-16T00:00:00Z")
        .bind(attempt.as_str())
        .execute(f.storage.reader())
        .await
        .unwrap();
    let runs = f.core.runs();
    runs.mark_done(
        "done-three".into(),
        &f.workflow,
        1,
        3,
        true,
        Some("Verified by the person".into()),
    )
    .await
    .unwrap();
    let id = asked(&f, &f.workflow, "live").await.unwrap();
    let mut watch = runs.watch_recheck(&id).await.unwrap();
    let mut seen = Vec::new();
    let wait = tokio::time::timeout(std::time::Duration::from_secs(120), async {
        while let Some(output) = watch.next().await {
            seen.push(output);
        }
    });
    wait.await.expect("the batch ends");
    let chunk = seen
        .iter()
        .position(|output| {
            matches!(output, shadows_core::RecheckOutput::Chunk { check_id, text, .. }
                if check_id == "c1" && text.contains("printed-early"))
        })
        .unwrap_or_else(|| panic!("no chunk: {seen:?}"));
    let done = seen
        .iter()
        .position(|output| {
            matches!(output, shadows_core::RecheckOutput::Finished { check_id, .. }
                if check_id == "c1")
        })
        .unwrap_or_else(|| panic!("no finish: {seen:?}"));
    assert!(chunk < done, "{seen:?}");
    assert_eq!(seen.last(), Some(&shadows_core::RecheckOutput::End));
    finished(&f).await;
    let mut after = runs.watch_recheck(&id).await.unwrap();
    assert_eq!(after.next().await, Some(shadows_core::RecheckOutput::End));
    assert_eq!(after.next().await, None);
    f.finish().await;
}

/// The project stream's recheck events, as (kind, recheck_id), in sequence order.
async fn recheck_events(f: &Fixture) -> Vec<(String, String)> {
    f.core
        .events()
        .read_project_events_after(core_events::EventCursor(0), &f.project, 1000)
        .await
        .unwrap()
        .into_iter()
        .filter(|event| event.kind.starts_with("Recheck"))
        .map(|event| {
            let payload: Value = serde_json::from_str(&event.payload_json).unwrap();
            assert_eq!(payload["project_id"], f.project.as_str());
            assert_eq!(payload["workflow_id"], f.workflow.as_str());
            (
                event.kind,
                payload["recheck_id"].as_str().unwrap().to_string(),
            )
        })
        .collect()
}

#[tokio::test]
async fn an_asked_recheck_starts_and_finishes_on_the_project_stream() {
    let f = done_task(false).await;
    let id = asked(&f, &f.workflow, "cause").await.unwrap();
    asked(&f, &f.workflow, "cause").await;
    finished(&f).await;
    assert_eq!(
        recheck_events(&f).await,
        vec![
            ("RecheckStarted".to_string(), id.clone()),
            ("RecheckFinished".to_string(), id),
        ]
    );
    f.finish().await;
}

#[tokio::test]
async fn a_completing_mark_starts_a_recheck_on_the_project_stream() {
    let f = done_task(false).await;
    for task in [1, 4, 5, 6, 7] {
        f.core
            .runs()
            .mark_done(format!("done-{task}"), &f.workflow, 1, task, true, None)
            .await
            .unwrap();
    }
    let id: String = sqlx::query_scalar("SELECT id FROM run_rechecks")
        .fetch_one(f.storage.reader())
        .await
        .unwrap();
    let events = recheck_events(&f).await;
    assert_eq!(events[0], ("RecheckStarted".to_string(), id));
    finished(&f).await;
    f.finish().await;
}
