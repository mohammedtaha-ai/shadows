//! One job: recording a steered message (spec §20.4).

use super::queue::{SCOPE, event, take_queued_in};
use crate::command::CommandContext;
use crate::db::{Storage, StorageError, classify, now, record_command};
use crate::events::Actor;
use crate::threads::{NewThreadEntry, ThreadEntryId, ThreadEntryKind, ThreadId, append_entry_in};
use crate::turns::model::{OperationId, QueuedMessageId, SentNow};

impl Storage {
    /// §20.4: the steered message as a user entry of the running turn, and
    /// its row taken, in one transaction; a replay answers the entry.
    pub async fn steered_entry(
        &self,
        ctx: &CommandContext,
        thread: &ThreadId,
        op: &OperationId,
        row: &QueuedMessageId,
        prompt: &str,
    ) -> Result<ThreadEntryId, StorageError> {
        let (ctx, thread, op, row, prompt, ts) = (
            ctx.clone(),
            thread.clone(),
            op.clone(),
            row.clone(),
            prompt.to_owned(),
            now(),
        );
        self.write_txn(move |conn| {
            Box::pin(async move {
                if let Some(outcome) = classify(conn, &ctx, SCOPE, thread.as_str()).await? {
                    let v: serde_json::Value = serde_json::from_str(&outcome)?;
                    let id = v["entry_id"]
                        .as_str()
                        .ok_or(StorageError::NotFound("entry_id"))?;
                    return Ok(ThreadEntryId::from_stored(id.to_owned()));
                }
                take_queued_in(conn, &thread, &row).await?;
                let entry = append_entry_in(
                    conn,
                    &thread,
                    NewThreadEntry {
                        kind: ThreadEntryKind::UserMessage,
                        author: Actor::user(&ctx.principal_id),
                        body: &prompt,
                        refs: &[],
                        operation_id: Some(&op),
                    },
                    &ts,
                )
                .await?;
                let payload = serde_json::json!({
                    "queued_id": row.as_str(), "how": "steered", "entry_id": entry.id.as_str(),
                });
                let actor = Actor::user(&ctx.principal_id);
                event(conn, "QueuedMessageSent", &thread, actor, payload, &ts).await?;
                let outcome = serde_json::json!({ "entry_id": entry.id.as_str() }).to_string();
                record_command(
                    conn,
                    &ctx,
                    SCOPE,
                    thread.as_str(),
                    "ThreadEntry",
                    &outcome,
                    &ts,
                )
                .await?;
                Ok(entry.id)
            })
        })
        .await
    }

    /// Read-only: what an earlier `turn.send_now` with this command answered.
    pub async fn replayed_send_now(
        &self,
        ctx: &CommandContext,
        thread: &ThreadId,
    ) -> Result<Option<SentNow>, StorageError> {
        let mut conn = self.reader().acquire().await?;
        let Some(outcome) = classify(&mut conn, ctx, SCOPE, thread.as_str()).await? else {
            return Ok(None);
        };
        let v: serde_json::Value = serde_json::from_str(&outcome)?;
        if let Some(op) = v["operation_id"].as_str() {
            let operation_id = OperationId::from_stored(op.to_owned());
            return Ok(Some(SentNow::Started { operation_id }));
        }
        let id = v["entry_id"]
            .as_str()
            .ok_or(StorageError::NotFound("entry_id"))?;
        Ok(Some(SentNow::Steered {
            entry_id: ThreadEntryId::from_stored(id.to_owned()),
        }))
    }
}
