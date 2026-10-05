//! One job: the queue's SQLite queries (spec §20.2).

use sqlx::SqliteConnection;

use super::turn::has_open_operation;
use crate::command::CommandContext;
use crate::db::{Storage, StorageError, append_event, classify, now, record_command};
use crate::events::{Actor, DurableEvent};
use crate::plans::{Focus, PlanId};
use crate::threads::ThreadId;
use crate::turns::model::{QueuedMessage, QueuedMessageId};

const SCOPE: &str = "Thread";

pub struct NewQueued<'a> {
    pub prompt: &'a str,
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
);

/// The select every read shares; a `&'static str` because `sqlx` refuses a
/// built one. Each use adds its `WHERE`.
macro_rules! queued_select {
    ($tail:literal) => {
        concat!(
            "SELECT q.id, q.thread_id, q.position, q.prompt, q.model, q.mode,
                    q.effort, q.focus_json, q.plan_id, q.last_error, q.created_at
               FROM queued_message q ",
            $tail
        )
    };
}

fn message(r: Row) -> Result<QueuedMessage, StorageError> {
    Ok(QueuedMessage {
        id: QueuedMessageId::from_stored(r.0),
        thread_id: ThreadId::from_stored(r.1),
        position: r.2,
        prompt: r.3,
        model: r.4,
        mode: r.5,
        effort: r.6,
        focus: r.7.map(|f| serde_json::from_str(&f)).transpose()?,
        plan: r.8.map(PlanId::from_stored),
        last_error: r.9,
        created_at: r.10,
    })
}

async fn event(
    conn: &mut SqliteConnection,
    kind: &str,
    thread: &ThreadId,
    actor: Actor,
    payload: serde_json::Value,
    ts: &str,
) -> Result<(), StorageError> {
    let event = DurableEvent::new(kind, actor)
        .with_thread(thread)
        .with_payload(payload);
    append_event(conn, &event, ts).await.map(|_| ())
}

/// Removes a waiting message inside the caller's transaction, or answers
/// `QueuedMessageGone`: of two writers racing for one row, one wins.
pub(crate) async fn take_queued_in(
    conn: &mut SqliteConnection,
    thread: &ThreadId,
    id: &QueuedMessageId,
) -> Result<(), StorageError> {
    let gone = sqlx::query("DELETE FROM queued_message WHERE id = ? AND thread_id = ?")
        .bind(id.as_str())
        .bind(thread.as_str())
        .execute(&mut *conn)
        .await?
        .rows_affected()
        == 0;
    if gone {
        return Err(StorageError::QueuedMessageGone);
    }
    Ok(())
}

impl Storage {
    /// §20.2: a replay answers the message as first answered; a removed
    /// thread is `NotFound`; an idle thread writes nothing and is `Idle`;
    /// a busy one gets the row, last in `position` order.
    pub async fn queue_message(
        &self,
        ctx: &CommandContext,
        thread: &ThreadId,
        new: NewQueued<'_>,
    ) -> Result<QueueAnswer, StorageError> {
        let (ctx, thread, ts) = (ctx.clone(), thread.clone(), now());
        let id = QueuedMessageId::generate();
        let fields = (
            new.prompt.to_owned(),
            new.model.to_owned(),
            new.mode.to_owned(),
            new.effort.map(str::to_owned),
            new.focus.map(serde_json::to_string).transpose()?,
            new.plan.map(|p| p.as_str().to_owned()),
        );
        self.write_txn(move |conn| {
            Box::pin(async move {
                if let Some(outcome) = classify(conn, &ctx, SCOPE, thread.as_str()).await? {
                    return Ok(QueueAnswer::Waiting(serde_json::from_str(&outcome)?));
                }
                let live: Option<i64> = sqlx::query_scalar(
                    "SELECT 1 FROM planning_thread WHERE id = ? AND removed_at IS NULL",
                )
                .bind(thread.as_str())
                .fetch_optional(&mut *conn)
                .await?;
                live.ok_or(StorageError::NotFound("planning_thread"))?;
                if !has_open_operation(conn, &thread).await? {
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
                        focus_json, plan_id, last_error, created_at)
                     VALUES (?,?,?,?,?,?,?,?,?,NULL,?)",
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
                .execute(&mut *conn)
                .await?;
                let row: Row = sqlx::query_as(queued_select!("WHERE q.id = ?"))
                    .bind(id.as_str())
                    .fetch_one(&mut *conn)
                    .await?;
                let queued = message(row)?;
                let actor = Actor::user(&ctx.principal_id);
                let payload = serde_json::json!({ "queued_id": id.as_str() });
                event(conn, "MessageQueued", &thread, actor, payload, &ts).await?;
                let outcome = serde_json::to_string(&queued)?;
                record_command(
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
        let rows: Vec<Row> = sqlx::query_as(queued_select!(
            "JOIN planning_thread t ON t.id = q.thread_id AND t.removed_at IS NULL
              WHERE q.thread_id = ? ORDER BY q.position"
        ))
        .bind(thread.as_str())
        .fetch_all(self.reader())
        .await?;
        rows.into_iter().map(message).collect()
    }

    #[cfg_attr(
        not(feature = "test-support"),
        expect(
            dead_code,
            reason = "no product caller until the send path; only tests use it"
        )
    )]
    pub async fn queued_message(
        &self,
        thread: &ThreadId,
        id: &QueuedMessageId,
    ) -> Result<Option<QueuedMessage>, StorageError> {
        let row: Option<Row> = sqlx::query_as(queued_select!(
            "JOIN planning_thread t ON t.id = q.thread_id AND t.removed_at IS NULL
              WHERE q.thread_id = ? AND q.id = ?"
        ))
        .bind(thread.as_str())
        .bind(id.as_str())
        .fetch_optional(self.reader())
        .await?;
        row.map(message).transpose()
    }

    /// §20.5: a replay is judged first; a new command on a gone row is
    /// `QueuedMessageGone`.
    pub async fn unqueue_message(
        &self,
        ctx: &CommandContext,
        thread: &ThreadId,
        id: &QueuedMessageId,
    ) -> Result<(), StorageError> {
        let (ctx, thread, id, ts) = (ctx.clone(), thread.clone(), id.clone(), now());
        self.write_txn(move |conn| {
            Box::pin(async move {
                if classify(conn, &ctx, SCOPE, thread.as_str())
                    .await?
                    .is_some()
                {
                    return Ok(());
                }
                take_queued_in(conn, &thread, &id).await?;
                let actor = Actor::user(&ctx.principal_id);
                let payload = serde_json::json!({ "queued_id": id.as_str() });
                event(conn, "QueuedMessageRemoved", &thread, actor, payload, &ts).await?;
                record_command(
                    conn,
                    &ctx,
                    SCOPE,
                    thread.as_str(),
                    "QueuedMessage",
                    "{}",
                    &ts,
                )
                .await
            })
        })
        .await
    }

    /// §20.3, §20.4: why the last send of a waiting message failed.
    pub async fn fail_queued(
        &self,
        thread: &ThreadId,
        id: &QueuedMessageId,
        reason: &str,
    ) -> Result<(), StorageError> {
        let (thread, id, reason, ts) = (thread.clone(), id.clone(), reason.to_owned(), now());
        self.write_txn(move |conn| {
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
                let payload = serde_json::json!({ "queued_id": id.as_str(), "last_error": reason });
                event(
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
