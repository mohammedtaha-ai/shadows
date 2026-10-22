//! One job: forking a thread from its last completed entry.
//!
//! The fork point is decided from durable rows only: the entry is the
//! source's highest-ordinal one, it names an operation, that operation is
//! `Completed`, and the source has a harness session to fork. Widening it to
//! any entry is a change here alone, once the Context Compiler exists.

use sqlx::SqliteConnection;

use crate::db::{Actor, CommandContext, StorageError, append_event, record_command};
use crate::threads::model::{PlanningThread, ThreadEntryId, ThreadId};

/// The session to fork, when `at` is a valid fork point of `source`.
async fn fork_point(
    conn: &mut SqliteConnection,
    source: &ThreadId,
    at: &ThreadEntryId,
    completed: bool,
) -> Result<(i64, String), crate::threads::ThreadsError> {
    let entry: Option<(i64, Option<String>)> = sqlx::query_as(
        "SELECT ordinal, operation_id FROM thread_entry WHERE id = ? AND thread_id = ?",
    )
    .bind(at.as_str())
    .bind(source.as_str())
    .fetch_optional(&mut *conn)
    .await?;
    let (ordinal, _operation) = entry.ok_or(StorageError::NotFound("thread_entry"))?;
    let (last, session): (i64, Option<String>) = sqlx::query_as(
        "SELECT next_entry_ordinal - 1, harness_session_id FROM planning_thread WHERE id = ?",
    )
    .bind(source.as_str())
    .fetch_one(&mut *conn)
    .await?;
    match session {
        Some(session) if ordinal == last && completed => Ok((ordinal, session)),
        _ => Err(crate::threads::ThreadsError::ForkPointNotSupported),
    }
}

impl crate::threads::Threads {
    /// Creates, as one command, a thread in the source's project on its
    /// harness, holding copies of the source's entries up to `at_entry`
    /// under new ids and ordinals, and naming the source, the entry and the
    /// source's harness session. Copied entries keep their `operation_id`
    /// and refs: provenance, read-only. The source is not changed. A replay
    /// answers the fork already made.
    pub(crate) async fn fork_in(
        conn: &mut SqliteConnection,
        ctx: &CommandContext,
        source: &ThreadId,
        at: &ThreadEntryId,
        completed: bool,
        ts: &str,
    ) -> Result<PlanningThread, crate::threads::ThreadsError> {
        let original = super::ThreadsStore::load_thread(conn, source).await?;
        if original.removed_at.is_some() {
            return Err(StorageError::NotFound("planning_thread").into());
        }
        let (upto, session) = fork_point(conn, source, at, completed).await?;
        let id = ThreadId::generate();
        // The fork's title keeps its source's `title_source`:
        // a name a person gave is still theirs, and the harness may
        // title the fork's own session as it would its source's.
        let title = format!("{} (fork)", original.title);
        sqlx::query(
            "INSERT INTO planning_thread
                       (id, project_id, title, title_source, status, next_entry_ordinal,
                        harness_kind, forked_from_thread, forked_from_entry, fork_session_id,
                        created_at, profile)
                     VALUES (?,?,?, (SELECT title_source FROM planning_thread WHERE id = ?),
                             'Open', ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(id.as_str())
        .bind(original.project_id.as_str())
        .bind(&title)
        .bind(source.as_str())
        .bind(upto + 1)
        .bind(&original.harness)
        .bind(source.as_str())
        .bind(at.as_str())
        .bind(&session)
        .bind(ts)
        .bind(&original.profile)
        .execute(&mut *conn)
        .await?;
        append_event(
            conn,
            crate::db::JournalEntry {
                event_id: uuid::Uuid::new_v4().to_string(),
                kind: "PlanningThreadCreated".into(),
                actor: Actor::user(&ctx.principal_id),
                causation: None,
                correlation_id: None,
                project_id: Some(original.project_id.as_str().to_owned()),
                thread_id: Some(id.as_str().to_owned()),
                operation_id: None,
                payload_json: serde_json::json!({
                    "title": title,
                    "harness": original.harness,
                    "forked_from_thread": source.as_str(),
                    "forked_from_entry": at.as_str(),
                })
                .to_string(),
            },
            ts,
        )
        .await?;
        let copied = copy_entries(conn, source, &id, upto, ts).await?;
        sqlx::query("UPDATE planning_thread SET next_entry_ordinal = ? WHERE id = ?")
            .bind(copied + 1)
            .bind(id.as_str())
            .execute(&mut *conn)
            .await?;
        record_command(
            conn,
            ctx,
            "Thread",
            source.as_str(),
            "PlanningThread",
            id.as_str(),
            ts,
        )
        .await?;
        Ok(super::ThreadsStore::load_thread(conn, &id).await?)
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
    Option<String>,
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
        "SELECT ordinal, kind, author_kind, author_id, body, refs_json, operation_id, created_at,
                card_json
           FROM thread_entry WHERE thread_id = ? AND ordinal <= ? ORDER BY ordinal",
    )
    .bind(source.as_str())
    .bind(upto)
    .fetch_all(&mut *conn)
    .await?;
    let mut ordinal = 0;
    for row in rows {
        ordinal += 1;
        let (_, kind, author_kind, author_id, body, refs_json, operation_id, created_at, card) =
            row;
        sqlx::query(
            "INSERT INTO thread_entry
               (id, thread_id, ordinal, kind, author_kind, author_id, body, refs_json,
                operation_id, created_at, card_json)
             VALUES (?,?,?,?,?,?,?,?,?,?,?)",
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
        .bind(&card)
        .execute(&mut *conn)
        .await?;
        append_event(
            conn,
            crate::db::JournalEntry {
                event_id: uuid::Uuid::new_v4().to_string(),
                kind: "ThreadEntryAppended".into(),
                actor: Actor {
                    kind: author_kind,
                    id: author_id,
                },
                causation: None,
                correlation_id: None,
                project_id: None,
                thread_id: Some(fork.as_str().to_owned()),
                operation_id: None,
                payload_json: serde_json::json!({ "ordinal": ordinal, "kind": kind }).to_string(),
            },
            ts,
        )
        .await?;
    }
    Ok(ordinal)
}
