//! One job: the queue's SQLite queries.

use sqlx::SqliteConnection;

use crate::operations::Operations;
use crate::plans::{Focus, PlanId};
use crate::turns::model::{QueuedMessage, QueuedMessageId};
use core_db::{Actor, CommandContext, JournalEntry, Storage, StorageError};
use core_threads::ThreadId;

pub(super) const SCOPE: &str = "Thread";

pub struct NewQueued<'a> {
    pub prompt: &'a str,
    pub attachments: &'a [core_threads::AttachmentId],
    pub model: &'a str,
    pub mode: &'a str,
    pub effort: Option<&'a str>,
    pub focus: Option<&'a Focus>,
    pub plan: Option<&'a PlanId>,
}

#[expect(
    clippy::large_enum_variant,
    reason = "one answer per request; boxing buys nothing"
)]
pub enum QueueAnswer {
    Waiting(QueuedMessage),
    /// The thread runs no turn: nothing was written; the caller starts one.
    Idle,
}

type Row = (
    String,
    String,
    i64,
    String,
    String,
    String,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    String,
    String,
);

fn message(r: Row) -> Result<QueuedMessage, StorageError> {
    Ok(QueuedMessage {
        id: QueuedMessageId::from_stored(r.0),
        thread_id: ThreadId::from_recorded(r.1),
        position: r.2,
        prompt: r.3,
        attachments: serde_json::from_str(&r.11)?,
        model: r.4,
        mode: r.5,
        effort: r.6,
        focus: r.7.map(|f| serde_json::from_str(&f)).transpose()?,
        plan: r.8.map(PlanId::from_stored),
        last_error: r.9,
        created_at: r.10,
    })
}

impl super::TurnsStore {
    pub(super) async fn event(
        conn: &mut SqliteConnection,
        kind: &str,
        thread: &ThreadId,
        actor: Actor,
        payload: serde_json::Value,
        ts: &str,
    ) -> Result<(), StorageError> {
        let event = JournalEntry::new(kind, actor)
            .with_thread(thread.as_str())
            .with_payload(payload);
        JournalEntry::append_event(conn, event.clone(), ts)
            .await
            .map(|_| ())
    }
}

/// Removes a waiting message inside the caller's transaction, or answers
/// `QueuedMessageGone`: of two writers racing for one row, one wins.
impl super::TurnsStore {
    pub(crate) async fn take_queued_in(
        conn: &mut SqliteConnection,
        thread: &ThreadId,
        id: &QueuedMessageId,
    ) -> Result<(), crate::CoreError> {
        let gone = sqlx::query("DELETE FROM queued_message WHERE id = ? AND thread_id = ?")
            .bind(id.as_str())
            .bind(thread.as_str())
            .execute(&mut *conn)
            .await?
            .rows_affected()
            == 0;
        if gone {
            return Err(crate::turns::error::TurnsError::QueuedMessageGone.into());
        }
        Ok(())
    }
}

impl super::TurnsStore {
    /// A replay answers the message as first answered; a removed
    /// thread is `NotFound`; an idle thread writes nothing and is `Idle`;
    /// a busy one gets the row, last in `position` order.
    pub async fn queue_message(
        &self,
        ctx: &CommandContext,
        thread: &ThreadId,
        new: NewQueued<'_>,
    ) -> Result<QueueAnswer, StorageError> {
        let (ctx, thread, ts) = (ctx.clone(), thread.clone(), Storage::now());
        let id = QueuedMessageId::generate();
        let attachments = new.attachments.to_vec();
        let fields = (
            new.prompt.to_owned(),
            new.model.to_owned(),
            new.mode.to_owned(),
            new.effort.map(str::to_owned),
            new.focus.map(serde_json::to_string).transpose()?,
            new.plan.map(|p| p.as_str().to_owned()),
        );
        self.storage
            .write_txn(move |conn| {
                Box::pin(async move {
                    if let Some(outcome) =
                        CommandContext::classify(conn, &ctx, SCOPE, thread.as_str()).await?
                    {
                        return Ok(QueueAnswer::Waiting(serde_json::from_str(&outcome)?));
                    }
                    core_threads::Threads::require_live_in(conn, &thread).await?;
                    core_threads::Threads::attachment_refs_in(conn, &thread, &attachments).await?;
                    if !Operations::has_open_in(conn, &thread).await? {
                        return Ok(QueueAnswer::Idle);
                    }
                    let position: i64 = sqlx::query_scalar(
                    "SELECT COALESCE(MAX(position), 0) + 1 FROM queued_message WHERE thread_id = ?",
                )
                .bind(thread.as_str())
                .fetch_one(&mut *conn)
                .await?;
                    let (prompt, model, mode, effort, focus, plan) = &fields;
                    sqlx::query(
                        "INSERT INTO queued_message
                       (id, thread_id, position, prompt, model, mode, effort,
                        focus_json, plan_id, last_error, created_at, attachments_json)
                     VALUES (?,?,?,?,?,?,?,?,?,NULL,?,?)",
                    )
                    .bind(id.as_str())
                    .bind(thread.as_str())
                    .bind(position)
                    .bind(prompt)
                    .bind(model)
                    .bind(mode)
                    .bind(effort)
                    .bind(focus)
                    .bind(plan)
                    .bind(&ts)
                    .bind(serde_json::to_string(&attachments)?)
                    .execute(&mut *conn)
                    .await?;
                    let row: Row = sqlx::query_as(
                        "SELECT id,thread_id,position,prompt,model,mode,effort,focus_json,plan_id,
                        last_error,created_at,attachments_json FROM queued_message WHERE id=?",
                    )
                    .bind(id.as_str())
                    .fetch_one(&mut *conn)
                    .await?;
                    let queued = message(row)?;
                    let actor = Actor::user(&ctx.principal_id);
                    let payload = serde_json::json!({ "queued_id": id.as_str() });
                    Self::event(conn, "MessageQueued", &thread, actor, payload, &ts).await?;
                    let outcome = serde_json::to_string(&queued)?;
                    CommandContext::record_command(
                        conn,
                        &ctx,
                        SCOPE,
                        thread.as_str(),
                        "QueuedMessage",
                        &outcome,
                        &ts,
                    )
                    .await?;
                    Ok(QueueAnswer::Waiting(queued))
                })
            })
            .await
    }

    /// The thread's waiting messages in `position` order; none for a removed thread.
    pub async fn queued_messages(
        &self,
        thread: &ThreadId,
    ) -> Result<Vec<QueuedMessage>, StorageError> {
        let mut conn = self.storage.reader().begin().await?;
        if !core_threads::Threads::is_live_in(&mut conn, thread).await? {
            return Ok(Vec::new());
        }
        let rows: Vec<Row> = sqlx::query_as(
            "SELECT id,thread_id,position,prompt,model,mode,effort,focus_json,plan_id,
                last_error,created_at,attachments_json FROM queued_message
                WHERE thread_id=? ORDER BY position",
        )
        .bind(thread.as_str())
        .fetch_all(&mut *conn)
        .await?;
        rows.into_iter().map(message).collect()
    }

    pub async fn queued_message(
        &self,
        thread: &ThreadId,
        id: &QueuedMessageId,
    ) -> Result<Option<QueuedMessage>, StorageError> {
        let mut conn = self.storage.reader().begin().await?;
        if !core_threads::Threads::is_live_in(&mut conn, thread).await? {
            return Ok(None);
        }
        let row: Option<Row> = sqlx::query_as(
            "SELECT id,thread_id,position,prompt,model,mode,effort,focus_json,plan_id,
                last_error,created_at,attachments_json FROM queued_message
                WHERE thread_id=? AND id=?",
        )
        .bind(thread.as_str())
        .bind(id.as_str())
        .fetch_optional(&mut *conn)
        .await?;
        row.map(message).transpose()
    }

    /// A replay is judged first; a new command on a gone row is
    /// `QueuedMessageGone`.
    pub async fn unqueue_message(
        &self,
        ctx: &CommandContext,
        thread: &ThreadId,
        id: &QueuedMessageId,
    ) -> Result<(), crate::CoreError> {
        let (ctx, thread, id, ts) = (ctx.clone(), thread.clone(), id.clone(), Storage::now());
        self.storage
            .write_txn_as(move |conn| {
                Box::pin(async move {
                    if CommandContext::classify(conn, &ctx, SCOPE, thread.as_str())
                        .await?
                        .is_some()
                    {
                        return Ok(());
                    }
                    Self::take_queued_in(conn, &thread, &id).await?;
                    let actor = Actor::user(&ctx.principal_id);
                    let payload = serde_json::json!({ "queued_id": id.as_str() });
                    Self::event(conn, "QueuedMessageRemoved", &thread, actor, payload, &ts).await?;
                    Ok(CommandContext::record_command(
                        conn,
                        &ctx,
                        SCOPE,
                        thread.as_str(),
                        "QueuedMessage",
                        "{}",
                        &ts,
                    )
                    .await?)
                })
            })
            .await
    }

    /// Why the last send of a waiting message failed.
    pub async fn fail_queued(
        &self,
        thread: &ThreadId,
        id: &QueuedMessageId,
        reason: &str,
    ) -> Result<(), StorageError> {
        let (thread, id, reason, ts) = (
            thread.clone(),
            id.clone(),
            reason.to_owned(),
            Storage::now(),
        );
        self.storage
            .write_txn(move |conn| {
                Box::pin(async move {
                    let found = sqlx::query(
                        "UPDATE queued_message SET last_error = ? WHERE id = ? AND thread_id = ?",
                    )
                    .bind(&reason)
                    .bind(id.as_str())
                    .bind(thread.as_str())
                    .execute(&mut *conn)
                    .await?
                    .rows_affected();
                    if found == 0 {
                        return Ok(());
                    }
                    let payload =
                        serde_json::json!({ "queued_id": id.as_str(), "last_error": reason });
                    Self::event(
                        conn,
                        "QueuedMessageFailed",
                        &thread,
                        Actor::system(),
                        payload,
                        &ts,
                    )
                    .await
                })
            })
            .await
    }
}

impl super::TurnsStore {
    pub(in crate::turns) async fn replayed_queue(
        &self,
        ctx: &CommandContext,
        thread: &ThreadId,
    ) -> Result<Option<QueuedMessage>, StorageError> {
        let mut conn = self.storage.reader().acquire().await?;
        CommandContext::classify(&mut conn, ctx, SCOPE, thread.as_str())
            .await?
            .map(|answer| serde_json::from_str(&answer).map_err(StorageError::from))
            .transpose()
    }
}
