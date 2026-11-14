//! The saved context of each attempt: written once inside the write that
//! admits the attempt, then only read.
use sqlx::SqliteConnection;

use super::RunsStore;
use crate::RunsError;
use crate::model::AttemptId;
use core_db::{StorageError, Tx};
use core_plans::{Plans, TaskContext, WorkflowId};
use core_projects::ProjectId;

impl RunsStore {
    /// Plans' context of the task, as the admitting write sees it, saved for
    /// `attempt`. Called after the attempt's insert and before its commit, so
    /// a failure here rolls the whole admission back.
    pub(super) async fn capture_in(
        tx: &mut Tx,
        attempt: &AttemptId,
        (project, workflow, task): (&ProjectId, &WorkflowId, u32),
        captured_at: &str,
    ) -> Result<(), RunsError> {
        let context = Plans::task_context_in(tx, project, workflow, task, None)
            .await
            .map_err(RunsError::from_plans)?;
        Self::capture_context_in(tx, attempt, &context, captured_at).await
    }

    async fn capture_context_in(
        tx: &mut SqliteConnection,
        attempt: &AttemptId,
        context: &TaskContext,
        captured_at: &str,
    ) -> Result<(), RunsError> {
        sqlx::query(
            "INSERT INTO run_attempt_context (attempt_id, context_json, captured_at)
             VALUES (?, ?, ?)",
        )
        .bind(attempt.as_str())
        .bind(serde_json::to_string(context)?)
        .bind(captured_at)
        .execute(&mut *tx)
        .await?;
        Ok(())
    }

    /// The context saved for `attempt` and when; `None` for an attempt
    /// recorded before contexts were saved.
    pub async fn context(
        &self,
        attempt: &AttemptId,
    ) -> Result<Option<(TaskContext, String)>, RunsError> {
        let row: Option<(String, String)> = sqlx::query_as(
            "SELECT context_json, captured_at FROM run_attempt_context WHERE attempt_id = ?",
        )
        .bind(attempt.as_str())
        .fetch_optional(self.storage.reader())
        .await
        .map_err(StorageError::from)?;
        row.map(|(json, at)| Ok((serde_json::from_str(&json)?, at)))
            .transpose()
    }
}
