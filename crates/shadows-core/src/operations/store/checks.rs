use crate::operations::Operations;
use core_db::StorageError;
use core_threads::{ThreadId, Threads};
use sqlx::SqliteConnection;
impl Operations {
    pub(crate) async fn has_open_in(
        conn: &mut SqliteConnection,
        thread: &ThreadId,
    ) -> Result<bool, StorageError> {
        let found: Option<i64> = sqlx::query_scalar(
            "SELECT 1 FROM operation
          WHERE thread_id = ?
            AND status_kind NOT IN ('Completed','Failed','Cancelled','Interrupted')
          LIMIT 1",
        )
        .bind(thread.as_str())
        .fetch_optional(&mut *conn)
        .await?;
        Ok(found.is_some())
    }
}
impl Operations {
    pub(crate) async fn require_running_in(
        conn: &mut SqliteConnection,
        thread: &ThreadId,
        operation: &str,
    ) -> Result<(), StorageError> {
        let found: Option<i64> = sqlx::query_scalar(
            "SELECT 1 FROM operation o
         WHERE o.id=? AND o.thread_id=? AND o.status_kind='Running' ",
        )
        .bind(operation)
        .bind(thread.as_str())
        .fetch_optional(&mut *conn)
        .await?;
        if found.is_none() || !Threads::is_live_in(conn, thread).await? {
            return Err(StorageError::Constraint(
                "the proposal's turn is no longer running".into(),
            ));
        }
        Ok(())
    }
}

impl Operations {
    pub(crate) async fn require_status_in(
        conn: &mut SqliteConnection,
        op: &crate::operations::OperationId,
        expected: &str,
    ) -> Result<(), StorageError> {
        let status: Option<String> =
            sqlx::query_scalar("SELECT status_kind FROM operation WHERE id=?")
                .bind(op.as_str())
                .fetch_optional(conn)
                .await?;
        if status.as_deref() != Some(expected) {
            return Err(StorageError::TransitionConflict {
                expected: expected.into(),
                found: "another status".into(),
            });
        }
        Ok(())
    }
    pub(crate) async fn running_for_in(
        conn: &mut SqliteConnection,
        thread: &ThreadId,
    ) -> Result<Option<crate::operations::OperationId>, StorageError> {
        let id: Option<String> = sqlx::query_scalar(
            "SELECT id FROM operation WHERE thread_id=? AND status_kind='Running' ORDER BY id",
        )
        .bind(thread.as_str())
        .fetch_optional(conn)
        .await?;
        Ok(id.map(crate::operations::OperationId::from_stored))
    }
    pub(crate) async fn started_thread_in(
        conn: &mut SqliteConnection,
        op: &crate::operations::OperationId,
    ) -> Result<Option<String>, StorageError> {
        Ok(sqlx::query_scalar(
            "SELECT thread_id FROM operation WHERE id=? AND started_at IS NOT NULL",
        )
        .bind(op.as_str())
        .fetch_optional(conn)
        .await?
        .flatten())
    }
    pub(crate) async fn started_for_thread_in(
        conn: &mut SqliteConnection,
        thread: &ThreadId,
    ) -> Result<Vec<String>, StorageError> {
        Ok(sqlx::query_scalar(
            "SELECT id FROM operation WHERE thread_id=? AND started_at IS NOT NULL ORDER BY id",
        )
        .bind(thread.as_str())
        .fetch_all(conn)
        .await?)
    }
}

impl Operations {
    pub(crate) async fn require_unfinished_in(
        conn: &mut SqliteConnection,
        op: &crate::operations::OperationId,
    ) -> Result<(), StorageError> {
        let state: Option<String> =
            sqlx::query_scalar("SELECT status_kind FROM operation WHERE id=?")
                .bind(op.as_str())
                .fetch_optional(conn)
                .await?;
        if !matches!(state.as_deref(), Some("Pending" | "Running")) {
            return Err(StorageError::TransitionConflict {
                expected: "Pending or Running".into(),
                found: "already terminal".into(),
            });
        }
        Ok(())
    }
}

impl Operations {
    pub(crate) async fn completed_in_snapshot(
        conn: &mut SqliteConnection,
        op: &str,
    ) -> Result<bool, StorageError> {
        let status: Option<String> =
            sqlx::query_scalar("SELECT status_kind FROM operation WHERE id=?")
                .bind(op)
                .fetch_optional(conn)
                .await?;
        Ok(status.as_deref() == Some("Completed"))
    }
}

impl Operations {
    /// Whether any operation — running or ended — exists on the thread. Its first
    /// one fixes the harness.
    pub(crate) async fn has_any_in(
        conn: &mut SqliteConnection,
        thread: &ThreadId,
    ) -> Result<bool, StorageError> {
        let found: Option<i64> =
            sqlx::query_scalar("SELECT 1 FROM operation WHERE thread_id = ? LIMIT 1")
                .bind(thread.as_str())
                .fetch_optional(&mut *conn)
                .await?;
        Ok(found.is_some())
    }
}
