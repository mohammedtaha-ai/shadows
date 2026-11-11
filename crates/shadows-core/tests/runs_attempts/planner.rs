//! The Planner's calls on its project's tasks: its done marks, the task they
//! name next, and its Start, refused to any grant but a Planner's. No agent
//! runs.
use super::*;

impl Fixture {
    /// A `failed` attempt at `task` that ended and freed its scope, as a turn
    /// that ended without a commit leaves one.
    async fn failed_attempt(&self, task: u32) {
        let (project, workflow) = (self.project.clone(), self.workflow.clone());
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
                    let id = format!("failed-{task}");
                    sqlx::query(
                        "INSERT INTO run_attempts
                           (id, project_id, workflow_id, version, task, number, thread_id,
                            model, state, reason, writes_json, started_at, ended_at,
                            released_at, command_id, fingerprint)
                         VALUES (?, ?, ?, 1, ?, 1, ?, 'fake-large', 'failed',
                                 'ended without a commit', '[]', '2026-10-05T00:00:00Z',
                                 '2026-10-05T00:01:00Z', '2026-10-05T00:01:00Z', ?, 'f')",
                    )
                    .bind(&id)
                    .bind(project.as_str())
                    .bind(workflow.as_str())
                    .bind(i64::from(task))
                    .bind(thread.as_str())
                    .bind(&id)
                    .execute(&mut *conn)
                    .await?;
                    Ok(())
                })
            })
            .await
            .unwrap();
    }
}

/// An executor's grant on T3, as an executor session holds one.
fn executor_grant(f: &Fixture) -> shadows_core::Grant {
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
    assert_eq!(marked.next, Some(3), "T3 never ran and needs nothing");
    let t6 = runs.task(&f.workflow, 1, 6).await.unwrap();
    assert!(t6.start.allowed, "{:?}", t6.start);
    let unmarked = runs
        .mark_done_for(&grant, &f.workflow, 1, false)
        .await
        .unwrap();
    assert!(!unmarked.task.done);
    assert_eq!(unmarked.next, None);
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
    assert_eq!(marked.next, Some(4), "T3 failed: it goes to the person");
    runs.mark_done("mark-t4".into(), &f.workflow, 1, 4, true)
        .await
        .unwrap();
    let marked = runs
        .mark_done_for(&grant, &f.workflow, 1, true)
        .await
        .unwrap();
    assert_eq!(marked.next, Some(5), "T4 is done: it never starts twice");
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
