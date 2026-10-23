//! One job: a thread's entries — appending one inside a transaction, reading
//! them back in ordinal order. The thread row itself is `thread.rs`.

use sqlx::SqliteConnection;

use crate::db::{Actor, StorageError, append_event};
use crate::threads::model::{
    EntryRef, NewThreadEntry, ThreadEntry, ThreadEntryId, ThreadEntryKind, ThreadId,
};

/// Appends one entry inside the caller's transaction: allocates its ordinal,
/// inserts it, and journals `ThreadEntryAppended`. The turn command
/// writes its user entry with the operation it creates, so this cannot own a
/// transaction of its own.
impl crate::threads::Threads {
    pub(crate) async fn append_entry_in(
        conn: &mut SqliteConnection,
        thread_id: &ThreadId,
        entry: NewThreadEntry<'_>,
        ts: &str,
    ) -> Result<ThreadEntry, StorageError> {
        Self::append_entry_with_attachments_in(conn, thread_id, entry, &[], ts).await
    }

    pub(crate) async fn append_entry_with_attachments_in(
        conn: &mut SqliteConnection,
        thread_id: &ThreadId,
        entry: NewThreadEntry<'_>,
        ids: &[crate::threads::AttachmentId],
        ts: &str,
    ) -> Result<ThreadEntry, StorageError> {
        let attachments = Self::attachment_refs_in(conn, thread_id, ids).await?;
        let refs = entry.refs.to_vec();
        let refs_json = serde_json::to_string(&refs)?;
        let card_json = entry.card.map(serde_json::to_string).transpose()?;
        // Allocate inside this transaction so rollback consumes no ordinal.
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
            card_json, operation_id, created_at, attachments_json)
         VALUES (?,?,?,?,?,?,?,?,?,?,?,?)",
        )
        .bind(id.as_str())
        .bind(thread_id.as_str())
        .bind(ordinal)
        .bind(entry.kind.as_str())
        .bind(&entry.author.kind)
        .bind(&entry.author.id)
        .bind(entry.body)
        .bind(&refs_json)
        .bind(&card_json)
        .bind(entry.operation_id)
        .bind(ts)
        .bind(serde_json::to_string(&attachments)?)
        .execute(&mut *conn)
        .await?;

        let mut payload = serde_json::json!({ "ordinal": ordinal, "kind": entry.kind.as_str() });
        if !attachments.is_empty() {
            payload["attachments"] = serde_json::to_value(&attachments)?;
        }
        append_event(
            conn,
            crate::db::JournalEntry {
                event_id: uuid::Uuid::new_v4().to_string(),
                kind: "ThreadEntryAppended".into(),
                actor: entry.author.clone(),
                causation: None,
                correlation_id: None,
                project_id: None,
                thread_id: Some(thread_id.as_str().to_owned()),
                operation_id: None,
                payload_json: payload.to_string(),
            },
            ts,
        )
        .await?;

        Ok(ThreadEntry {
            attachments,
            id,
            thread_id: thread_id.clone(),
            ordinal,
            kind: entry.kind,
            author: entry.author,
            body: entry.body.to_string(),
            refs,
            card: entry.card.cloned(),
            created_at: ts.to_string(),
            operation_id: entry.operation_id.map(str::to_owned),
        })
    }
}

/// A row of `thread_entry` as `SELECT id, thread_id, ordinal, kind,
/// author_kind, author_id, body, refs_json, created_at, operation_id,
/// card_json` reads it; `ThreadsStore::into_entry` maps it.
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
    Option<String>,
    String,
);

impl super::ThreadsStore {
    pub(in crate::threads::store) fn into_entry(r: EntryRow) -> Result<ThreadEntry, StorageError> {
        Ok(ThreadEntry {
            attachments: serde_json::from_str(&r.11)?,
            id: ThreadEntryId::from_stored(r.0),
            thread_id: ThreadId::from_stored(r.1),
            ordinal: r.2,
            kind: ThreadEntryKind::parse(&r.3).ok_or_else(|| {
                StorageError::Constraint(format!("unknown thread entry kind: {}", r.3))
            })?,
            author: Actor { kind: r.4, id: r.5 },
            body: r.6,
            refs: serde_json::from_str::<Vec<EntryRef>>(&r.7)?,
            card: r.10.as_deref().map(serde_json::from_str).transpose()?,
            created_at: r.8,
            operation_id: r.9,
        })
    }
}

impl super::ThreadsStore {
    /// Internal write: no CommandRecord. Entries appended while a turn streams
    /// are produced by the daemon, not commanded by a client.
    ///
    /// The entry's fields arrive as one named struct rather than positional
    /// `&str`s. See `NewThreadEntry` for why.
    #[cfg(feature = "test-support")]
    pub(in crate::threads) async fn append_thread_entry(
        &self,
        thread_id: &ThreadId,
        entry: NewThreadEntry<'_>,
    ) -> Result<ThreadEntry, StorageError> {
        let (thread_id, kind, author, body, refs, card, operation_id, ts) = (
            thread_id.clone(),
            entry.kind,
            entry.author.clone(),
            entry.body.to_string(),
            entry.refs.to_vec(),
            entry.card.cloned(),
            entry.operation_id.map(str::to_owned),
            crate::db::now(),
        );
        self.storage
            .write_txn(move |conn| {
                Box::pin(async move {
                    let entry = NewThreadEntry {
                        kind,
                        author,
                        body: &body,
                        refs: &refs,
                        card: card.as_ref(),
                        operation_id: operation_id.as_deref(),
                    };
                    crate::threads::Threads::append_entry_in(conn, &thread_id, entry, &ts).await
                })
            })
            .await
    }

    pub(in crate::threads) async fn list_thread_entries(
        &self,
        thread_id: &ThreadId,
    ) -> Result<Vec<ThreadEntry>, StorageError> {
        let rows: Vec<EntryRow> = sqlx::query_as(
            "SELECT id, thread_id, ordinal, kind, author_kind, author_id, body, refs_json,
                    created_at, operation_id, card_json, attachments_json
               FROM thread_entry WHERE thread_id = ? ORDER BY ordinal",
        )
        .bind(thread_id.as_str())
        .fetch_all(self.storage.reader())
        .await?;
        rows.into_iter().map(Self::into_entry).collect()
    }
}
