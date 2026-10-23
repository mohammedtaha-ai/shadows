//! One job: the atomic removal of a planning thread.

use crate::db::{Actor, CommandContext, StorageError, append_event, classify, record_command};
use crate::grants::Grants;
use crate::threads::{PlanningThread, ThreadId};

impl crate::threads::Threads {
    /// Mark the thread removed, revoke every Planner grant, and journal the
    /// change in one write. A replay returns the same removed thread.
    pub(crate) async fn remove_in(
        conn: &mut sqlx::SqliteConnection,
        ctx: &CommandContext,
        thread: &ThreadId,
        ts: &str,
    ) -> Result<PlanningThread, StorageError> {
        if classify(conn, ctx, "Thread", thread.as_str())
            .await?
            .is_some()
        {
            return super::ThreadsStore::load_thread(conn, thread).await;
        }
        let current = super::ThreadsStore::load_thread(conn, thread).await?;
        if current.removed_at.is_some() {
            return Err(StorageError::NotFound("planning_thread"));
        }
        crate::projects::Projects::require_live_in(conn, &current.project_id).await?;
        sqlx::query("UPDATE planning_thread SET removed_at = ? WHERE id = ?")
            .bind(ts)
            .bind(thread.as_str())
            .execute(&mut *conn)
            .await?;
        let actor = Actor::user(&ctx.principal_id);
        Grants::revoke_thread_grants_in(conn, thread, actor.clone(), ts).await?;
        append_event(
            conn,
            crate::db::JournalEntry {
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
        record_command(
            conn,
            ctx,
            "Thread",
            thread.as_str(),
            "PlanningThread",
            thread.as_str(),
            ts,
        )
        .await?;
        super::ThreadsStore::load_thread(conn, thread).await
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
