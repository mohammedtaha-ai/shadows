//! One job: recording a steered message.

use super::queue::SCOPE;
use crate::model::{OperationId, QueuedMessageId, SentNow};
use core_db::{Actor, CommandContext, Storage, StorageError};
use core_threads::{NewThreadEntry, ThreadEntryId, ThreadEntryKind, ThreadId, Threads};

impl super::TurnsStore {
    /// The steered message as a user entry of the running turn, and
    /// its row taken, in one transaction; a replay answers the entry.
    pub async fn steered_entry(
        &self,
        ctx: &CommandContext,
        thread: &ThreadId,
        op: &OperationId,
        row: &QueuedMessageId,
        prompt: &str,
        attachments: &[core_threads::AttachmentId],
    ) -> Result<ThreadEntryId, crate::TurnsError> {
        let (ctx, thread, op, row, prompt, ts) = (
            ctx.clone(),
            thread.clone(),
            op.clone(),
            row.clone(),
            prompt.to_owned(),
            Storage::now(),
        );
        let attachments = attachments.to_vec();
        self.storage
            .write_txn_as(move |conn| {
                Box::pin(async move {
                    if let Some(outcome) =
                        CommandContext::classify(conn, &ctx, SCOPE, thread.as_str()).await?
                    {
                        let v: serde_json::Value = serde_json::from_str(&outcome)?;
                        let id = v["entry_id"]
                            .as_str()
                            .ok_or(StorageError::NotFound("entry_id"))?;
                        return Ok(ThreadEntryId::from_recorded(id.to_owned()));
                    }
                    Threads::attachment_refs_in(conn, &thread, &attachments).await?;
                    Self::take_queued_in(conn, &thread, &row).await?;
                    let entry = Threads::append_entry_with_attachments_in(
                        conn,
                        &thread,
                        NewThreadEntry {
                            kind: ThreadEntryKind::UserMessage,
                            author: Actor::user(&ctx.principal_id),
                            body: &prompt,
                            refs: &[],
                            card: None,
                            operation_id: Some(op.as_str()),
                        },
                        &attachments,
                        &ts,
                    )
                    .await?;
                    let payload = serde_json::json!({
                        "queued_id": row.as_str(), "how": "steered", "entry_id": entry.id.as_str(),
                    });
                    let actor = Actor::user(&ctx.principal_id);
                    Self::event(conn, "QueuedMessageSent", &thread, actor, payload, &ts).await?;
                    let outcome = serde_json::json!({ "entry_id": entry.id.as_str() }).to_string();
                    CommandContext::record_command(
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
        let mut conn = self.storage.reader().acquire().await?;
        let Some(outcome) =
            CommandContext::classify(&mut conn, ctx, SCOPE, thread.as_str()).await?
        else {
            return Ok(None);
        };
        let v: serde_json::Value = serde_json::from_str(&outcome)?;
        if let Some(op) = v["operation_id"].as_str() {
            let operation_id = OperationId::from_recorded(op.to_owned());
            return Ok(Some(SentNow::Started { operation_id }));
        }
        let id = v["entry_id"]
            .as_str()
            .ok_or(StorageError::NotFound("entry_id"))?;
        Ok(Some(SentNow::Steered {
            entry_id: ThreadEntryId::from_recorded(id.to_owned()),
        }))
    }
}
