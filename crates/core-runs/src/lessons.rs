//! One job: the record a task leaves when it stops on the cap.
//!
//! When a task's fourth addition is refused, Runs keeps a record that can be
//! read to improve Shadows: the chain of attempts from its Start with each
//! addition, the shell calls its executor ran, and, once the Planner writes
//! them, what stopped the work and what the plan should have said. Nothing in
//! Shadows reads the records yet; they are read from `run_lessons`.
use crate::model::{LessonAttempt, LessonCommand, TaskLesson};
use crate::{Attempt, Runs, RunsError};
use core_db::Storage;
use core_grants::Grant;
use core_plans::{Plan, WorkflowId};
use core_threads::ThreadEntryKind;
use shadows_agent::events::{CallKind, ToolCallCard};

impl Runs {
    /// Records the lesson of `stopped`, the attempt the cap stopped. A
    /// failure is logged and never changes the cap's answer.
    pub(crate) async fn record_lesson(&self, plan: &Plan, stopped: &Attempt) {
        match self.lesson_of(plan, stopped).await {
            Ok(lesson) => match self
                .inner
                .store
                .put_lesson(&stopped.id, &stopped.project_id, &lesson)
                .await
            {
                Ok(()) => tracing::info!(task = stopped.task, "runs.lesson_recorded"),
                Err(error) => tracing::warn!(%error, "runs.lesson_failed"),
            },
            Err(error) => tracing::warn!(%error, "runs.lesson_failed"),
        }
    }

    /// The record: the chain from the newest attempt down to the one a Start
    /// began, oldest first, and the thread's shell calls in order.
    async fn lesson_of(&self, plan: &Plan, stopped: &Attempt) -> Result<TaskLesson, RunsError> {
        let all = self
            .attempts(&stopped.workflow_id, stopped.version, stopped.task)
            .await?;
        let mut attempts = Vec::new();
        for attempt in all {
            let began = attempt.addition.is_none();
            attempts.push(LessonAttempt {
                number: attempt.number,
                addition: attempt.addition,
                commit: attempt.commit,
                report: attempt.report,
            });
            if began {
                break;
            }
        }
        attempts.reverse();
        let entries = self
            .inner
            .threads
            .entries(&stopped.thread_id)
            .await
            .map_err(|error| RunsError::Unavailable(error.to_string()))?;
        let commands = entries
            .into_iter()
            .filter(|entry| entry.kind == ThreadEntryKind::ToolCall)
            .filter_map(|entry| {
                let card: ToolCallCard = serde_json::from_value(entry.card?).ok()?;
                (card.kind == CallKind::Execute).then(|| LessonCommand {
                    command: entry.body,
                    status: serde_json::to_value(&card.status)
                        .ok()
                        .and_then(|v| v.as_str().map(str::to_string))
                        .unwrap_or_default(),
                    error: card.error,
                })
            })
            .collect();
        let title = plan
            .tasks
            .iter()
            .find(|t| t.content.number == stopped.task)
            .map(|t| t.content.title.clone())
            .unwrap_or_default();
        Ok(TaskLesson {
            workflow_id: stopped.workflow_id.clone(),
            version: stopped.version,
            task: stopped.task,
            title,
            attempts,
            commands,
            stopped_by: None,
            plan_should_have_said: None,
            recorded_at: Storage::now(),
        })
    }

    /// The Planner writes what stopped the work and what the plan should have
    /// said into the task's latest record. Writing again replaces them.
    pub async fn write_lesson(
        &self,
        grant: &Grant,
        workflow: &WorkflowId,
        task: u32,
        stopped_by: String,
        plan_should_have_said: String,
    ) -> Result<TaskLesson, RunsError> {
        let plan = self.planner_plan(grant, workflow).await?;
        let (stopped_by, said) = (stopped_by.trim(), plan_should_have_said.trim());
        if stopped_by.is_empty() || said.is_empty() {
            return Err(RunsError::EmptyLesson);
        }
        let (attempt, project, mut lesson) = self
            .inner
            .store
            .latest_lesson(workflow, plan.version, task)
            .await?
            .ok_or(RunsError::NoLesson { task })?;
        lesson.stopped_by = Some(stopped_by.to_string());
        lesson.plan_should_have_said = Some(said.to_string());
        self.inner
            .store
            .put_lesson(&attempt, &project, &lesson)
            .await?;
        Ok(lesson)
    }
}
