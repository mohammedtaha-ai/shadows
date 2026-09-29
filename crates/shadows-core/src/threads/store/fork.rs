//! One job: forking a thread from its last completed entry (spec §12.9).
//!
//! The fork point is decided from durable rows only: the entry is the
//! source's highest-ordinal one, it names an operation, that operation is
//! `Completed`, and the source has a harness session to fork. Widening it to
//! any entry is a change here alone, once the Context Compiler exists.

use sqlx::SqliteConnection;

use super::thread::load_thread;
use crate::command::CommandContext;
use crate::db::{Storage, StorageError, append_event, classify, now, record_command};
use crate::events::{Actor, DurableEvent};
use crate::threads::model::{PlanningThread, ThreadEntryId, ThreadId};
use crate::turns::has_open_operation;

/// The session to fork, when `at` is a valid fork point of `source`.
async fn fork_point(
    conn: &mut SqliteConnection,
    source: &ThreadId,
    at: &ThreadEntryId,
) -> Result<(i64, String), StorageError> {
    let entry: Option<(i64, Option<String>)> = sqlx::query_as(
        "SELECT ordinal, operation_id FROM thread_entry WHERE id = ? AND thread_id = ?",
    )
    .bind(at.as_str())
    .bind(source.as_str())
    .fetch_optional(&mut *conn)
    .await?;
    let (ordinal, operation) = entry.ok_or(StorageError::NotFound("thread_entry"))?;
    let (last, session): (i64, Option<String>) = sqlx::query_as(
        "SELECT next_entry_ordinal - 1, harness_session_id FROM planning_thread WHERE id = ?",
    )
    .bind(source.as_str())
    .fetch_one(&mut *conn)
    .await?;
    let completed = match &operation {
        Some(op) => {
            let status: Option<String> =
                sqlx::query_scalar("SELECT status_kind FROM operation WHERE id = ?")
                    .bind(op)
                    .fetch_optional(&mut *conn)
                    .await?;
            status.as_deref() == Some("Completed")
        }
        None => false,
    };
    match session {
        Some(session) if ordinal == last && completed => Ok((ordinal, session)),
        _ => Err(StorageError::ForkPointNotSupported),
    }
}

impl Storage {
    /// Creates, as one command, a thread in the source's project on its
    /// harness, holding copies of the source's entries up to `at_entry`
    /// under new ids and ordinals, and naming the source, the entry and the
    /// source's harness session. Copied entries keep their `operation_id`
    /// and refs: provenance, read-only. The source is not changed. A replay
    /// answers the fork already made.
    pub async fn fork_thread(
        &self,
        ctx: &CommandContext,
        source: &ThreadId,
        at_entry: &ThreadEntryId,
    ) -> Result<PlanningThread, StorageError> {
        let (ctx, source, at, ts) = (ctx.clone(), source.clone(), at_entry.clone(), now());
        self.write_txn(move |conn| {
            Box::pin(async move {
                if let Some(id) = classify(conn, &ctx, "Thread", source.as_str()).await? {
                    return load_thread(conn, &ThreadId::from_stored(id)).await;
                }
                let original = load_thread(conn, &source).await?;
                if has_open_operation(conn, &source).await? {
                    return Err(StorageError::ThreadBusy);
                }
                let (upto, session) = fork_point(conn, &source, &at).await?;
                let id = ThreadId::generate();
                let title = format!("{} (fork)", original.title);
                sqlx::query(
                    "INSERT INTO planning_thread
                       (id, project_id, title, status, next_entry_ordinal, harness_kind,
                        forked_from_thread, forked_from_entry, fork_session_id, created_at)
                     VALUES (?,?,?, 'Open', ?, ?, ?, ?, ?, ?)",
                )
                .bind(id.as_str())
                .bind(original.project_id.as_str())
                .bind(&title)
                .bind(upto + 1)
                .bind(&original.harness)
                .bind(source.as_str())
                .bind(at.as_str())
                .bind(&session)
                .bind(&ts)
                .execute(&mut *conn)
                .await?;
                append_event(
                    conn,
                    &DurableEvent::new("PlanningThreadCreated", Actor::user(&ctx.principal_id))
                        .with_project(&original.project_id)
                        .with_thread(&id)
                        .with_payload(serde_json::json!({
                            "title": title,
                            "harness": original.harness,
                            "forked_from_thread": source.as_str(),
                            "forked_from_entry": at.as_str(),
                        })),
                    &ts,
                )
                .await?;
                let copied = copy_entries(conn, &source, &id, upto, &ts).await?;
                sqlx::query("UPDATE planning_thread SET next_entry_ordinal = ? WHERE id = ?")
                    .bind(copied + 1)
                    .bind(id.as_str())
                    .execute(&mut *conn)
                    .await?;
                record_command(
                    conn,
                    &ctx,
                    "Thread",
                    source.as_str(),
                    "PlanningThread",
                    id.as_str(),
                    &ts,
                )
                .await?;
                load_thread(conn, &id).await
            })
        })
        .await
    }
}

type CopiedRow = (
    i64,
    String,
    String,
    String,
    String,
    String,
    Option<String>,
    String,
);

/// Copies the source's entries with ordinal up to `upto` into `fork`, in
/// ordinal order, under new ids and ordinals `1..=n`, journaling each as
/// appended to the fork. Answers `n`.
async fn copy_entries(
    conn: &mut SqliteConnection,
    source: &ThreadId,
    fork: &ThreadId,
    upto: i64,
    ts: &str,
) -> Result<i64, StorageError> {
    let rows: Vec<CopiedRow> = sqlx::query_as(
        "SELECT ordinal, kind, author_kind, author_id, body, refs_json, operation_id, created_at
           FROM thread_entry WHERE thread_id = ? AND ordinal <= ? ORDER BY ordinal",
    )
    .bind(source.as_str())
    .bind(upto)
    .fetch_all(&mut *conn)
    .await?;
    let mut ordinal = 0;
    for row in rows {
        ordinal += 1;
        let (_, kind, author_kind, author_id, body, refs_json, operation_id, created_at) = row;
        sqlx::query(
            "INSERT INTO thread_entry
               (id, thread_id, ordinal, kind, author_kind, author_id, body, refs_json,
                operation_id, created_at)
             VALUES (?,?,?,?,?,?,?,?,?,?)",
        )
        .bind(ThreadEntryId::generate().as_str())
        .bind(fork.as_str())
        .bind(ordinal)
        .bind(&kind)
        .bind(&author_kind)
        .bind(&author_id)
        .bind(&body)
        .bind(&refs_json)
        .bind(&operation_id)
        .bind(&created_at)
        .execute(&mut *conn)
        .await?;
        append_event(
            conn,
            &DurableEvent::new(
                "ThreadEntryAppended",
                Actor {
                    kind: author_kind,
                    id: author_id,
                },
            )
            .with_thread(fork)
            .with_payload(serde_json::json!({ "ordinal": ordinal, "kind": kind })),
            ts,
        )
        .await?;
    }
    Ok(ordinal)
}
