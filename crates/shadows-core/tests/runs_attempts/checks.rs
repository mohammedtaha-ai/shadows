//! Recorded confirmations use real Git commands, a real repository and persisted attempts.
use super::*;
use core_runs::{CheckOutcome, CheckResult};
use serde_json::json;

pub(super) async fn candidate(run: Vec<Vec<String>>) -> (Fixture, AttemptId, Grant, String) {
    candidate_with_timeout(run, 5).await
}

async fn candidate_with_timeout(
    run: Vec<Vec<String>>,
    timeout: u32,
) -> (Fixture, AttemptId, Grant, String) {
    let f = fixture_with(false, None).await;
    let PlanOp::TaskAdd { task: mut content } = task(3, "backend/app/**") else {
        unreachable!()
    };
    let checks: Vec<_> = run
        .into_iter()
        .enumerate()
        .map(|(i, run)| json!({"id":format!("c{}",i+1),"run":run,"timeout_secs":timeout}))
        .collect();
    content.contract = Some(
        serde_json::from_value(json!({
            "files":[{"path":"backend/app/a.php","action":"create","purpose":"Count"}],
            "checks":checks
        }))
        .unwrap(),
    );

    f.core
        .plans()
        .edit_plan_for_test(
            &command("checks"),
            &Writer::Person,
            None,
            &f.workflow,
            1,
            &[PlanOp::TaskUpdate { task: content }],
        )
        .await
        .unwrap();
    f.core
        .plans()
        .approve_plan_for_test(&command("approve-checks"), &f.workflow, 2)
        .await
        .unwrap();
    let (id, grant) = f.begin(3).await;
    let context = f
        .core
        .plans()
        .task_context(&grant, Some(&f.workflow), 3, None)
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO run_attempt_context (attempt_id,context_json,captured_at) VALUES (?,?,?)",
    )
    .bind(id.as_str())
    .bind(serde_json::to_string(&context).unwrap())
    .bind("2026-11-15T00:00:00Z")
    .execute(f.storage.reader())
    .await
    .unwrap();
    f.put("backend/app/a.php", "one\n");
    let sha = f.commit(&["backend/app/a.php"], "build");
    let reports: Vec<_> = checks
        .iter()
        .map(|c| json!({"id":c["id"],"outcome":"passed"}))
        .collect();
    f.core
        .runs()
        .report(
            &grant,
            &sha,
            "Built.".into(),
            serde_json::from_value(json!(reports)).unwrap(),
        )
        .await
        .unwrap();
    (f, id, grant, sha)
}

async fn clone_attempt(f: &Fixture, previous: &AttemptId, name: &str, origin: &str) -> AttemptId {
    sqlx::query(
        "INSERT INTO run_attempts
        (id,project_id,workflow_id,version,task,number,thread_id,model,state,writes_json,
         started_at,command_id,fingerprint,start_head,addition,origin)
        SELECT ?,project_id,workflow_id,version,task,number+1,thread_id,model,'running',writes_json,
            started_at,?,'fixture',commit_sha,'check results',? FROM run_attempts WHERE id=?",
    )
    .bind(name)
    .bind(name)
    .bind(origin)
    .bind(previous.as_str())
    .execute(f.storage.reader())
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO run_attempt_context (attempt_id,context_json,captured_at)
        SELECT ?,context_json,captured_at FROM run_attempt_context WHERE attempt_id=?",
    )
    .bind(name)
    .bind(previous.as_str())
    .execute(f.storage.reader())
    .await
    .unwrap();
    sqlx::query("UPDATE run_attempts SET released_at='2026-11-15T00:00:01Z' WHERE id=?")
        .bind(previous.as_str())
        .execute(f.storage.reader())
        .await
        .unwrap();
    AttemptId::from_recorded(name.into())
}

#[tokio::test]
async fn check_time_limit_ends_it() {
    let (f, id, _, _) = candidate_with_timeout(
        vec![cmd(&["git", "-c", "alias.pause=!sleep 3", "pause"])],
        1,
    )
    .await;
    f.core.runs().recover().await.unwrap();
    assert_eq!(
        results(&f, &id, 1).await[0].outcome,
        Some(CheckOutcome::TimedOut)
    );
    f.finish().await;
}

#[tokio::test]
async fn long_output_keeps_its_end() {
    use std::io::Write;
    let (f, id, _, _) = candidate(vec![cmd(&["git", "config", "--get", "check.tail"])]).await;
    let mut config = std::fs::OpenOptions::new()
        .append(true)
        .open(f.root.join(".git/config"))
        .unwrap();
    writeln!(config, "\n[check]\n tail = {}TAIL", "x".repeat(70_000)).unwrap();
    drop(config);
    f.core.runs().recover().await.unwrap();
    let rows = results(&f, &id, 1).await;
    assert_eq!(rows[0].outcome, Some(CheckOutcome::Passed));
    assert!(rows[0].truncated);
    let output = rows[0].output.as_deref().unwrap();
    assert_eq!(output.len(), 65_536);
    assert!(output.ends_with("TAIL\n"));
    f.finish().await;
}

#[tokio::test]
async fn return_without_change_reconfirms_previous_commit() {
    let (f, id, grant, sha) = candidate(vec![cmd(&["git", "--version"])]).await;
    let next = clone_attempt(&f, &id, "returned", "checks").await;
    let reported = f
        .core
        .runs()
        .report(
            &grant,
            &sha,
            "No code change was needed.".into(),
            serde_json::from_value(json!([{"id":"c1","outcome":"passed"}])).unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(reported.id, next);
    assert_eq!(reported.commit.as_deref(), Some(sha.as_str()));
    f.core.runs().recover().await.unwrap();
    assert_eq!(
        results(&f, &next, 1).await[0].outcome,
        Some(CheckOutcome::Passed)
    );
    f.finish().await;
}

#[tokio::test]
async fn failed_confirmation_returns_to_executor() {
    let (f, id, _, _) = candidate(vec![cmd(&["git", "rev-parse", "no-such-ref"])]).await;
    sqlx::query("UPDATE run_attempts SET effort = 'low' WHERE id = ?")
        .bind(id.as_str())
        .execute(f.storage.reader())
        .await
        .unwrap();
    f.core.runs().recover().await.unwrap();
    results(&f, &id, 1).await;
    for _ in 0..300 {
        let attempts = f.core.runs().attempts(&f.workflow, 1, 3).await.unwrap();
        if attempts.len() > 1 {
            assert_eq!(attempts[0].origin, core_runs::AttemptOrigin::Checks);
            assert_eq!(attempts[0].thread_id, attempts[1].thread_id);
            assert_eq!(attempts[0].harness, attempts[1].harness);
            assert_eq!(attempts[0].model, attempts[1].model);
            assert_eq!(attempts[0].effort.as_deref(), Some("low"));
            let addition = attempts[0].addition.as_deref().unwrap();
            assert!(addition.contains("did not all pass"), "{addition}");
            assert!(addition.contains("\nc1 failed (exit 128)"), "{addition}");
            assert!(addition.contains("no-such-ref"), "{addition}");
            f.finish().await;
            return;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    panic!("failed confirmation never returned to the executor");
}

#[tokio::test]
async fn restart_sends_an_unsent_notice() {
    let (f, id, _, _) = candidate(vec![cmd(&["git", "--version"])]).await;
    let planner = prepare_planner(&f).await;
    sqlx::query(
        "UPDATE run_checks SET outcome='passed',output='confirmed before restart',
        started_at='2026-11-14T00:00:00Z',finished_at='2026-11-14T00:00:01Z'
        WHERE attempt_id=?",
    )
    .bind(id.as_str())
    .execute(f.storage.reader())
    .await
    .unwrap();
    f.core.runs().recover().await.unwrap();
    tokio::time::sleep(Duration::from_secs(2)).await;
    let rows = results(&f, &id, 1).await;
    assert_eq!(rows[0].output.as_deref(), Some("confirmed before restart"));
    wait_notice(&f, &id).await;
    sqlx::query("UPDATE run_attempts SET notice_sent_at=NULL WHERE id=?")
        .bind(id.as_str())
        .execute(f.storage.reader())
        .await
        .unwrap();
    f.core.runs().recover().await.unwrap();
    wait_notice(&f, &id).await;
    assert_eq!(f.core.turns().queued(&planner).await.unwrap().len(), 1);
    f.finish().await;
}

#[tokio::test]
async fn restart_sends_no_notice_from_before_notice_tracking() {
    let (f, id, _, _) = candidate(vec![cmd(&["git", "--version"])]).await;
    let planner = prepare_planner(&f).await;
    // The database as an upgrade finds it: this attempt committed before the migration that
    // added notice_sent_at, and the migration after it has not run yet.
    for sql in [
        "UPDATE _sqlx_migrations SET installed_on='2999-01-01 00:00:00' WHERE version=64",
        "DELETE FROM _sqlx_migrations WHERE version=68",
    ] {
        sqlx::query(sql).execute(f.storage.reader()).await.unwrap();
    }
    let upgraded = Storage::open(&f._tmp.path().join("s.sqlite3"))
        .await
        .unwrap();
    upgraded.close().await.unwrap();
    let sent: Option<String> =
        sqlx::query_scalar("SELECT notice_sent_at FROM run_attempts WHERE id=?")
            .bind(id.as_str())
            .fetch_one(f.storage.reader())
            .await
            .unwrap();
    assert_eq!(sent.as_deref(), Some("2999-01-01T00:00:00Z"));
    f.core.runs().recover().await.unwrap();
    tokio::time::sleep(Duration::from_secs(2)).await;
    let rows = f.core.runs().check_results(&id).await.unwrap();
    assert!(rows.iter().all(|row| row.outcome.is_none()), "{rows:?}");
    assert!(f.core.turns().queued(&planner).await.unwrap().is_empty());
    f.finish().await;
}

pub(super) async fn prepare_planner(f: &Fixture) -> ThreadId {
    planner_turn(f, "judge", "claude-code", "fake-large").await;
    let thread = planner_grant(f).await.thread_id.unwrap();
    let runtime = shadows_core::testing::Runtime::start(f.storage.clone())
        .await
        .unwrap();
    shadows_core::Turns::create_pending_operation_for_test(
        &f.storage,
        &thread,
        runtime.instance_id(),
    )
    .await
    .unwrap();
    thread
}

async fn wait_notice(f: &Fixture, id: &AttemptId) {
    for _ in 0..600 {
        let sent: Option<String> =
            sqlx::query_scalar("SELECT notice_sent_at FROM run_attempts WHERE id=?")
                .bind(id.as_str())
                .fetch_one(f.storage.reader())
                .await
                .unwrap();
        if sent.is_some() {
            return;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    panic!("notice was not durably queued");
}

async fn second_candidate(f: &Fixture, run: Vec<String>, uses: bool) -> (AttemptId, Grant) {
    let (id, grant) = f.begin(5).await;
    let mut context = f
        .core
        .plans()
        .task_context(&grant, Some(&f.workflow), 5, None)
        .await
        .unwrap();
    context.task.contract = Some(
        serde_json::from_value(json!({
            "files":[{"path":"web/b.js","action":"create","purpose":"Consume the result"}],
            "checks":[{"id":"c1","run":run,"timeout_secs":5}],
            "uses":if uses {json!([{"source":{"kind":"task","target":{"kind":"local","task":3},
                "output":"o1"},"purpose":"Use the producer"}])} else {json!([])}
        }))
        .unwrap(),
    );
    sqlx::query(
        "INSERT INTO run_attempt_context (attempt_id,context_json,captured_at) VALUES (?,?,?)",
    )
    .bind(id.as_str())
    .bind(serde_json::to_string(&context).unwrap())
    .bind("2026-11-15T00:00:00Z")
    .execute(f.storage.reader())
    .await
    .unwrap();
    f.put("web/b.js", "export const count = 1;\n");
    let sha = f.commit(&["web/b.js"], "consumer");
    f.core
        .runs()
        .report(
            &grant,
            &sha,
            "Built consumer.".into(),
            serde_json::from_value(json!([{"id":"c1","outcome":"passed"}])).unwrap(),
        )
        .await
        .unwrap();
    (id, grant)
}

#[tokio::test]
async fn used_tasks_checks_run_too() {
    let (f, _, _, _) = candidate(vec![cmd(&["git", "--version"])]).await;
    let (consumer, _) = second_candidate(&f, cmd(&["git", "--version"]), true).await;
    f.core.runs().recover().await.unwrap();
    let rows = results(&f, &consumer, 2).await;
    assert_eq!(
        rows.iter().map(|r| r.for_task).collect::<Vec<_>>(),
        vec![5, 3]
    );
    assert!(rows.iter().all(|r| r.outcome == Some(CheckOutcome::Passed)));
    assert!(rows[1].reported.is_none() && rows[1].matches.is_none());
    f.finish().await;
}

#[tokio::test]
async fn checks_of_one_project_run_one_at_a_time() {
    let slow = cmd(&["git", "-c", "alias.pause=!sleep 2", "pause"]);
    let (f, producer, _, _) = candidate(vec![slow.clone()]).await;
    let (consumer, _) = second_candidate(&f, slow, false).await;
    let (a, b) = tokio::join!(f.core.runs().recover(), f.core.runs().recover());
    a.unwrap();
    b.unwrap();
    let one = results(&f, &producer, 1).await;
    let two = results(&f, &consumer, 1).await;
    assert!(one[0].finished_at <= two[0].started_at || two[0].finished_at <= one[0].started_at);
    f.finish().await;
}

#[tokio::test]
async fn quiet_is_checked_again_under_the_lock() {
    let slow = cmd(&["git", "-c", "alias.pause=!sleep 2", "pause"]);
    let (f, id, _, _) = candidate(vec![slow.clone()]).await;
    let (other, _) = second_candidate(&f, slow, false).await;
    f.core.runs().recover().await.unwrap();
    started(&f, &id).await;
    f.core.runs().recover().await.unwrap();
    let operation = recorded_turn(&f, &other, "became-busy").await;
    tokio::time::sleep(Duration::from_millis(2300)).await;
    assert!(
        f.core.runs().check_results(&other).await.unwrap()[0]
            .started_at
            .is_none()
    );
    end_turn(&f, &operation).await;
    assert_eq!(
        results(&f, &id, 1).await[0].outcome,
        Some(CheckOutcome::Passed)
    );
    assert_eq!(
        results(&f, &other, 1).await[0].outcome,
        Some(CheckOutcome::Passed)
    );
    f.finish().await;
}

#[tokio::test]
async fn restart_after_return_passed_notifies_once() {
    let (f, id, grant, sha) =
        candidate(vec![cmd(&["git", "config", "--get", "check.ready"])]).await;
    let planner = prepare_planner(&f).await;
    sqlx::query(
        "UPDATE run_checks SET outcome='failed',finished_at='2026-11-15T00:00:00Z'
        WHERE attempt_id=?",
    )
    .bind(id.as_str())
    .execute(f.storage.reader())
    .await
    .unwrap();
    let next = clone_attempt(&f, &id, "returned-and-passed", "checks").await;
    f.git(&["config", "check.ready", "yes"]);
    f.core
        .runs()
        .report(
            &grant,
            &sha,
            "The check is available now.".into(),
            serde_json::from_value(json!([{"id":"c1","outcome":"passed"}])).unwrap(),
        )
        .await
        .unwrap();
    f.core.runs().recover().await.unwrap();
    assert_eq!(
        results(&f, &next, 1).await[0].outcome,
        Some(CheckOutcome::Passed)
    );
    wait_notice(&f, &next).await;
    f.core.runs().recover().await.unwrap();
    tokio::time::sleep(Duration::from_millis(300)).await;
    let notices = f.core.turns().queued(&planner).await.unwrap();
    assert_eq!(notices.len(), 1);
    assert_eq!(
        notices[0].notice.as_ref().unwrap().card["attempt_id"],
        next.as_str()
    );
    f.finish().await;
}

#[tokio::test]
async fn restart_reruns_unfinished_checks_and_notifies_once() {
    let (f, id, _, _) = candidate(vec![cmd(&["git", "--version"])]).await;
    sqlx::query("UPDATE run_checks SET started_at='2026-11-14T00:00:00Z' WHERE attempt_id=?")
        .bind(id.as_str())
        .execute(f.storage.reader())
        .await
        .unwrap();
    f.core.runs().recover().await.unwrap();
    let rows = results(&f, &id, 1).await;
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].outcome, Some(CheckOutcome::Passed));
    assert_ne!(rows[0].started_at.as_deref(), Some("2026-11-14T00:00:00Z"));
    f.finish().await;
}

fn cmd(args: &[&str]) -> Vec<String> {
    args.iter().map(|s| s.to_string()).collect()
}

async fn recorded_turn(f: &Fixture, id: &AttemptId, name: &str) -> core_operations::OperationId {
    use shadows_core::testing::turn::{new_turn, turn_command};
    let attempt = f.core.runs().attempt(id).await.unwrap();
    let runtime = shadows_core::testing::Runtime::start(f.storage.clone())
        .await
        .unwrap();
    let settings = shadows_agent::TurnSettings {
        model: "fake-large".into(),
        mode: "auto".into(),
        effort: None,
    };
    let turn = new_turn(&attempt.thread_id, &runtime, "working", &settings);
    let op = shadows_core::Turns::start_turn_for_test(
        &f.storage,
        &turn_command(name, &attempt.thread_id, "working", &settings),
        turn,
    )
    .await
    .unwrap();
    op.operation_id
}

async fn end_turn(f: &Fixture, operation: &core_operations::OperationId) {
    shadows_core::Turns::mark_operation_failed_for_test(
        &f.storage,
        operation,
        shadows_core::testing::FailureStage::Prepare,
        "fixture ended",
    )
    .await
    .unwrap();
}

async fn started(f: &Fixture, id: &AttemptId) -> String {
    for _ in 0..600 {
        let rows = f.core.runs().check_results(id).await.unwrap();
        if let Some(start) = rows.first().and_then(|r| r.started_at.clone()) {
            return start;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    panic!("check did not start");
}

#[tokio::test]
async fn checks_wait_for_a_quiet_project() {
    let (f, id, _, _) = candidate(vec![cmd(&["git", "--version"])]).await;
    let (other, _) = f.begin(5).await;
    sqlx::query(
        "UPDATE run_attempts SET state='committed',ended_at='2026-11-15T00:00:00Z' WHERE id=?",
    )
    .bind(other.as_str())
    .execute(f.storage.reader())
    .await
    .unwrap();
    let operation = recorded_turn(&f, &other, "busy").await;
    f.core.runs().recover().await.unwrap();
    tokio::time::sleep(Duration::from_millis(1200)).await;
    assert!(
        f.core.runs().check_results(&id).await.unwrap()[0]
            .started_at
            .is_none()
    );
    end_turn(&f, &operation).await;
    assert_eq!(
        results(&f, &id, 1).await[0].outcome,
        Some(CheckOutcome::Passed)
    );
    f.finish().await;
}

#[tokio::test]
async fn stopped_attempts_leftover_is_inconclusive() {
    let (f, id, _, _) = candidate(vec![cmd(&["git", "--version"])]).await;
    let (other, _) = f.begin(5).await;
    f.core.runs().stop(&other).await.unwrap();
    f.put("web/unfinished.go", "package unfinished\n");
    f.core.runs().recover().await.unwrap();
    let rows = results(&f, &id, 1).await;
    assert_eq!(rows[0].outcome, Some(CheckOutcome::Inconclusive));
    assert!(
        rows[0]
            .output
            .as_deref()
            .unwrap()
            .contains("web/unfinished.go")
    );
    f.finish().await;
}

#[tokio::test]
async fn another_attempts_leftover_goes_to_the_planner_without_a_return() {
    let (f, id, _, _) = candidate(vec![cmd(&["git", "--version"])]).await;
    let planner = prepare_planner(&f).await;
    let (other, _) = f.begin(5).await;
    f.core.runs().stop(&other).await.unwrap();
    f.put("web/unfinished.go", "package unfinished\n");
    f.core.runs().recover().await.unwrap();
    let rows = results(&f, &id, 1).await;
    assert_eq!(rows[0].outcome, Some(CheckOutcome::Inconclusive));
    let output = rows[0].output.as_deref().unwrap();
    assert!(output.contains("web/unfinished.go"), "{output}");
    assert!(output.contains("another attempt's scope"), "{output}");
    wait_notice(&f, &id).await;
    let attempts = f.core.runs().attempts(&f.workflow, 1, 3).await.unwrap();
    assert_eq!(
        attempts.len(),
        1,
        "no checks return for another attempt's files"
    );
    let notices = f.core.turns().queued(&planner).await.unwrap();
    assert!(
        notices
            .iter()
            .any(|n| n.notice.as_ref().unwrap().card["attempt_id"] == id.as_str()),
        "the Planner is told of the attempt"
    );
    f.finish().await;
}

#[tokio::test]
async fn head_moved_during_check_is_inconclusive() {
    let (f, id, _, _) = candidate(vec![cmd(&["git", "-c", "alias.pause=!sleep 2", "pause"])]).await;
    f.core.runs().recover().await.unwrap();
    started(&f, &id).await;
    f.put("personal.txt", "a committed note\n");
    f.commit(&["personal.txt"], "person");
    let rows = results(&f, &id, 1).await;
    assert_eq!(rows[0].outcome, Some(CheckOutcome::Inconclusive));
    assert_ne!(rows[0].head_before, rows[0].head_after);
    let moved = format!(
        "the check moved HEAD from {} to {}",
        rows[0].head_before.as_deref().unwrap(),
        rows[0].head_after.as_deref().unwrap()
    );
    assert!(rows[0].output.as_deref().unwrap().contains(&moved));
    f.finish().await;
}

#[tokio::test]
async fn file_the_check_wrote_is_named() {
    let (f, id, _, _) = candidate(vec![cmd(&[
        "git",
        "-c",
        "alias.write=!echo made > made-by-check.txt",
        "write",
    ])])
    .await;
    f.core.runs().recover().await.unwrap();
    let rows = results(&f, &id, 1).await;
    assert_eq!(rows[0].outcome, Some(CheckOutcome::Inconclusive));
    assert_eq!(rows[0].exit_code, Some(0));
    let output = rows[0].output.as_deref().unwrap();
    assert!(
        output.contains("the check changed files in the working tree: made-by-check.txt"),
        "{output}"
    );
    f.finish().await;
}

#[tokio::test]
async fn content_change_during_check_is_inconclusive() {
    let (f, id, _, _) = candidate(vec![cmd(&["git", "-c", "alias.pause=!sleep 2", "pause"])]).await;
    f.put("personal.txt", "before\n");
    f.core.runs().recover().await.unwrap();
    started(&f, &id).await;
    tokio::time::sleep(Duration::from_millis(400)).await;
    f.put("personal.txt", "after\n");
    assert_eq!(
        results(&f, &id, 1).await[0].outcome,
        Some(CheckOutcome::Inconclusive)
    );
    f.finish().await;
}

async fn overlap_case(finish: bool) {
    let (f, id, _, _) = candidate(vec![cmd(&["git", "-c", "alias.pause=!sleep 2", "pause"])]).await;
    f.core.runs().recover().await.unwrap();
    let first = started(&f, &id).await;
    let (other, _) = f.begin(5).await;
    let operation = recorded_turn(&f, &other, "overlap").await;
    if !finish {
        tokio::time::sleep(Duration::from_millis(2300)).await;
    }
    end_turn(&f, &operation).await;
    sqlx::query(
        "UPDATE run_attempts SET state='stopped',ended_at='2026-11-15T00:00:00Z',
                 released_at=? WHERE id=?",
    )
    .bind(shadows_core::testing::Storage::now())
    .bind(other.as_str())
    .execute(f.storage.reader())
    .await
    .unwrap();
    let rows = results(&f, &id, 1).await;
    assert_eq!(rows[0].outcome, Some(CheckOutcome::Passed));
    assert_ne!(rows[0].started_at.as_deref(), Some(first.as_str()));
    f.finish().await;
}

#[tokio::test]
async fn turn_during_check_reruns_it() {
    overlap_case(false).await;
}

#[tokio::test]
async fn turn_begun_and_ended_during_check_reruns_it() {
    overlap_case(true).await;
}

#[tokio::test]
async fn two_returns_then_planner() {
    let (f, id, _, sha) = candidate(vec![cmd(&["git", "rev-parse", "no-such-ref"])]).await;
    let second = clone_attempt(&f, &id, "check-one", "checks").await;
    sqlx::query(
        "UPDATE run_attempts SET state='committed',ended_at='2026-11-15T00:00:00Z',
                     commit_sha=? WHERE id=?",
    )
    .bind(&sha)
    .bind(second.as_str())
    .execute(f.storage.reader())
    .await
    .unwrap();
    let third = clone_attempt(&f, &second, "check-two", "checks").await;
    sqlx::query(
        "UPDATE run_attempts SET state='committed',ended_at='2026-11-15T00:00:00Z',
                     commit_sha=? WHERE id=?",
    )
    .bind(&sha)
    .bind(third.as_str())
    .execute(f.storage.reader())
    .await
    .unwrap();
    f.core.runs().recover().await.unwrap();
    results(&f, &third, 1).await;
    tokio::time::sleep(Duration::from_secs(1)).await;
    assert_eq!(
        f.core
            .runs()
            .attempts(&f.workflow, 1, 3)
            .await
            .unwrap()
            .len(),
        3
    );
    f.finish().await;
}

#[tokio::test]
async fn returns_are_not_additions() {
    let (f, id, _, sha) = candidate(vec![cmd(&["git", "--version"])]).await;
    let mut previous = id;
    for n in 1..=3 {
        let next = clone_attempt(&f, &previous, &format!("return-{n}"), "checks").await;
        sqlx::query(
            "UPDATE run_attempts SET state='committed',ended_at='2026-11-15T00:00:00Z',
                     commit_sha=? WHERE id=?",
        )
        .bind(&sha)
        .bind(next.as_str())
        .execute(f.storage.reader())
        .await
        .unwrap();
        previous = next;
    }
    let grant = planner_grant(&f).await;
    let next = f
        .core
        .runs()
        .amend(&grant, &f.workflow, 3, "Adjust the error message.".into())
        .await
        .unwrap();
    assert_eq!(next.origin, core_runs::AttemptOrigin::Planner);
    assert_eq!(next.number, 5);
    f.finish().await;
}

#[tokio::test]
async fn repeated_return_answers_the_same_attempt() {
    let (f, id, _, _) = candidate(vec![cmd(&["git", "rev-parse", "no-such-ref"])]).await;
    let runs = f.core.runs();
    let (a, b) = tokio::join!(runs.recover(), runs.recover());
    a.unwrap();
    b.unwrap();
    results(&f, &id, 1).await;
    for _ in 0..400 {
        if runs.attempts(&f.workflow, 1, 3).await.unwrap().len() > 1 {
            break;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    let attempts = runs.attempts(&f.workflow, 1, 3).await.unwrap();
    assert_eq!(
        attempts
            .iter()
            .filter(|a| a.origin == core_runs::AttemptOrigin::Checks)
            .count(),
        1
    );
    f.finish().await;
}

#[tokio::test]
async fn restart_during_return_sends_no_old_notice() {
    let (f, id, _, _) = candidate(vec![cmd(&["git", "--version"])]).await;
    clone_attempt(&f, &id, "in-progress-return", "checks").await;
    f.core.runs().recover().await.unwrap();
    tokio::time::sleep(Duration::from_secs(1)).await;
    let sent: Option<String> =
        sqlx::query_scalar("SELECT notice_sent_at FROM run_attempts WHERE id=?")
            .bind(id.as_str())
            .fetch_one(f.storage.reader())
            .await
            .unwrap();
    assert!(sent.is_none());
    assert!(
        f.core.runs().check_results(&id).await.unwrap()[0]
            .started_at
            .is_none()
    );
    f.finish().await;
}

async fn results(f: &Fixture, id: &AttemptId, count: usize) -> Vec<CheckResult> {
    for _ in 0..600 {
        let results = f.core.runs().check_results(id).await.unwrap();
        if results.len() == count && results.iter().all(|r| r.finished_at.is_some()) {
            return results;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    panic!("checks did not finish");
}

#[tokio::test]
async fn passed_and_failed_checks_are_recorded() {
    let (f, id, _, sha) = candidate(vec![
        cmd(&["git", "--version"]),
        cmd(&["git", "rev-parse", "no-such-ref"]),
    ])
    .await;
    f.core.runs().recover().await.unwrap();
    let rows = results(&f, &id, 2).await;
    assert_eq!(rows[0].outcome, Some(CheckOutcome::Passed));
    assert_eq!(rows[1].outcome, Some(CheckOutcome::Failed));
    assert_eq!(rows[0].head_before.as_deref(), Some(sha.as_str()));
    assert_eq!(rows[0].exit_code, Some(0));
    assert!(rows[0].output.as_deref().unwrap().contains("git version"));
    f.finish().await;
}

#[tokio::test]
async fn mismatch_with_report_is_marked() {
    let (f, id, _, _) = candidate(vec![cmd(&["git", "rev-parse", "no-such-ref"])]).await;
    f.core.runs().recover().await.unwrap();
    assert_eq!(results(&f, &id, 1).await[0].matches, Some(false));
    f.finish().await;
}

#[tokio::test]
async fn missing_program_is_not_run() {
    let (f, id, _, _) = candidate(vec![cmd(&["shadows-nonexistent-check-program"])]).await;
    f.core.runs().recover().await.unwrap();
    let rows = results(&f, &id, 1).await;
    assert_eq!(rows[0].outcome, Some(CheckOutcome::NotRun));
    assert!(rows[0].exit_code.is_none());
    f.finish().await;
}

#[tokio::test]
async fn uncommitted_change_in_writes_is_inconclusive() {
    let (f, id, _, _) = candidate(vec![cmd(&["git", "--version"])]).await;
    f.put("backend/app/a.php", "uncommitted\n");
    f.core.runs().recover().await.unwrap();
    let rows = results(&f, &id, 1).await;
    assert_eq!(rows[0].outcome, Some(CheckOutcome::Inconclusive));
    assert!(rows[0].exit_code.is_none());
    assert!(
        rows[0]
            .output
            .as_deref()
            .unwrap()
            .contains("backend/app/a.php")
    );
    f.finish().await;
}

#[tokio::test]
async fn person_files_do_not_count() {
    let (f, id, _, _) = candidate(vec![cmd(&["git", "--version"])]).await;
    f.put("personal.txt", "notes\n");
    f.core.runs().recover().await.unwrap();
    assert_eq!(
        results(&f, &id, 1).await[0].outcome,
        Some(CheckOutcome::Passed)
    );
    f.finish().await;
}

async fn confirmed(f: &Fixture, id: &AttemptId, outcome: &str) {
    sqlx::query(
        "UPDATE run_checks SET outcome=?,finished_at='2026-11-15T00:00:00Z'
        WHERE attempt_id=?",
    )
    .bind(outcome)
    .bind(id.as_str())
    .execute(f.storage.reader())
    .await
    .unwrap();
}

#[tokio::test]
async fn done_over_failed_check_needs_reason() {
    let (f, id, _, _) = candidate(vec![cmd(&["git", "--version"])]).await;
    confirmed(&f, &id, "failed").await;
    let error = f
        .core
        .runs()
        .mark_done("judge".into(), &f.workflow, 1, 3, true, None)
        .await
        .unwrap_err();
    assert_eq!(error.code(), "checks_not_passed");
    assert!(!f.core.runs().task(&f.workflow, 1, 3).await.unwrap().done);
    f.finish().await;
}

#[tokio::test]
async fn done_before_results_needs_reason() {
    let (f, _, _, _) = candidate(vec![cmd(&["git", "--version"])]).await;
    let error = f
        .core
        .runs()
        .mark_done("judge".into(), &f.workflow, 1, 3, true, None)
        .await
        .unwrap_err();
    assert_eq!(error.code(), "checks_not_passed");
    f.finish().await;
}

#[tokio::test]
async fn done_reason_is_recorded() {
    let (f, id, _, _) = candidate(vec![cmd(&["git", "--version"])]).await;
    confirmed(&f, &id, "not_run").await;
    let result = f
        .core
        .runs()
        .mark_done(
            "judge".into(),
            &f.workflow,
            1,
            3,
            true,
            Some("  Tool unavailable; checked manually.  ".into()),
        )
        .await
        .unwrap();
    assert!(result.done);
    assert_eq!(
        result.done_reason.as_deref(),
        Some("Tool unavailable; checked manually.")
    );
    let replay = f
        .core
        .runs()
        .mark_done(
            "judge".into(),
            &f.workflow,
            1,
            3,
            true,
            Some("Tool unavailable; checked manually.".into()),
        )
        .await
        .unwrap();
    assert_eq!(replay.done_reason, result.done_reason);
    let conflict = f
        .core
        .runs()
        .mark_done(
            "judge".into(),
            &f.workflow,
            1,
            3,
            true,
            Some("Another reason".into()),
        )
        .await
        .unwrap_err();
    assert_eq!(conflict.code(), "command_conflict");
    f.finish().await;
}

#[tokio::test]
async fn done_again_after_undo_needs_reason() {
    let (f, id, _, _) = candidate(vec![cmd(&["git", "--version"])]).await;
    confirmed(&f, &id, "failed").await;
    f.core
        .runs()
        .mark_done(
            "judge".into(),
            &f.workflow,
            1,
            3,
            true,
            Some("Known infrastructure failure".into()),
        )
        .await
        .unwrap();
    f.core
        .runs()
        .mark_done("undo".into(), &f.workflow, 1, 3, false, None)
        .await
        .unwrap();
    let error = f
        .core
        .runs()
        .mark_done("again".into(), &f.workflow, 1, 3, true, None)
        .await
        .unwrap_err();
    assert_eq!(error.code(), "checks_not_passed");
    f.finish().await;
}

#[tokio::test]
async fn done_replay_answers_after_checks_change() {
    let (f, id, _, _) = candidate(vec![cmd(&["git", "--version"])]).await;
    confirmed(&f, &id, "passed").await;
    f.core
        .runs()
        .mark_done("judge".into(), &f.workflow, 1, 3, true, None)
        .await
        .unwrap();
    confirmed(&f, &id, "failed").await;
    assert!(
        f.core
            .runs()
            .mark_done("judge".into(), &f.workflow, 1, 3, true, None)
            .await
            .unwrap()
            .done
    );
    f.finish().await;
}

#[tokio::test]
async fn replayed_done_does_not_release_a_newer_attempt() {
    let (f, id, _, sha) = candidate(vec![cmd(&["git", "--version"])]).await;
    confirmed(&f, &id, "passed").await;
    f.core
        .runs()
        .mark_done("judge".into(), &f.workflow, 1, 3, true, None)
        .await
        .unwrap();
    f.core
        .runs()
        .mark_done("undo".into(), &f.workflow, 1, 3, false, None)
        .await
        .unwrap();
    let next = clone_attempt(&f, &id, "newer", "start").await;
    sqlx::query(
        "UPDATE run_attempts SET state='committed',commit_sha=?,
        ended_at='2026-11-15T00:00:01Z' WHERE id=?",
    )
    .bind(sha)
    .bind(next.as_str())
    .execute(f.storage.reader())
    .await
    .unwrap();
    let answer = f
        .core
        .runs()
        .mark_done("judge".into(), &f.workflow, 1, 3, true, None)
        .await
        .unwrap();
    assert!(!answer.done);
    let released: Option<String> =
        sqlx::query_scalar("SELECT released_at FROM run_attempts WHERE id=?")
            .bind(next.as_str())
            .fetch_one(f.storage.reader())
            .await
            .unwrap();
    assert!(
        released.is_none(),
        "a replay must not release the newer attempt"
    );
    f.finish().await;
}

#[tokio::test]
async fn committed_notice_replay_keeps_planner_settings() {
    let (f, id, _, _) = candidate(vec![cmd(&["git", "--version"])]).await;
    let planner = prepare_planner(&f).await;
    confirmed(&f, &id, "passed").await;
    f.core.runs().recover().await.unwrap();
    wait_notice(&f, &id).await;
    for op in f.core.turns().operations(&planner).await.unwrap() {
        if op.finished_at.is_none() {
            shadows_core::Turns::mark_operation_failed_for_test(
                &f.storage,
                &op.id,
                shadows_core::testing::FailureStage::Prepare,
                "recorded only",
            )
            .await
            .unwrap();
        }
    }
    planner_turn(&f, "settings-changed", "claude-code", "fake-small").await;
    let runtime = shadows_core::testing::Runtime::start(f.storage.clone())
        .await
        .unwrap();
    shadows_core::Turns::create_pending_operation_for_test(
        &f.storage,
        &planner,
        runtime.instance_id(),
    )
    .await
    .unwrap();
    sqlx::query("UPDATE run_attempts SET notice_sent_at=NULL WHERE id=?")
        .bind(id.as_str())
        .execute(f.storage.reader())
        .await
        .unwrap();
    f.core.runs().recover().await.unwrap();
    wait_notice(&f, &id).await;
    assert_eq!(f.core.turns().queued(&planner).await.unwrap().len(), 1);
    f.finish().await;
}

#[tokio::test]
async fn done_without_reason_replays_a_fingerprint_with_no_reason_key() {
    let (f, id, _, _) = candidate(vec![cmd(&["git", "--version"])]).await;
    confirmed(&f, &id, "passed").await;
    f.core
        .runs()
        .mark_done("earlier".into(), &f.workflow, 1, 3, true, None)
        .await
        .unwrap();
    let old = CommandContext::fingerprint(
        "run.mark_done",
        &serde_json::json!({
            "workflow":f.workflow,"version":1,"task":3,"done":true
        }),
    );
    sqlx::query(
        "UPDATE command_record SET request_fingerprint=?
        WHERE command_scope_kind='run.mark_done' AND command_id='earlier'",
    )
    .bind(old)
    .execute(f.storage.reader())
    .await
    .unwrap();
    assert!(
        f.core
            .runs()
            .mark_done("earlier".into(), &f.workflow, 1, 3, true, None)
            .await
            .unwrap()
            .done
    );
    f.finish().await;
}

#[tokio::test]
async fn done_replay_finishes_only_its_original_scope_release() {
    let (f, id, _, sha) = candidate(vec![cmd(&["git", "--version"])]).await;
    confirmed(&f, &id, "passed").await;
    f.core
        .runs()
        .mark_done("judge".into(), &f.workflow, 1, 3, true, None)
        .await
        .unwrap();
    sqlx::query("UPDATE run_attempts SET released_at=NULL WHERE id=?")
        .bind(id.as_str())
        .execute(f.storage.reader())
        .await
        .unwrap();
    f.core
        .runs()
        .mark_done("judge".into(), &f.workflow, 1, 3, true, None)
        .await
        .unwrap();
    let released: Option<String> =
        sqlx::query_scalar("SELECT released_at FROM run_attempts WHERE id=?")
            .bind(id.as_str())
            .fetch_one(f.storage.reader())
            .await
            .unwrap();
    assert!(
        released.is_some(),
        "a replay must finish its original pending release"
    );
    let next = clone_attempt(&f, &id, "newer", "start").await;
    sqlx::query(
        "UPDATE run_attempts SET state='committed',commit_sha=?,
        ended_at='2026-11-15T00:00:01Z' WHERE id=?",
    )
    .bind(sha)
    .bind(next.as_str())
    .execute(f.storage.reader())
    .await
    .unwrap();
    f.core
        .runs()
        .mark_done("judge".into(), &f.workflow, 1, 3, true, None)
        .await
        .unwrap();
    let released: Option<String> =
        sqlx::query_scalar("SELECT released_at FROM run_attempts WHERE id=?")
            .bind(next.as_str())
            .fetch_one(f.storage.reader())
            .await
            .unwrap();
    assert!(
        released.is_none(),
        "the mark still names its original attempt"
    );
    f.finish().await;
}

#[tokio::test]
async fn notice_lists_each_check_on_its_line() {
    let (f, id, _, _) = candidate(vec![cmd(&["git", "--version"])]).await;
    let planner = prepare_planner(&f).await;
    confirmed(&f, &id, "passed").await;
    sqlx::query("UPDATE run_checks SET reported='failed' WHERE attempt_id=?")
        .bind(id.as_str())
        .execute(f.storage.reader())
        .await
        .unwrap();
    f.core.runs().recover().await.unwrap();
    wait_notice(&f, &id).await;
    let queued = f.core.turns().queued(&planner).await.unwrap();
    let prompt = &queued[0].prompt;
    assert!(
        prompt.contains("Checks:\nc1 passed; the executor reported failed\n"),
        "{prompt}"
    );
    assert!(!prompt.contains("Some("), "{prompt}");
    f.finish().await;
}

#[tokio::test]
async fn return_waits_for_a_turn_on_its_thread() {
    let (f, id, _, _) = candidate(vec![cmd(&["git", "--version"])]).await;
    confirmed(&f, &id, "failed").await;
    let operation = recorded_turn(&f, &id, "question").await;
    f.core.runs().recover().await.unwrap();
    tokio::time::sleep(Duration::from_millis(1500)).await;
    assert_eq!(
        f.core
            .runs()
            .attempts(&f.workflow, 1, 3)
            .await
            .unwrap()
            .len(),
        1,
        "a return must not start while a turn runs on its thread"
    );
    end_turn(&f, &operation).await;
    for _ in 0..300 {
        let attempts = f.core.runs().attempts(&f.workflow, 1, 3).await.unwrap();
        if attempts.len() > 1 {
            assert_eq!(attempts[0].origin, core_runs::AttemptOrigin::Checks);
            f.finish().await;
            return;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    panic!("the return never followed the turn's end");
}
