use super::*;
use shadows_core::{Chooser, ExecutorChoice, ExecutorInput};

async fn fixture() -> Fixture {
    fixture_with(true, None).await
}

fn input(model: &str, effort: Option<&str>) -> ExecutorInput {
    ExecutorInput {
        harness: "claude-code".into(),
        model: model.into(),
        effort: effort.map(str::to_owned),
    }
}

async fn save(f: &Fixture, command: &str, effort: &str) -> ExecutorChoice {
    f.core
        .runs()
        .set_executor(
            command.into(),
            &f.workflow,
            1,
            3,
            input("fake-large", Some(effort)),
        )
        .await
        .unwrap()
}

async fn draft(f: &Fixture) -> WorkflowId {
    let plans = f.core.plans();
    let plan = plans.get(&f.workflow).await.unwrap();
    let grant = planner_grant(f).await;
    let writer = Writer::Planner {
        thread: grant.thread_id.unwrap(),
        grant: grant.id,
    };
    plans
        .start_draft_for_test(
            &command("next"),
            &writer,
            &f.project,
            Some(&plan.plan_id),
            Some(&f.workflow),
            None,
            Some("Correct the plan"),
            None,
            None,
            &[],
        )
        .await
        .unwrap()
        .workflow_id
}

async fn replace_task(f: &Fixture, draft: &WorkflowId) {
    let plans = f.core.plans();
    plans
        .edit_plan_for_test(
            &command("remove-5"),
            &Writer::Person,
            None,
            draft,
            0,
            &[PlanOp::TaskRemove { number: 5 }],
        )
        .await
        .unwrap();
    plans
        .edit_plan_for_test(
            &command("new-5"),
            &Writer::Person,
            None,
            draft,
            1,
            &[task(5, "other/**")],
        )
        .await
        .unwrap();
}

#[tokio::test]
async fn a_start_without_a_choice_is_refused() {
    let fixture = fixture().await;
    let result = fixture
        .core
        .runs()
        .start("no-choice".into(), &fixture.workflow, 1, 1)
        .await;
    assert!(matches!(result, Err(RunsError::NoExecutorChoice)));
    assert!(
        fixture
            .core
            .runs()
            .attempts(&fixture.workflow, 1, 1)
            .await
            .unwrap()
            .is_empty()
    );
    let page = fixture
        .core
        .runs()
        .task(&fixture.workflow, 1, 1)
        .await
        .unwrap();
    assert_eq!(page.start.code.as_deref(), Some("no_executor_choice"));
    fixture.finish().await;
}

#[tokio::test]
async fn the_task_page_says_no_choice() {
    let f = fixture().await;
    let grant = planner_grant(&f).await;
    let runs = f.core.runs();
    let page = runs.task(&f.workflow, 1, 1).await.unwrap();
    assert!(page.executor.is_none());
    assert_eq!(page.start.code.as_deref(), Some("no_executor_choice"));
    let group = runs.ready_for(&grant, &f.workflow).await.unwrap();
    assert!(group.tasks.iter().all(|ready| ready.task != 1));
    runs.set_executor(
        "one".into(),
        &f.workflow,
        1,
        1,
        input("fake-large", Some("low")),
    )
    .await
    .unwrap();
    let group = runs.ready_for(&grant, &f.workflow).await.unwrap();
    let one = group.tasks.iter().find(|ready| ready.task == 1).unwrap();
    assert_eq!(one.executor.as_ref().unwrap().model, "fake-large");
    f.finish().await;
}

#[tokio::test]
async fn a_start_runs_on_the_saved_choice() {
    let f = fixture().await;
    f.hold();
    let saved = save(&f, "saved", "low").await;
    let attempt = f
        .core
        .runs()
        .start("saved-start".into(), &f.workflow, 1, 3)
        .await
        .unwrap();
    assert_eq!(attempt.state, AttemptState::Running, "{attempt:?}");
    assert_eq!(attempt.harness, saved.harness);
    assert_eq!(attempt.model, saved.model);
    assert_eq!(attempt.effort, saved.effort);
    let offered = f.sessions.offered(&attempt.thread_id).await.unwrap();
    assert_eq!(offered.current.effort, saved.effort);
    f.finish().await;
}

#[tokio::test]
async fn a_replayed_older_save_keeps_the_latest() {
    let f = fixture().await;
    let grant = planner_grant(&f).await;
    let proposed = f
        .core
        .runs()
        .set_executor_for(&grant, &f.workflow, 3, input("fake-large", Some("high")))
        .await
        .unwrap();
    assert_eq!(proposed.chosen_by, Chooser::Planner);
    let a = save(&f, "a", "low").await;
    let b = save(&f, "b", "max").await;
    assert_eq!(save(&f, "a", "low").await, a);
    let task = f.core.runs().task(&f.workflow, 1, 3).await.unwrap();
    assert_eq!(task.executor, Some(b));
    assert_eq!(a.chosen_by, Chooser::Person);
    assert!(matches!(
        f.core
            .runs()
            .set_executor(
                "a".into(),
                &f.workflow,
                1,
                3,
                input("fake-large", Some("max"))
            )
            .await,
        Err(RunsError::CommandConflict)
    ));
    let events: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM durable_event WHERE kind='TaskExecutorChosen'")
            .fetch_one(f.storage.reader())
            .await
            .unwrap();
    assert_eq!(events, 3);
    f.finish().await;
}

#[tokio::test]
async fn a_model_with_efforts_needs_one() {
    let f = fixture().await;
    for (model, effort) in [
        ("fake-large", None),
        ("fake-small", Some("max")),
        ("unknown", None),
    ] {
        assert!(matches!(
            f.core
                .runs()
                .set_executor("invalid".into(), &f.workflow, 1, 3, input(model, effort))
                .await,
            Err(RunsError::ExecutorNotOffered(_))
        ));
    }
    assert!(
        f.core
            .runs()
            .task(&f.workflow, 1, 3)
            .await
            .unwrap()
            .executor
            .is_none()
    );
    f.finish().await;
}

#[tokio::test]
async fn a_replaced_task_starts_empty() {
    let f = fixture().await;
    let runs = f.core.runs();
    let old = runs
        .set_executor(
            "save-five".into(),
            &f.workflow,
            1,
            5,
            input("fake-large", Some("low")),
        )
        .await
        .unwrap();
    let version = draft(&f).await;
    assert_eq!(runs.task(&version, 2, 5).await.unwrap().executor, Some(old));
    replace_task(&f, &version).await;
    assert!(runs.task(&version, 2, 5).await.unwrap().executor.is_none());
    f.core
        .plans()
        .approve_plan_for_test(&command("approve-next"), &version, 2)
        .await
        .unwrap();
    assert!(matches!(
        runs.start("new-five".into(), &version, 2, 5).await,
        Err(RunsError::NoExecutorChoice)
    ));
    f.finish().await;
}

#[tokio::test]
async fn a_save_racing_a_replacement_is_refused() {
    let f = fixture().await;
    let version = draft(&f).await;
    let runs = f.core.runs().clone();
    let (reached, release) = runs.pause_next_save_for_test();
    let saved = tokio::spawn({
        let version = version.clone();
        async move {
            runs.set_executor(
                "old-five".into(),
                &version,
                2,
                5,
                input("fake-large", Some("low")),
            )
            .await
        }
    });
    reached.await.unwrap();
    replace_task(&f, &version).await;
    release.send(()).unwrap();
    assert!(matches!(saved.await.unwrap(), Err(RunsError::NotFound)));
    assert!(
        f.core
            .runs()
            .task(&version, 2, 5)
            .await
            .unwrap()
            .executor
            .is_none()
    );
    f.finish().await;
}

#[tokio::test]
async fn a_save_racing_a_new_version_is_refused() {
    let f = fixture().await;
    let runs = f.core.runs().clone();
    let (reached, release) = runs.pause_next_save_for_test();
    let save = tokio::spawn({
        let workflow = f.workflow.clone();
        async move {
            runs.set_executor(
                "old-save".into(),
                &workflow,
                1,
                3,
                input("fake-large", Some("low")),
            )
            .await
        }
    });
    reached.await.unwrap();
    let next = draft(&f).await;
    release.send(()).unwrap();
    assert!(matches!(save.await.unwrap(), Err(RunsError::Superseded)));
    assert!(
        f.core
            .runs()
            .task(&next, 2, 3)
            .await
            .unwrap()
            .executor
            .is_none()
    );
    f.finish().await;
}

#[tokio::test]
async fn a_start_command_from_before_the_upgrade_replays() {
    let root = tempfile::tempdir().unwrap();
    let db_path = root.path().join("before.sqlite3");
    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .connect_with(
            sqlx::sqlite::SqliteConnectOptions::new()
                .filename(&db_path)
                .create_if_missing(true),
        )
        .await
        .unwrap();
    let migrations = sqlx::migrate!("../core-db/migrations");
    sqlx::migrate::Migrator::with_migrations(
        migrations
            .iter()
            .filter(|migration| migration.version <= 72)
            .cloned()
            .collect(),
    )
    .run(&pool)
    .await
    .unwrap();
    let workflow = WorkflowId::from_literal(SOURCE);
    let fingerprint = CommandContext::fingerprint(
        "run.start",
        &serde_json::json!({"workflow": workflow, "version": 1, "task": 3, "model": "fake-large"}),
    );
    sqlx::query(
        "INSERT INTO run_attempts (
            id, project_id, workflow_id, version, task, number, thread_id, model, state,
            writes_json, started_at, ended_at, released_at, command_id, fingerprint
        ) VALUES (
            'legacy-attempt', 'old-project', ?, 1, 3, 1, 'old-thread', 'fake-large',
            'failed', '[]', '2026-11-17T00:00:00Z', '2026-11-17T00:00:01Z',
            '2026-11-17T00:00:01Z', 'legacy', ?
        )",
    )
    .bind(workflow.as_str())
    .bind(fingerprint)
    .execute(&pool)
    .await
    .unwrap();
    pool.close().await;
    let core = shadows_core::testing::app(db_path).await.unwrap();
    let runs = core.runs();
    let replay = runs.start("legacy".into(), &workflow, 1, 3).await.unwrap();
    assert_eq!(replay.id.as_str(), "legacy-attempt");
    assert_eq!(replay.model, "fake-large");
    assert_eq!(replay.harness, "claude-code");
    assert_eq!(replay.effort, None);
    assert!(matches!(
        runs.start("legacy".into(), &workflow, 1, 4).await,
        Err(RunsError::CommandConflict)
    ));
    core.shut_down(Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
}

#[tokio::test]
async fn a_saved_model_no_longer_offered_fails_the_attempt() {
    let f = fixture().await;
    let runs = f.core.runs();
    runs.choose_executor_for_test(&f.workflow, 3, "claude-code", "retired", None)
        .await
        .unwrap();
    let attempt = runs
        .start("start-retired".into(), &f.workflow, 1, 3)
        .await
        .unwrap();
    assert_eq!(attempt.state, AttemptState::Failed);
    assert!(
        attempt
            .reason
            .unwrap()
            .contains("model: retired is not offered")
    );
    f.finish().await;
}

#[tokio::test]
async fn an_addition_keeps_the_attempts_own_choice() {
    let f = fixture().await;
    let thread = f.executor_thread(3).await;
    let previous = f
        .chain_attempt(&thread, 3, 1, "committed", None, false)
        .await;
    sqlx::query("UPDATE run_attempts SET effort = 'low' WHERE id = ?")
        .bind(previous.as_str())
        .execute(f.storage.reader())
        .await
        .unwrap();
    save(&f, "future", "max").await;
    let grant = planner_grant(&f).await;
    let next = f
        .core
        .runs()
        .amend(&grant, &f.workflow, 3, "Keep the rest intact.".into())
        .await
        .unwrap();
    assert_eq!(next.thread_id, thread);
    assert_eq!(next.harness, "claude-code");
    assert_eq!(next.model, "fake-large");
    assert_eq!(next.effort.as_deref(), Some("low"));
    let operation = f
        .core
        .turns()
        .operations(&thread)
        .await
        .unwrap()
        .into_iter()
        .find(|op| Some(&op.id) == next.operation_id.as_ref())
        .unwrap();
    assert_eq!(
        operation.invocation.unwrap().requested_effort.as_deref(),
        Some("low")
    );
    f.finish().await;
}
