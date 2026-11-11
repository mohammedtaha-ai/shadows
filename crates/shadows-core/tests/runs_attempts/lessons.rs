//! The record a task leaves when it stops on the cap of additions, and the
//! Planner's account written into it. No agent runs: the attempts are rows on
//! one executor thread, and its tool calls are entries written by the test.
use super::planner::executor_grant;
use super::*;
use core_runs::TaskLesson;

/// A tool card of `kind` with `status` and `error`, as the watcher stores one.
fn card(kind: &str, status: &str, error: Option<&str>) -> Value {
    serde_json::json!({
        "id": format!("call-{kind}-{status}"),
        "tool": null,
        "kind": kind,
        "status": status,
        "target": null,
        "lines": null,
        "added": null,
        "removed": null,
        "diff": null,
        "error": error,
        "started_at": "2026-10-05T00:00:00Z",
        "finished_at": "2026-10-05T00:00:01Z",
    })
}

impl Fixture {
    /// T3 with four committed attempts on one thread, 2 to 4 each carrying an
    /// addition, 4 still waiting; the thread holds two shell calls and an edit.
    async fn capped_chain(&self) {
        let thread = self.executor_thread(3).await;
        let calls = [
            ("go test ./...", card("execute", "completed", None)),
            ("Edit main.go", card("edit", "completed", None)),
            (
                "docker compose up -d",
                card("execute", "failed", Some("docker: not found")),
            ),
        ];
        for (body, card) in &calls {
            shadows_core::Threads::append_thread_entry_for_test(
                &self.storage,
                &thread,
                shadows_core::testing::NewThreadEntry {
                    kind: ThreadEntryKind::ToolCall,
                    author: shadows_core::Actor::system(),
                    body,
                    refs: &[],
                    card: Some(card),
                    operation_id: None,
                },
            )
            .await
            .unwrap();
        }
        self.chain_attempt(&thread, 3, 1, "committed", None, true)
            .await;
        for (number, addition) in [(2, "add B"), (3, "add C")] {
            self.chain_attempt(&thread, 3, number, "committed", Some(addition), true)
                .await;
        }
        self.chain_attempt(&thread, 3, 4, "committed", Some("add D"), false)
            .await;
    }

    async fn lessons(&self) -> Vec<TaskLesson> {
        let rows: Vec<String> = sqlx::query_scalar("SELECT record_json FROM run_lessons")
            .fetch_all(self.storage.reader())
            .await
            .unwrap();
        rows.iter()
            .map(|json| serde_json::from_str(json).unwrap())
            .collect()
    }
}

#[tokio::test]
async fn a_task_stopped_on_the_cap_leaves_a_record() {
    let f = fixture().await;
    let grant = planner_grant(&f).await;
    f.capped_chain().await;
    let refused = f
        .core
        .runs()
        .amend(&grant, &f.workflow, 3, "more".into())
        .await;
    assert!(
        matches!(refused, Err(RunsError::AdditionsSpent { task: 3 })),
        "{refused:?}"
    );
    let lessons = f.lessons().await;
    assert_eq!(lessons.len(), 1, "{lessons:?}");
    let lesson = &lessons[0];
    assert_eq!(lesson.task, 3);
    assert_eq!(lesson.workflow_id, f.workflow);
    let chain: Vec<(u32, Option<&str>)> = lesson
        .attempts
        .iter()
        .map(|a| (a.number, a.addition.as_deref()))
        .collect();
    assert_eq!(
        chain,
        vec![
            (1, None),
            (2, Some("add B")),
            (3, Some("add C")),
            (4, Some("add D"))
        ]
    );
    let commands: Vec<(&str, &str, Option<&str>)> = lesson
        .commands
        .iter()
        .map(|c| (c.command.as_str(), c.status.as_str(), c.error.as_deref()))
        .collect();
    assert_eq!(
        commands,
        vec![
            ("go test ./...", "completed", None),
            ("docker compose up -d", "failed", Some("docker: not found")),
        ]
    );
    assert_eq!(lesson.stopped_by, None);
    assert_eq!(lesson.plan_should_have_said, None);
    f.finish().await;
}

#[tokio::test]
async fn the_planner_writes_what_the_plan_should_have_said() {
    let f = fixture().await;
    let grant = planner_grant(&f).await;
    f.capped_chain().await;
    let runs = f.core.runs();
    let refused = runs.amend(&grant, &f.workflow, 3, "more".into()).await;
    assert!(
        matches!(refused, Err(RunsError::AdditionsSpent { task: 3 })),
        "{refused:?}"
    );
    let (stopped_by, said) = (
        "Docker is not installed here.",
        "T3 needs Docker; the setup plan should install or check it.",
    );
    let written = runs
        .write_lesson(&grant, &f.workflow, 3, stopped_by.into(), said.into())
        .await
        .unwrap();
    assert_eq!(written.stopped_by.as_deref(), Some(stopped_by));
    assert_eq!(written.plan_should_have_said.as_deref(), Some(said));
    let lessons = f.lessons().await;
    assert_eq!(lessons, vec![written]);
    let empty = runs
        .write_lesson(&grant, &f.workflow, 3, "  ".into(), said.into())
        .await;
    assert!(matches!(empty, Err(RunsError::EmptyLesson)), "{empty:?}");
    let none = runs
        .write_lesson(&grant, &f.workflow, 4, stopped_by.into(), said.into())
        .await;
    assert!(
        matches!(none, Err(RunsError::NoLesson { task: 4 })),
        "{none:?}"
    );
    let other = runs
        .write_lesson(
            &executor_grant(&f),
            &f.workflow,
            3,
            stopped_by.into(),
            said.into(),
        )
        .await;
    assert!(matches!(other, Err(RunsError::NotFound)), "{other:?}");
    f.finish().await;
}
