//! The Planner's calls on its project's tasks: its done marks, the group they
//! name next, and its Start, refused to any grant but a Planner's. No agent
//! runs.
use super::*;

impl Fixture {
    /// A `failed` attempt at `task` that ended and freed its scope, as a turn
    /// that ended without a commit leaves one.
    async fn failed_attempt(&self, task: u32) {
        self.ended_attempt(task, "failed", Some("2026-10-05T00:01:00Z"))
            .await;
    }

    /// A `committed` attempt at T3 whose turn has ended and which still holds
    /// its scope, waiting for the Planner's judgment. It has no operation.
    async fn committed_attempt(&self) -> core_runs::AttemptId {
        self.ended_attempt(3, "committed", None).await
    }

    /// An attempt at `task` in `state`, ended, released at `released`.
    async fn ended_attempt(
        &self,
        task: u32,
        state: &'static str,
        released: Option<&'static str>,
    ) -> core_runs::AttemptId {
        let (project, workflow) = (self.project.clone(), self.workflow.clone());
        let id = format!("{state}-{task}");
        let row = id.clone();
        self.storage
            .write_txn(move |conn| {
                Box::pin(async move {
                    let thread = shadows_core::Threads::create_executor_in(
                        conn,
                        &project,
                        format!("T{task}"),
                        "claude-code",
                    )
                    .await?;
                    sqlx::query(
                        "INSERT INTO run_attempts
                           (id, project_id, workflow_id, version, task, number, thread_id,
                            model, state, reason, writes_json, started_at, ended_at,
                            released_at, command_id, fingerprint)
                         VALUES (?, ?, ?, 1, ?, 1, ?, 'fake-large', ?, NULL,
                                 '[\"backend/app/**\"]', '2026-10-05T00:00:00Z',
                                 '2026-10-05T00:01:00Z', ?, ?, 'f')",
                    )
                    .bind(&row)
                    .bind(project.as_str())
                    .bind(workflow.as_str())
                    .bind(i64::from(task))
                    .bind(thread.as_str())
                    .bind(state)
                    .bind(released)
                    .bind(&row)
                    .execute(&mut *conn)
                    .await?;
                    Ok(())
                })
            })
            .await
            .unwrap();
        core_runs::AttemptId::from_recorded(id)
    }

    /// A new executor thread for T`task`, for a chain of attempts to share.
    pub(crate) async fn executor_thread(&self, task: u32) -> ThreadId {
        let project = self.project.clone();
        self.storage
            .write_txn(move |conn| {
                Box::pin(async move {
                    shadows_core::Threads::create_executor_in(
                        conn,
                        &project,
                        format!("T{task}"),
                        "claude-code",
                    )
                    .await
                })
            })
            .await
            .unwrap()
    }

    /// Attempt `number` at `task` on `thread` in `state` (ended unless
    /// running), carrying `addition`, released when `released`.
    pub(crate) async fn chain_attempt(
        &self,
        thread: &ThreadId,
        task: u32,
        number: u32,
        state: &'static str,
        addition: Option<&'static str>,
        released: bool,
    ) -> core_runs::AttemptId {
        let (project, workflow) = (self.project.clone(), self.workflow.clone());
        let id = format!("chain-{task}-{number}");
        let (row, thread) = (id.clone(), thread.clone());
        let ended = (state != "running").then_some("2026-10-05T00:01:00Z");
        let released = released.then_some("2026-10-05T00:01:00Z");
        self.storage
            .write_txn(move |conn| {
                Box::pin(async move {
                    sqlx::query(
                        "INSERT INTO run_attempts
                           (id, project_id, workflow_id, version, task, number, thread_id,
                            model, state, commit_sha, writes_json, started_at, ended_at,
                            released_at, command_id, fingerprint, addition)
                         VALUES (?, ?, ?, 1, ?, ?, ?, 'fake-large', ?, ?,
                                 '[\"backend/app/**\"]', '2026-10-05T00:00:00Z',
                                 ?, ?, ?, 'f', ?)",
                    )
                    .bind(&row)
                    .bind(project.as_str())
                    .bind(workflow.as_str())
                    .bind(i64::from(task))
                    .bind(i64::from(number))
                    .bind(thread.as_str())
                    .bind(state)
                    .bind(format!("{number:040x}"))
                    .bind(ended)
                    .bind(released)
                    .bind(&row)
                    .bind(addition)
                    .execute(&mut *conn)
                    .await?;
                    Ok(())
                })
            })
            .await
            .unwrap();
        core_runs::AttemptId::from_recorded(id)
    }

    /// How many turns the plan's Planner conversation has.
    async fn planner_turns(&self) -> usize {
        let thread: String =
            sqlx::query_scalar("SELECT written_by_thread FROM workflow WHERE id = ?")
                .bind(SOURCE)
                .fetch_one(self.storage.reader())
                .await
                .unwrap();
        self.core
            .turns()
            .operations(&ThreadId::from_literal(thread))
            .await
            .unwrap()
            .len()
    }
}

/// An executor's grant on T3, as an executor session holds one.
pub(crate) fn executor_grant(f: &Fixture) -> shadows_core::Grant {
    shadows_core::Grant {
        id: shadows_core::GrantId::generate(),
        kind: shadows_core::GrantKind::Executor,
        project_id: f.project.clone(),
        thread_id: None,
        created_at: "2026-11-11T00:00:00Z".into(),
        revoked_at: None,
        scope: Some(shadows_core::testing::TaskScope {
            workflow: SOURCE.into(),
            version: 1,
            task: 3,
        }),
    }
}

/// The task numbers a done mark names next.
fn next(marked: &shadows_core::DoneMark) -> Vec<u32> {
    marked.next.iter().map(|ready| ready.task).collect()
}

#[tokio::test]
async fn the_planner_marks_a_task_done_and_not_done() {
    let f = fixture().await;
    let grant = planner_grant(&f).await;
    let runs = f.core.runs();
    let marked = runs
        .mark_done_for(&grant, &f.workflow, 1, true)
        .await
        .unwrap();
    assert!(marked.task.done);
    assert_eq!(
        next(&marked),
        [3, 5, 6],
        "T4 meets T3; T7 waits on another plan"
    );
    let t6 = runs.task(&f.workflow, 1, 6).await.unwrap();
    assert!(t6.start.allowed, "{:?}", t6.start);
    let unmarked = runs
        .mark_done_for(&grant, &f.workflow, 1, false)
        .await
        .unwrap();
    assert!(!unmarked.task.done);
    assert!(unmarked.next.is_empty());
    let t6 = runs.task(&f.workflow, 1, 6).await.unwrap();
    assert_eq!(t6.start.code.as_deref(), Some("waiting"));
    f.finish().await;
}

#[tokio::test]
async fn the_next_task_is_one_that_never_ran() {
    let f = fixture().await;
    let grant = planner_grant(&f).await;
    f.failed_attempt(3).await;
    let runs = f.core.runs();
    let marked = runs
        .mark_done_for(&grant, &f.workflow, 1, true)
        .await
        .unwrap();
    assert_eq!(next(&marked), [4, 5, 6], "T3 failed: it goes to the person");
    runs.mark_done("mark-t4".into(), &f.workflow, 1, 4, true)
        .await
        .unwrap();
    let marked = runs
        .mark_done_for(&grant, &f.workflow, 1, true)
        .await
        .unwrap();
    assert_eq!(next(&marked), [5, 6], "T4 is done: it never starts twice");
    f.finish().await;
}

#[tokio::test]
async fn the_planner_start_needs_a_model() {
    let f = fixture().await;
    let grant = planner_grant(&f).await;
    let refused = f.core.runs().start_for(&grant, &f.workflow, 3, None).await;
    assert!(matches!(refused, Err(RunsError::NoModel)), "{refused:?}");
    let attempts = f.core.runs().attempts(&f.workflow, 1, 3).await.unwrap();
    assert!(attempts.is_empty(), "{attempts:?}");
    f.finish().await;
}

#[tokio::test]
async fn the_planner_tools_refuse_a_grant_that_is_not_a_planners() {
    let f = fixture().await;
    let project = shadows_core::Grants::issue_project_grant_for_test(
        &f.storage,
        &command("project-grant"),
        &f.project,
    )
    .await
    .unwrap()
    .grant;
    let runs = f.core.runs();
    for grant in [executor_grant(&f), project] {
        let started = runs
            .start_for(&grant, &f.workflow, 3, Some("fake-large".into()))
            .await;
        assert!(matches!(started, Err(RunsError::NotFound)), "{started:?}");
        let marked = runs.mark_done_for(&grant, &f.workflow, 1, true).await;
        assert!(matches!(marked, Err(RunsError::NotFound)), "{marked:?}");
    }
    assert!(!runs.task(&f.workflow, 1, 1).await.unwrap().done);
    assert!(runs.attempts(&f.workflow, 1, 3).await.unwrap().is_empty());
    f.finish().await;
}

#[tokio::test]
async fn stopping_a_committed_attempt_frees_its_scope() {
    let f = fixture().await;
    let id = f.committed_attempt().await;
    let runs = f.core.runs();
    let t3 = runs.task(&f.workflow, 1, 3).await.unwrap();
    assert_eq!(
        t3.start.code.as_deref(),
        Some("task_busy"),
        "{:?}",
        t3.start
    );
    let turns = f.planner_turns().await;
    let stopped = runs.stop(&id).await.unwrap();
    assert_eq!(stopped.state, AttemptState::Committed);
    let t3 = runs.task(&f.workflow, 1, 3).await.unwrap();
    assert!(t3.start.allowed, "{:?}", t3.start);
    assert_eq!(f.planner_turns().await, turns, "the Planner was told again");
    f.finish().await;
}

#[tokio::test]
async fn an_addition_needs_a_committed_attempt_waiting() {
    let f = fixture().await;
    let grant = planner_grant(&f).await;
    let runs = f.core.runs();
    let refused = runs.amend(&grant, &f.workflow, 3, "x".into()).await;
    assert!(
        matches!(refused, Err(RunsError::NotAwaiting { task: 3 })),
        "{refused:?}"
    );
    let thread = f.executor_thread(3).await;
    f.chain_attempt(&thread, 3, 1, "running", None, false).await;
    let refused = runs.amend(&grant, &f.workflow, 3, "x".into()).await;
    assert!(
        matches!(refused, Err(RunsError::NotAwaiting { task: 3 })),
        "a running attempt takes no addition: {refused:?}"
    );
    let empty = runs.amend(&grant, &f.workflow, 3, "   ".into()).await;
    assert!(matches!(empty, Err(RunsError::EmptyAddition)), "{empty:?}");
    let other = runs
        .amend(&executor_grant(&f), &f.workflow, 3, "x".into())
        .await;
    assert!(matches!(other, Err(RunsError::NotFound)), "{other:?}");
    assert_eq!(runs.attempts(&f.workflow, 1, 3).await.unwrap().len(), 1);
    f.finish().await;
}

#[tokio::test]
async fn the_fourth_addition_stops_the_task() {
    let f = fixture().await;
    let grant = planner_grant(&f).await;
    let runs = f.core.runs();
    let thread = f.executor_thread(3).await;
    f.chain_attempt(&thread, 3, 1, "committed", None, true)
        .await;
    for number in 2..=3 {
        f.chain_attempt(&thread, 3, number, "committed", Some("more"), true)
            .await;
    }
    let fourth = f
        .chain_attempt(&thread, 3, 4, "committed", Some("more"), false)
        .await;
    let refused = runs.amend(&grant, &f.workflow, 3, "more".into()).await;
    assert!(
        matches!(refused, Err(RunsError::AdditionsSpent { task: 3 })),
        "{refused:?}"
    );
    assert_eq!(
        runs.attempt(&fourth).await.unwrap().state,
        AttemptState::Committed
    );
    let t3 = runs.task(&f.workflow, 1, 3).await.unwrap();
    assert!(t3.start.allowed, "{:?}", t3.start);
    assert_eq!(runs.attempts(&f.workflow, 1, 3).await.unwrap().len(), 4);
    let thread = f.executor_thread(3).await;
    f.chain_attempt(&thread, 3, 5, "committed", None, true)
        .await;
    f.chain_attempt(&thread, 3, 6, "committed", Some("more"), false)
        .await;
    let other = runs
        .amend(&executor_grant(&f), &f.workflow, 3, "more".into())
        .await;
    assert!(
        matches!(other, Err(RunsError::NotFound)),
        "the cap does not open the tool to others: {other:?}"
    );
    f.finish().await;
}

#[tokio::test]
async fn recovery_keeps_a_committed_attempt_waiting_for_its_judgment() {
    let f = fixture().await;
    f.committed_attempt().await;
    let runs = f.core.runs();
    let report = runs.recover().await.unwrap();
    assert_eq!(report.interrupted, 0);
    let t3 = runs.task(&f.workflow, 1, 3).await.unwrap();
    assert_eq!(
        t3.start.code.as_deref(),
        Some("task_busy"),
        "{:?}",
        t3.start
    );
    f.finish().await;
}

#[tokio::test]
async fn a_notice_waits_on_a_busy_planner_in_the_executors_name() {
    let f = fixture().await;
    let planner: String = sqlx::query_scalar("SELECT written_by_thread FROM workflow WHERE id = ?")
        .bind(SOURCE)
        .fetch_one(f.storage.reader())
        .await
        .unwrap();
    let planner = ThreadId::from_literal(planner);
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
    let executor = f.executor_thread(3).await;
    let turn = |command: &str, prompt: &str| shadows_core::SendTurn {
        command_id: command.into(),
        prompt: prompt.into(),
        attachments: vec![],
        model: "fake-large".into(),
        mode: "default".into(),
        effort: None,
        focus: None,
        plan: None,
        client_tab: None,
    };
    let notice = shadows_core::Notice {
        executor: Some(executor.clone()),
        card: serde_json::json!({ "task": 3, "attempt": 1, "outcome": "committed" }),
    };
    let turns = f.core.turns();
    let waiting = |queued| match queued {
        shadows_core::Queued::Waiting { message } => message,
        other => panic!("a busy Planner keeps it waiting: {other:?}"),
    };
    let first = turns
        .queue_notice(
            planner.clone(),
            turn("n1", "[Shadows] T3 ..."),
            notice.clone(),
        )
        .await
        .unwrap();
    let first = waiting(first);
    assert_eq!(first.notice.as_ref(), Some(&notice));
    let own = waiting(
        turns
            .queue(planner.clone(), turn("o1", "my own words"))
            .await
            .unwrap(),
    );
    assert_eq!(own.notice, None);
    let queued = turns.queued(&planner).await.unwrap();
    let listed: Vec<_> = queued
        .iter()
        .map(|m| (m.id.clone(), m.notice.clone()))
        .collect();
    assert_eq!(
        listed,
        [
            (first.id.clone(), Some(notice.clone())),
            (own.id.clone(), None)
        ]
    );
    for _ in 0..2 {
        let kept = turns.unqueue(&planner, &first.id, "u1".into()).await;
        assert!(
            matches!(kept, Err(shadows_core::TurnsError::QueuedNoticeKept)),
            "{kept:?}"
        );
    }
    turns.unqueue(&planner, &own.id, "u2".into()).await.unwrap();
    let left: Vec<_> = turns
        .queued(&planner)
        .await
        .unwrap()
        .into_iter()
        .map(|m| m.id)
        .collect();
    assert_eq!(left, std::slice::from_ref(&first.id));
    let principal: (String, String) = sqlx::query_as(
        "SELECT principal_kind, principal_id FROM command_record WHERE command_id = 'n1'",
    )
    .fetch_one(f.storage.reader())
    .await
    .unwrap();
    assert_eq!(
        principal,
        ("Executor".into(), executor.as_str().to_string())
    );
    f.finish().await;
}

/// A refused answer is Start's own refusal: the same code, and Start writes
/// nothing. T3 is busy with a committed attempt; T6 waits on T1.
#[tokio::test]
async fn a_refused_answer_is_starts_refusal() {
    let f = fixture().await;
    f.committed_attempt().await;
    let runs = f.core.runs();
    for task in [3, 6] {
        let answer = runs.task(&f.workflow, 1, task).await.unwrap().start;
        assert!(!answer.allowed, "T{task}: {answer:?}");
        let started = runs
            .start(
                format!("start-{task}"),
                &f.workflow,
                1,
                task,
                "fake-large".into(),
            )
            .await;
        let Err(error) = started else {
            panic!("T{task} started though refused: {started:?}")
        };
        assert_eq!(answer.code.as_deref(), Some(error.code()), "T{task}");
    }
    assert!(runs.attempts(&f.workflow, 1, 6).await.unwrap().is_empty());
    f.finish().await;
}
