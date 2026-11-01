//! Replayable permission-answer command records.
use super::HarnessStore;
use crate::{HarnessError, PermissionAnswer};
use core_db::{CommandContext, Storage};
use core_threads::ThreadId;

impl HarnessStore {
    pub(crate) async fn permission_replay(
        &self,
        command: &CommandContext,
        thread: &ThreadId,
    ) -> Result<Option<PermissionAnswer>, HarnessError> {
        let mut conn = self.storage.reader().acquire().await?;
        CommandContext::classify(&mut conn, command, "Thread", thread.as_str())
            .await?
            .map(|json| serde_json::from_str(&json).map_err(Into::into))
            .transpose()
    }

    pub(crate) async fn record_permission_answer(
        &self,
        command: CommandContext,
        answer: PermissionAnswer,
    ) -> Result<(PermissionAnswer, bool), HarnessError> {
        self.storage
            .write_txn_as(move |conn| {
                Box::pin(async move {
                    if let Some(json) = CommandContext::classify(
                        conn,
                        &command,
                        "Thread",
                        answer.thread_id.as_str(),
                    )
                    .await?
                    {
                        return Ok((serde_json::from_str(&json)?, false));
                    }
                    CommandContext::record_command(
                        conn,
                        &command,
                        "Thread",
                        answer.thread_id.as_str(),
                        "PermissionAnswer",
                        &serde_json::to_string(&answer)?,
                        &Storage::now(),
                    )
                    .await?;
                    Ok((answer, true))
                })
            })
            .await
    }
}
