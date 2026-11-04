//! Immutable planning context records with a session acknowledgement.
use super::HarnessStore;
use crate::{CarriedContext, Forget, ThreadId};
use core_db::{Actor, CommandContext, JournalEntry, Storage, StorageError};
use core_operations::{OperationId, Operations};
use core_threads::Threads;

impl HarnessStore {
    pub async fn mark_context_heard_in(
        conn: &mut core_db::Tx,
        op: &OperationId,
    ) -> Result<(), StorageError> {
        sqlx::query("UPDATE invocation_context SET heard=1 WHERE operation_id=?")
            .bind(op.as_str())
            .execute(conn)
            .await?;
        Ok(())
    }
    pub async fn prepare_context(
        &self,
        op: &OperationId,
        thread: &ThreadId,
        session: &str,
        carried: &CarriedContext,
    ) -> Result<(), StorageError> {
        let (op, thread, session, carried) = (
            op.clone(),
            thread.clone(),
            session.to_owned(),
            carried.clone(),
        );
        self.storage
            .write_txn(move |conn| {
                Box::pin(async move {
                    let operation = Operations::get_operation_in(conn, &op).await?;
                    if operation.thread_id.as_deref() != Some(thread.as_str()) || session.is_empty()
                    {
                        return Err(StorageError::Constraint(
                            "context does not match its invocation".into(),
                        ));
                    }
                    sqlx::query(
                        "INSERT INTO invocation_context(operation_id,thread_id,session_id, \
                start_seq,digest,text,sources_json,index_tokens,detail_tokens,over_tokens, \
                cuts_json,created_at) VALUES (?,?,?,?,?,?,?,?,?,?,?,?)",
                    )
                    .bind(op.as_str())
                    .bind(thread.as_str())
                    .bind(session)
                    .bind(operation.seq)
                    .bind(&carried.digest)
                    .bind(&carried.text)
                    .bind(serde_json::to_string(&carried.sources)?)
                    .bind(carried.index_tokens)
                    .bind(carried.detail_tokens)
                    .bind(carried.over_tokens)
                    .bind(serde_json::to_string(&carried.cuts)?)
                    .bind(Storage::now())
                    .execute(conn)
                    .await?;
                    Ok(())
                })
            })
            .await
    }

    pub async fn mark_context_heard(&self, op: &OperationId) -> Result<(), StorageError> {
        let op = op.clone();
        self.storage
            .write_txn(move |conn| {
                Box::pin(async move { Self::mark_context_heard_in(conn, &op).await })
            })
            .await
    }

    pub async fn heard_context_digest(
        &self,
        session: &str,
    ) -> Result<Option<String>, StorageError> {
        Ok(sqlx::query_scalar(
            "SELECT digest FROM invocation_context \
            WHERE session_id=? AND heard=1 AND start_seq > COALESCE( \
              (SELECT MAX(seq) FROM context_forget f \
                WHERE f.thread_id=invocation_context.thread_id), 0) \
            ORDER BY start_seq DESC LIMIT 1",
        )
        .bind(session)
        .fetch_optional(self.storage.reader())
        .await?)
    }

    pub(crate) async fn reader_conn(
        &self,
    ) -> Result<sqlx::pool::PoolConnection<sqlx::Sqlite>, StorageError> {
        Ok(self.storage.reader().acquire().await?)
    }

    /// Records the forget with its journal event. `false` for a replayed
    /// request, which wrote nothing.
    pub async fn forget_context(
        &self,
        thread: &ThreadId,
        forget: &Forget,
    ) -> Result<bool, StorageError> {
        let (thread, forget, ts) = (thread.clone(), forget.clone(), Storage::now());
        self.storage
            .write_txn(move |conn| {
                Box::pin(async move {
                    let (reason, trigger, operation, ctx) = match &forget {
                        Forget::Compaction { operation, trigger } => {
                            ("compaction", trigger.clone(), Some(operation.clone()), None)
                        }
                        Forget::Request { command_id } => {
                            let ctx = CommandContext::user(
                                command_id.clone(),
                                "thread.context_resend",
                                serde_json::json!({ "thread": thread.as_str() }),
                            );
                            if CommandContext::classify(conn, &ctx, "Thread", thread.as_str())
                                .await?
                                .is_some()
                            {
                                return Ok(false);
                            }
                            ("request", None, None, Some(ctx))
                        }
                    };
                    let project = Threads::project_in(conn, &thread).await?;
                    let actor = match &ctx {
                        Some(ctx) => Actor::user(&ctx.principal_id),
                        None => Actor::system(),
                    };
                    let mut event = JournalEntry::new("PlannerContextForgotten", actor)
                        .with_project(project.as_str())
                        .with_thread(thread.as_str())
                        .with_payload(serde_json::json!({ "reason": reason, "trigger": trigger }));
                    if let Some(op) = &operation {
                        event = event.with_operation(op.as_str());
                    }
                    let seq = JournalEntry::append_event(conn, event, &ts).await?;
                    sqlx::query(
                        "INSERT INTO context_forget(seq,thread_id,operation_id,reason,trigger, \
                        created_at) VALUES (?,?,?,?,?,?)",
                    )
                    .bind(seq)
                    .bind(thread.as_str())
                    .bind(operation.as_ref().map(OperationId::as_str))
                    .bind(reason)
                    .bind(&trigger)
                    .bind(&ts)
                    .execute(&mut *conn)
                    .await?;
                    if let Some(ctx) = &ctx {
                        CommandContext::record_command(
                            conn,
                            ctx,
                            "Thread",
                            thread.as_str(),
                            "ContextForget",
                            &seq.to_string(),
                            &ts,
                        )
                        .await?;
                    }
                    Ok(true)
                })
            })
            .await
    }

    pub async fn context_record(
        &self,
        op: &OperationId,
    ) -> Result<Option<(CarriedContext, bool)>, StorageError> {
        type Row = (String, String, String, i64, i64, i64, String, bool);
        let row: Option<Row> = sqlx::query_as(
            "SELECT digest,text,sources_json,index_tokens, \
            detail_tokens,over_tokens,cuts_json,heard FROM invocation_context WHERE operation_id=?",
        )
        .bind(op.as_str())
        .fetch_optional(self.storage.reader())
        .await?;
        row.map(
            |(digest, text, sources, index_tokens, detail_tokens, over_tokens, cuts, heard)| {
                Ok((
                    CarriedContext {
                        digest,
                        text,
                        sources: serde_json::from_str(&sources)?,
                        index_tokens,
                        detail_tokens,
                        over_tokens,
                        cuts: serde_json::from_str(&cuts)?,
                    },
                    heard,
                ))
            },
        )
        .transpose()
    }
}
