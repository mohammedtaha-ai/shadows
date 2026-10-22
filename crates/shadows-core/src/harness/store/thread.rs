//! Atomic harness selection over Threads and Operations owner methods.
use super::HarnessStore;
use crate::{
    db::{CommandContext, StorageError, classify, now, record_command},
    operations::Operations,
    threads::{PlanningThread, ThreadId, Threads, ThreadsError},
};

impl HarnessStore {
    pub(in crate::harness) async fn set_thread_harness(
        &self,
        ctx: &CommandContext,
        thread: &ThreadId,
        harness: &str,
    ) -> Result<PlanningThread, ThreadsError> {
        let (ctx, thread, harness, ts) = (ctx.clone(), thread.clone(), harness.to_owned(), now());
        self.storage
            .write_txn_as(move |conn| {
                Box::pin(async move {
                    if classify(conn, &ctx, "Thread", thread.as_str())
                        .await?
                        .is_some()
                    {
                        return Ok(Threads::get_in(conn, &thread).await?);
                    }
                    let current = Threads::get_in(conn, &thread).await?;
                    if current.removed_at.is_some() {
                        return Err(StorageError::NotFound("planning_thread").into());
                    }
                    if current.forked_from_thread.is_some()
                        || Operations::has_any_in(conn, &thread).await?
                    {
                        return Err(ThreadsError::HarnessLocked);
                    }
                    let updated = Threads::set_harness_in(
                        conn,
                        &thread,
                        &current.project_id,
                        &harness,
                        crate::db::Actor::user(&ctx.principal_id),
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
                    Ok(updated)
                })
            })
            .await
    }
}
