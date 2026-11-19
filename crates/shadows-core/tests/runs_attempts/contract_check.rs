//! The `contract` check: Shadows compares a committed attempt's code with the
//! symbols its contract promised, through `CodeRecord`, over recorded answers
//! (and one real `pyright-langserver`).
use super::checks::prepare_planner;
use super::{Fixture, command, fixture_with, task};
use code_record::RecordedAnswer;
use core_runs::{AttemptId, CheckOutcome, CheckResult};
use serde_json::{Value, json};
use shadows_core::{CodebaseConfig, PlanOp, Writer};
use std::time::Duration;

const FILE: &str = "backend/app/service.py";

/// Task 3 of the fixture with `checks` and the symbols `names` promised in
/// `FILE`, which it creates; attempted and committed with `text` as `FILE`.
async fn candidate(checks: Vec<Value>, names: &[&str], text: &str) -> (Fixture, AttemptId) {
    let f = fixture_with(false, None).await;
    let PlanOp::TaskAdd { task: mut content } = task(3, "backend/app/**") else {
        unreachable!()
    };
    let symbols: Vec<_> = names
        .iter()
        .enumerate()
        .map(|(i, name)| {
            json!({"id":format!("s{}",i+1),"path":FILE,"kind":"function","name":name,
                   "returns":{"type_name":"None","meaning":"Nothing"},
                   "behavior":["Does its work"]})
        })
        .collect();
    content.contract = Some(
        serde_json::from_value(json!({
            "files":[{"path":FILE,"action":"create","purpose":"The service"}],
            "symbols":symbols,
            "checks":checks
        }))
        .unwrap(),
    );
    f.core
        .plans()
        .edit_plan_for_test(
            &command("contract"),
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
        .approve_plan_for_test(&command("approve-contract"), &f.workflow, 2)
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
    .bind("2026-11-19T00:00:00Z")
    .execute(f.storage.reader())
    .await
    .unwrap();
    f.put(FILE, text);
    let sha = f.commit(&[FILE], "build");
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
    (f, id)
}

/// A `documentSymbol` answer holding one top-level function per name.
fn functions(names: &[&str]) -> RecordedAnswer {
    RecordedAnswer::Symbols(json!(
        names
            .iter()
            .map(|n| json!({"name":n,"kind":12,"range":{},"selectionRange":{}}))
            .collect::<Vec<_>>()
    ))
}

/// The attempt's check rows once `count` of them are finished.
async fn results(f: &Fixture, id: &AttemptId, count: usize) -> Vec<CheckResult> {
    for _ in 0..2400 {
        let results = f.core.runs().check_results(id).await.unwrap();
        if results.len() == count && results.iter().all(|r| r.finished_at.is_some()) {
            return results;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    panic!("checks did not finish");
}

/// The newest attempt at task 3, once a return has made a second one.
async fn returned(f: &Fixture) -> core_runs::Attempt {
    for _ in 0..300 {
        let attempts = f.core.runs().attempts(&f.workflow, 1, 3).await.unwrap();
        if attempts.len() > 1 {
            return attempts.into_iter().next().unwrap();
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    panic!("the contract check never returned to the executor");
}

/// The Planner's queued notice prompt about attempt `id`.
async fn notice(f: &Fixture, planner: &shadows_core::ThreadId, id: &AttemptId) -> String {
    for _ in 0..600 {
        let queued = f.core.turns().queued(planner).await.unwrap();
        if let Some(n) = queued
            .iter()
            .find(|n| n.notice.as_ref().unwrap().card["attempt_id"] == id.as_str())
        {
            return n.prompt.clone();
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    panic!("the Planner was not told");
}

#[tokio::test]
async fn a_task_with_no_checks_still_gets_its_contract_checked() {
    let (f, id) = candidate(vec![], &["login"], "def login():\n    pass\n").await;
    f.core
        .code_record()
        .record_answer_for_tests(FILE, functions(&["login"]));
    f.core.runs().recover().await.unwrap();
    let rows = results(&f, &id, 1).await;
    assert_eq!(rows[0].id, "contract");
    assert_eq!(rows[0].for_task, 3);
    assert!(rows[0].run.is_empty());
    assert_eq!(rows[0].outcome, Some(CheckOutcome::Passed));
    assert!(rows[0].head_after.is_some());
    let verdict = f.core.code_record().verdict(id.as_str()).await.unwrap();
    assert!(
        verdict.is_some(),
        "the verdict is stored under the attempt id"
    );
    f.finish().await;
}

#[tokio::test]
async fn a_missing_symbol_goes_back_to_the_executor_with_its_name() {
    let check = json!({"id":"c1","run":["git","--version"],"timeout_secs":5});
    let (f, id) = candidate(
        vec![check],
        &["login", "refresh_token"],
        "def login():\n    pass\n",
    )
    .await;
    f.core
        .code_record()
        .record_answer_for_tests(FILE, functions(&["login"]));
    f.core.runs().recover().await.unwrap();
    let rows = results(&f, &id, 2).await;
    assert_eq!(rows[0].id, "c1");
    assert_eq!(rows[1].id, "contract", "after the task's own checks");
    assert_eq!(rows[1].outcome, Some(CheckOutcome::Failed));
    let next = returned(&f).await;
    assert_eq!(next.origin, core_runs::AttemptOrigin::Checks);
    let addition = next.addition.unwrap();
    assert!(addition.contains("\ncontract failed"), "{addition}");
    assert!(
        addition.contains(&format!(
            "T3 promises refresh_token in {FILE}; the code does not have it."
        )),
        "{addition}"
    );
    f.finish().await;
}

#[tokio::test]
async fn a_long_verdict_keeps_its_first_missing_symbol() {
    let names: Vec<String> = (0..100).map(|i| format!("missing_{i:03}")).collect();
    let names: Vec<&str> = names.iter().map(String::as_str).collect();
    let (f, id) = candidate(vec![], &names, "def other():\n    pass\n").await;
    f.core
        .code_record()
        .record_answer_for_tests(FILE, functions(&["other"]));
    f.core.runs().recover().await.unwrap();
    let rows = results(&f, &id, 1).await;
    assert!(rows[0].by_shadows);
    assert!(rows[0].output.as_deref().unwrap().chars().count() > 4000);
    let addition = returned(&f).await.addition.unwrap();
    assert!(
        addition.contains(&format!(
            "T3 promises missing_000 in {FILE}; the code does not have it."
        )),
        "{addition}"
    );
    f.finish().await;
}

#[tokio::test]
async fn a_found_contract_passes_and_the_notice_says_so() {
    let (f, id) = candidate(vec![], &["login"], "def login():\n    pass\n").await;
    let planner = prepare_planner(&f).await;
    f.core
        .code_record()
        .record_answer_for_tests(FILE, functions(&["login"]));
    f.core.runs().recover().await.unwrap();
    let prompt = notice(&f, &planner, &id).await;
    assert!(prompt.contains("Checks:\ncontract passed\n"), "{prompt}");
    assert_eq!(
        f.core
            .runs()
            .attempts(&f.workflow, 1, 3)
            .await
            .unwrap()
            .len(),
        1
    );
    f.finish().await;
}

#[tokio::test]
async fn done_while_the_contract_check_runs_needs_a_reason() {
    let (f, id) = candidate(vec![], &["login"], "def login():\n    pass\n").await;
    // The row as `check_contract` leaves it while `verify` runs: started, not finished.
    sqlx::query(
        "INSERT INTO run_checks (attempt_id,for_task,check_id,ordinal,run_json,started_at)
         VALUES (?,3,'contract',0,'[]','2026-11-19T00:00:01Z')",
    )
    .bind(id.as_str())
    .execute(f.storage.reader())
    .await
    .unwrap();
    let error = f
        .core
        .runs()
        .mark_done("judge".into(), &f.workflow, 1, 3, true, None)
        .await
        .unwrap_err();
    assert_eq!(error.code(), "checks_not_passed");
    assert!(format!("{error:?}").contains("T3 contract"), "{error:?}");
    f.finish().await;
}

#[tokio::test]
async fn done_before_the_contract_check_starts_needs_a_reason() {
    // Committed and reported, its checks not confirmed yet: the due row does not exist.
    let (f, id) = candidate(vec![], &["login"], "def login():\n    pass\n").await;
    assert!(f.core.runs().check_results(&id).await.unwrap().is_empty());
    let error = f
        .core
        .runs()
        .mark_done("judge".into(), &f.workflow, 1, 3, true, None)
        .await
        .unwrap_err();
    assert_eq!(error.code(), "checks_not_passed");
    assert!(format!("{error:?}").contains("T3 contract"), "{error:?}");
    f.core
        .runs()
        .mark_done(
            "judge-why".into(),
            &f.workflow,
            1,
            3,
            true,
            Some("read".into()),
        )
        .await
        .unwrap();
    f.finish().await;
}

#[tokio::test]
async fn no_language_server_is_said_and_not_returned() {
    let (f, id) = candidate(vec![], &["login"], "def login():\n    pass\n").await;
    let planner = prepare_planner(&f).await;
    f.core
        .code_record()
        .record_answer_for_tests(FILE, RecordedAnswer::NoServer);
    f.core.runs().recover().await.unwrap();
    let rows = results(&f, &id, 1).await;
    assert_eq!(rows[0].outcome, Some(CheckOutcome::NotRun));
    let output = rows[0].output.as_deref().unwrap();
    assert!(
        output.contains(&format!("unchecked: {FILE} (no language server reads it).")),
        "{output}"
    );
    let prompt = notice(&f, &planner, &id).await;
    assert!(
        prompt.contains(&format!(
            "contract not_run\nunchecked: {FILE} (no language server reads it)."
        )),
        "{prompt}"
    );
    assert_eq!(
        f.core
            .runs()
            .attempts(&f.workflow, 1, 3)
            .await
            .unwrap()
            .len(),
        1,
        "a check that could not be run is not the executor's to fix"
    );
    f.core
        .runs()
        .mark_done("judge".into(), &f.workflow, 1, 3, true, None)
        .await
        .unwrap();
    f.finish().await;
}

#[tokio::test]
async fn a_check_named_contract_is_refused() {
    let f = fixture_with(false, None).await;
    let PlanOp::TaskAdd { task: mut content } = task(3, "backend/app/**") else {
        unreachable!()
    };
    content.contract = Some(
        serde_json::from_value(json!({
            "files":[{"path":FILE,"action":"create","purpose":"The service"}],
            "checks":[{"id":"contract","run":["git","--version"]}]
        }))
        .unwrap(),
    );
    let refused = f
        .core
        .plans()
        .edit_plan_for_test(
            &command("named-contract"),
            &Writer::Person,
            None,
            &f.workflow,
            1,
            &[PlanOp::TaskUpdate { task: content }],
        )
        .await
        .unwrap_err();
    let text = format!("{refused:?}");
    assert!(
        text.contains("the check id `contract` is Shadows' own"),
        "{text}"
    );
    f.finish().await;
}

#[tokio::test]
#[ignore = "needs pyright-langserver on PATH"]
async fn live_an_attempt_missing_a_promised_function_is_returned() {
    let (f, id) = candidate(
        vec![],
        &["login", "refresh_token"],
        "def login() -> None:\n    pass\n",
    )
    .await;
    f.core
        .codebase()
        .start(CodebaseConfig {
            name_index: false,
            ..CodebaseConfig::default()
        })
        .await
        .unwrap();
    f.core.runs().recover().await.unwrap();
    let rows = results(&f, &id, 1).await;
    assert_eq!(rows[0].outcome, Some(CheckOutcome::Failed), "{rows:?}");
    let next = returned(&f).await;
    let addition = next.addition.unwrap();
    assert!(
        addition.contains(&format!(
            "T3 promises refresh_token in {FILE}; the code does not have it."
        )),
        "{addition}"
    );
    assert!(!addition.contains("promises login"), "{addition}");
    f.core.codebase().shut_down_for_test().await;
    f.finish().await;
}

/// Task 3 promising `login`, checked with a recorded answer that has it, and
/// marked done once the Planner was told; returns the Planner's thread.
async fn done_with_contract() -> (Fixture, shadows_core::ThreadId) {
    let (f, id) = candidate(vec![], &["login"], "def login():\n    pass\n").await;
    let planner = prepare_planner(&f).await;
    f.core
        .code_record()
        .record_answer_for_tests(FILE, functions(&["login"]));
    f.core.runs().recover().await.unwrap();
    assert_eq!(
        results(&f, &id, 1).await[0].outcome,
        Some(CheckOutcome::Passed)
    );
    notice(&f, &planner, &id).await;
    f.core
        .runs()
        .mark_done("done-three".into(), &f.workflow, 1, 3, true, None)
        .await
        .unwrap();
    (f, planner)
}

/// Marks task 1 done under `command`, which rechecks the project's other done
/// tasks, and waits for that recheck to finish and deliver its notice.
async fn recheck(f: &Fixture, command: &str) {
    f.core
        .runs()
        .mark_done(command.into(), &f.workflow, 1, 1, true, None)
        .await
        .unwrap();
    for _ in 0..2400 {
        let pending: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM run_rechecks WHERE finished_at IS NULL
                 OR (notice_json IS NOT NULL AND notice_sent_at IS NULL)",
        )
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

/// The `contract` recheck outcomes of task 3, oldest batch first.
async fn contract_rechecks(f: &Fixture) -> Vec<String> {
    sqlx::query_scalar(
        "SELECT r.outcome FROM run_recheck_results r JOIN run_rechecks b ON b.id=r.recheck_id
         WHERE r.task=3 AND r.check_id='contract' ORDER BY b.created_at,b.id",
    )
    .fetch_all(f.storage.reader())
    .await
    .unwrap()
}

/// The recheck notices queued for the Planner.
async fn recheck_prompts(f: &Fixture, planner: &shadows_core::ThreadId) -> Vec<String> {
    f.core
        .turns()
        .queued(planner)
        .await
        .unwrap()
        .into_iter()
        .filter(|m| {
            m.notice
                .as_ref()
                .is_some_and(|n| n.card.get("recheck_id").is_some())
        })
        .map(|m| m.prompt)
        .collect()
}

#[tokio::test]
async fn a_later_task_removing_a_done_tasks_function_is_told_once() {
    let (f, planner) = done_with_contract().await;
    f.core
        .code_record()
        .record_answer_for_tests(FILE, functions(&[]));
    recheck(&f, "cause").await;
    recheck(&f, "cause-again").await;
    assert_eq!(contract_rechecks(&f).await, vec!["failed", "failed"]);
    let kept: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM code_record_verdict WHERE attempt_id LIKE 'recheck:%'",
    )
    .fetch_one(f.storage.reader())
    .await
    .unwrap();
    assert_eq!(
        kept, 1,
        "each recheck replaces the attempt's recheck verdict"
    );
    let prompts = recheck_prompts(&f, &planner).await;
    assert_eq!(prompts.len(), 1, "a failure already told is not told again");
    assert!(prompts[0].contains("T3 contract failed"), "{}", prompts[0]);
    assert!(
        prompts[0].contains(&format!(
            "T3 promises login in {FILE}; the code does not have it."
        )),
        "{}",
        prompts[0]
    );
    assert!(f.core.runs().task(&f.workflow, 1, 3).await.unwrap().done);
    f.finish().await;
}

#[tokio::test]
async fn a_not_run_recheck_never_hides_a_later_failure() {
    let (f, planner) = done_with_contract().await;
    f.core
        .code_record()
        .record_answer_for_tests(FILE, RecordedAnswer::NoServer);
    recheck(&f, "cause").await;
    assert!(
        recheck_prompts(&f, &planner).await.is_empty(),
        "a contract recheck that could not run is recorded, never told"
    );
    f.core
        .code_record()
        .record_answer_for_tests(FILE, functions(&[]));
    recheck(&f, "cause-again").await;
    assert_eq!(contract_rechecks(&f).await, vec!["not_run", "failed"]);
    let prompts = recheck_prompts(&f, &planner).await;
    assert_eq!(prompts.len(), 1);
    assert!(prompts[0].contains("T3 contract failed"), "{}", prompts[0]);
    f.finish().await;
}

#[tokio::test]
#[ignore = "needs pyright-langserver on PATH"]
async fn live_a_later_task_removing_a_done_tasks_function_is_told() {
    let (f, id) = candidate(vec![], &["login"], "def login() -> None:\n    pass\n").await;
    let planner = prepare_planner(&f).await;
    f.core
        .codebase()
        .start(CodebaseConfig {
            name_index: false,
            ..CodebaseConfig::default()
        })
        .await
        .unwrap();
    f.core.runs().recover().await.unwrap();
    let rows = results(&f, &id, 1).await;
    assert_eq!(rows[0].outcome, Some(CheckOutcome::Passed), "{rows:?}");
    notice(&f, &planner, &id).await;
    f.core
        .runs()
        .mark_done("done-three".into(), &f.workflow, 1, 3, true, None)
        .await
        .unwrap();
    f.put(FILE, "def logout() -> None:\n    pass\n");
    f.commit(&[FILE], "a later task removes login");
    recheck(&f, "cause").await;
    assert_eq!(contract_rechecks(&f).await, vec!["failed"]);
    let prompts = recheck_prompts(&f, &planner).await;
    assert_eq!(prompts.len(), 1);
    assert!(
        prompts[0].contains(&format!(
            "T3 promises login in {FILE}; the code does not have it."
        )),
        "{}",
        prompts[0]
    );
    f.core.codebase().shut_down_for_test().await;
    f.finish().await;
}

#[tokio::test]
async fn marking_done_writes_the_code_record_and_undone_removes_it() {
    let (f, id) = candidate(vec![], &["login", "logout"], "def login():\n    pass\n").await;
    f.core
        .code_record()
        .record_answer_for_tests(FILE, functions(&["login", "helper"]));
    f.core.runs().recover().await.unwrap();
    assert_eq!(
        results(&f, &id, 1).await[0].outcome,
        Some(CheckOutcome::Failed)
    );
    f.core
        .runs()
        .mark_done("record".into(), &f.workflow, 1, 3, true, Some("ok".into()))
        .await
        .unwrap();
    let rows = || async {
        let symbols: Vec<(String, String, String)> = sqlx::query_as(
            "SELECT symbol_id, name, state FROM code_record_symbol
             WHERE workflow_id = ? AND task = 3 ORDER BY symbol_id",
        )
        .bind(f.workflow.as_str())
        .fetch_all(f.storage.reader())
        .await
        .unwrap();
        let unpromised: Vec<String> =
            sqlx::query_scalar("SELECT name FROM code_record_unpromised WHERE task = 3")
                .fetch_all(f.storage.reader())
                .await
                .unwrap();
        (symbols, unpromised)
    };
    assert_eq!(
        rows().await,
        (
            vec![
                ("s1".into(), "login".into(), "found".into()),
                ("s2".into(), "logout".into(), "missing".into()),
            ],
            vec!["helper".to_string()],
        )
    );
    f.core
        .runs()
        .mark_done("undo".into(), &f.workflow, 1, 3, false, None)
        .await
        .unwrap();
    assert_eq!(rows().await, (vec![], vec![]));
    f.finish().await;
}
