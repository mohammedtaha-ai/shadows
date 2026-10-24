//! One job: the atomic removal of a planning thread.

use crate::threads::{PlanningThread, ThreadId};
use core_db::{Actor, CommandContext, JournalEntry, StorageError};

impl crate::threads::Threads {
    /// Marks the live thread removed inside the composing removal transaction.
    pub(crate) async fn remove_in(
        conn: &mut sqlx::SqliteConnection,
        thread: &ThreadId,
        ts: &str,
    ) -> Result<PlanningThread, StorageError> {
        let current = super::ThreadsStore::load_thread(conn, thread).await?;
        if current.removed_at.is_some() {
            return Err(StorageError::NotFound("planning_thread"));
        }
        core_projects::Projects::require_live_in(conn, &current.project_id).await?;
        sqlx::query("UPDATE planning_thread SET removed_at = ? WHERE id = ?")
            .bind(ts)
            .bind(thread.as_str())
            .execute(&mut *conn)
            .await?;
        sqlx::query("DELETE FROM thread_attachment WHERE thread_id=?")
            .bind(thread.as_str())
            .execute(&mut *conn)
            .await?;
        sqlx::query(
            "DELETE FROM attachment WHERE NOT EXISTS
            (SELECT 1 FROM thread_attachment t WHERE t.attachment_id=attachment.id)",
        )
        .execute(&mut *conn)
        .await?;
        super::ThreadsStore::load_thread(conn, thread).await
    }

    /// Journals and records the removal after its grants were revoked.
    pub(crate) async fn record_removal_in(
        conn: &mut sqlx::SqliteConnection,
        ctx: &CommandContext,
        current: &PlanningThread,
        ts: &str,
    ) -> Result<(), StorageError> {
        let thread = &current.id;
        let actor = Actor::user(&ctx.principal_id);
        JournalEntry::append_event(
            conn,
            core_db::JournalEntry {
                event_id: uuid::Uuid::new_v4().to_string(),
                kind: "ThreadRemoved".into(),
                actor,
                causation: None,
                correlation_id: None,
                project_id: Some(current.project_id.as_str().to_owned()),
                thread_id: Some(thread.as_str().to_owned()),
                operation_id: None,
                payload_json: "{}".into(),
            },
            ts,
        )
        .await?;
        CommandContext::record_command(
            conn,
            ctx,
            "Thread",
            thread.as_str(),
            "PlanningThread",
            thread.as_str(),
            ts,
        )
        .await?;
        Ok(())
    }
}

impl super::ThreadsStore {
    /// Removed threads remain readable for plan attribution and history.
    pub(in crate::threads) async fn get_thread(
        &self,
        thread: &ThreadId,
    ) -> Result<PlanningThread, StorageError> {
        let mut conn = self.storage.reader().acquire().await?;
        super::ThreadsStore::load_thread(&mut conn, thread).await
    }
}
