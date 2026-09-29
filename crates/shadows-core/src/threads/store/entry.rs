//! One job: a thread's entries — appending one inside a transaction, reading
//! them back in ordinal order. The thread row itself is `thread.rs`.

use sqlx::SqliteConnection;

use crate::db::{Storage, StorageError, append_event, now};
use crate::events::{Actor, DurableEvent};
use crate::threads::model::{
    EntryRef, NewThreadEntry, ThreadEntry, ThreadEntryId, ThreadEntryKind, ThreadId,
};
use crate::turns::OperationId;

/// Appends one entry inside the caller's transaction: allocates its ordinal,
/// inserts it, and journals `ThreadEntryAppended`. The turn command (§12.7)
/// writes its user entry with the operation it creates, so this cannot own a
/// transaction of its own.
pub(crate) async fn append_entry_in(
    conn: &mut SqliteConnection,
    thread_id: &ThreadId,
    entry: NewThreadEntry<'_>,
    ts: &str,
) -> Result<ThreadEntry, StorageError> {
    let refs = entry.refs.to_vec();
    let refs_json = serde_json::to_string(&refs)?;
    // Spec section 6.5: allocate inside this transaction. Never MAX+1.
    let ordinal: i64 = sqlx::query_scalar(
        "UPDATE planning_thread
            SET next_entry_ordinal = next_entry_ordinal + 1
          WHERE id = ?
      RETURNING next_entry_ordinal - 1",
    )
    .bind(thread_id.as_str())
    .fetch_optional(&mut *conn)
    .await?
    .ok_or(StorageError::NotFound("planning_thread"))?;

    let id = ThreadEntryId::generate();
    sqlx::query(
        "INSERT INTO thread_entry
           (id, thread_id, ordinal, kind, author_kind, author_id, body, refs_json,
            operation_id, created_at)
         VALUES (?,?,?,?,?,?,?,?,?,?)",
    )
    .bind(id.as_str())
    .bind(thread_id.as_str())
    .bind(ordinal)
    .bind(entry.kind.as_str())
    .bind(&entry.author.kind)
    .bind(&entry.author.id)
    .bind(entry.body)
    .bind(&refs_json)
    .bind(entry.operation_id.map(OperationId::as_str))
    .bind(ts)
    .execute(&mut *conn)
    .await?;

    append_event(
        conn,
        &DurableEvent::new("ThreadEntryAppended", entry.author.clone())
            .with_thread(thread_id)
            .with_payload(serde_json::json!({ "ordinal": ordinal, "kind": entry.kind.as_str() })),
        ts,
    )
    .await?;

    Ok(ThreadEntry {
        id,
        thread_id: thread_id.clone(),
        ordinal,
        kind: entry.kind,
        author: entry.author,
        body: entry.body.to_string(),
        refs,
        created_at: ts.to_string(),
        operation_id: entry.operation_id.cloned(),
    })
}

/// A row of `thread_entry` as `SELECT id, thread_id, ordinal, kind,
/// author_kind, author_id, body, refs_json, created_at, operation_id` reads
/// it; [`into_entry`] maps it.
pub(super) type EntryRow = (
    String,
    String,
    i64,
    String,
    String,
    String,
    String,
    String,
    String,
    Option<String>,
);

pub(super) fn into_entry(r: EntryRow) -> Result<ThreadEntry, StorageError> {
    Ok(ThreadEntry {
        id: ThreadEntryId::from_stored(r.0),
        thread_id: ThreadId::from_stored(r.1),
        ordinal: r.2,
        kind: ThreadEntryKind::parse(&r.3).ok_or_else(|| {
            StorageError::Constraint(format!("unknown thread entry kind: {}", r.3))
        })?,
        author: Actor { kind: r.4, id: r.5 },
        body: r.6,
        refs: serde_json::from_str::<Vec<EntryRef>>(&r.7)?,
        created_at: r.8,
        operation_id: r.9.map(OperationId::from_stored),
    })
}

impl Storage {
    /// Internal write: no CommandRecord. Entries appended while a turn streams
    /// are produced by the daemon, not commanded by a client. Spec section 5.2.
    ///
    /// The entry's fields arrive as one named struct rather than positional
    /// `&str`s. See `NewThreadEntry` for why.
    pub async fn append_thread_entry(
        &self,
        thread_id: &ThreadId,
        entry: NewThreadEntry<'_>,
    ) -> Result<ThreadEntry, StorageError> {
        let (thread_id, kind, author, body, refs, operation_id, ts) = (
            thread_id.clone(),
            entry.kind,
            entry.author.clone(),
            entry.body.to_string(),
            entry.refs.to_vec(),
            entry.operation_id.cloned(),
            now(),
        );
        self.write_txn(move |conn| {
            Box::pin(async move {
                let entry = NewThreadEntry {
                    kind,
                    author,
                    body: &body,
                    refs: &refs,
                    operation_id: operation_id.as_ref(),
                };
                append_entry_in(conn, &thread_id, entry, &ts).await
            })
        })
        .await
    }

    pub async fn list_thread_entries(
        &self,
        thread_id: &ThreadId,
    ) -> Result<Vec<ThreadEntry>, StorageError> {
        let rows: Vec<EntryRow> = sqlx::query_as(
            "SELECT id, thread_id, ordinal, kind, author_kind, author_id, body, refs_json,
                    created_at, operation_id
               FROM thread_entry WHERE thread_id = ? ORDER BY ordinal",
        )
        .bind(thread_id.as_str())
        .fetch_all(self.reader())
        .await?;
        rows.into_iter().map(into_entry).collect()
    }
}
