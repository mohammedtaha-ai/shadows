//! Lessons records SQL: one row per attempt a task was stopped at on the cap.
use super::RunsStore;
use crate::RunsError;
use crate::model::{AttemptId, TaskLesson};
use core_db::Storage;
use core_plans::WorkflowId;
use core_projects::ProjectId;

impl RunsStore {
    /// Inserts the record of `attempt`, or replaces the one it has.
    pub async fn put_lesson(
        &self,
        attempt: &AttemptId,
        project: &ProjectId,
        lesson: &TaskLesson,
    ) -> Result<(), RunsError> {
        let json = serde_json::to_string(lesson)?;
        let (attempt, project, ts) = (attempt.clone(), project.clone(), Storage::now());
        let (workflow, version, task) = (
            lesson.workflow_id.clone(),
            lesson.version,
            i64::from(lesson.task),
        );
        self.storage
            .write_txn_as(move |conn| {
                Box::pin(async move {
                    sqlx::query(
                        "INSERT INTO run_lessons
                           (attempt_id, project_id, workflow_id, version, task, record_json,
                            created_at, updated_at)
                         VALUES (?, ?, ?, ?, ?, ?, ?, ?)
                         ON CONFLICT(attempt_id) DO UPDATE SET
                           record_json = excluded.record_json,
                           updated_at = excluded.updated_at",
                    )
                    .bind(attempt.as_str())
                    .bind(project.as_str())
                    .bind(workflow.as_str())
                    .bind(version)
                    .bind(task)
                    .bind(&json)
                    .bind(&ts)
                    .bind(&ts)
                    .execute(&mut *conn)
                    .await?;
                    Ok(())
                })
            })
            .await
    }

    /// The task's latest record, by `created_at`, then `attempt_id`.
    pub async fn latest_lesson(
        &self,
        workflow: &WorkflowId,
        version: i64,
        task: u32,
    ) -> Result<Option<(AttemptId, ProjectId, TaskLesson)>, RunsError> {
        let row: Option<(String, String, String)> = sqlx::query_as(
            "SELECT attempt_id, project_id, record_json FROM run_lessons
             WHERE workflow_id = ? AND version = ? AND task = ?
             ORDER BY created_at DESC, attempt_id DESC LIMIT 1",
        )
        .bind(workflow.as_str())
        .bind(version)
        .bind(i64::from(task))
        .fetch_optional(self.storage.reader())
        .await?;
        row.map(|(attempt, project, json)| {
            Ok((
                AttemptId::from_recorded(attempt),
                ProjectId::from_recorded(project),
                serde_json::from_str(&json)?,
            ))
        })
        .transpose()
    }
}
