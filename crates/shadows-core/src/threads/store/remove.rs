//! One job: the atomic removal of a planning thread (§16.5).

use super::thread::load_thread;
use crate::command::CommandContext;
use crate::db::{Storage, StorageError, append_event, classify, now, record_command};
use crate::events::{Actor, DurableEvent};
use crate::grants::revoke_thread_grants_in;
use crate::threads::{PlanningThread, ThreadId};

impl Storage {
    /// Mark the thread removed, revoke every Planner grant, and journal the
    /// change in one write. A replay returns the same removed thread.
    pub async fn remove_thread(
        &self,
        ctx: &CommandContext,
        thread: &ThreadId,
    ) -> Result<PlanningThread, StorageError> {
        let (ctx, thread, ts) = (ctx.clone(), thread.clone(), now());
        self.write_txn(move |conn| {
            Box::pin(async move {
                if classify(conn, &ctx, "Thread", thread.as_str())
                    .await?
                    .is_some()
                {
                    return load_thread(conn, &thread).await;
                }
                let current = load_thread(conn, &thread).await?;
                if current.removed_at.is_some() {
                    return Err(StorageError::NotFound("planning_thread"));
                }
                let project_live: Option<i64> =
                    sqlx::query_scalar("SELECT 1 FROM project WHERE id = ? AND removed_at IS NULL")
                        .bind(current.project_id.as_str())
                        .fetch_optional(&mut *conn)
                        .await?;
                project_live.ok_or(StorageError::NotFound("project"))?;
                sqlx::query("UPDATE planning_thread SET removed_at = ? WHERE id = ?")
                    .bind(&ts)
                    .bind(thread.as_str())
                    .execute(&mut *conn)
                    .await?;
                let actor = Actor::user(&ctx.principal_id);
                revoke_thread_grants_in(conn, &thread, actor.clone(), &ts).await?;
                append_event(
                    conn,
                    &DurableEvent::new("ThreadRemoved", actor)
                        .with_project(&current.project_id)
                        .with_thread(&thread),
                    &ts,
                )
                .await?;
                record_command(
                    conn,
                    &ctx,
                    "Thread",
                    thread.as_str(),
                    "PlanningThread",
                    thread.as_str(),
                    &ts,
                )
                .await?;
                load_thread(conn, &thread).await
            })
        })
        .await
    }

    /// Removed threads remain readable for plan attribution and history.
    pub async fn get_thread(&self, thread: &ThreadId) -> Result<PlanningThread, StorageError> {
        let mut conn = self.reader().acquire().await?;
        load_thread(&mut conn, thread).await
    }
}
