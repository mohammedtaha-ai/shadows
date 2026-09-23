use std::path::PathBuf;

use sqlx::SqliteConnection;

use super::project::{classify, record_command};
use super::{Storage, StorageError, events::append_event, now};
use crate::command::CommandContext;
use crate::events::{Actor, DurableEvent};
use crate::project::ProjectId;
use crate::thread::{
    NewThreadEntry, PlanningThread, ThreadEntry, ThreadEntryId, ThreadId, TurnContext,
};

impl Storage {
    pub async fn create_planning_thread(
        &self,
        ctx: &CommandContext,
        project_id: &ProjectId,
        title: &str,
    ) -> Result<PlanningThread, StorageError> {
        let (ctx, project_id, title, ts) =
            (ctx.clone(), project_id.clone(), title.to_string(), now());
        self.write_txn(move |conn| {
            Box::pin(async move {
                let scope_key = project_id.as_str().to_string();
                if let Some(id) = classify(conn, &ctx, "Project", &scope_key).await? {
                    return load_thread(conn, &ThreadId::from_stored(id)).await;
                }
                // `NotFound`, not the foreign key's constraint failure: a
                // thread asked for under a project that does not exist names
                // something missing, not a conflict with what is stored.
                let project: Option<String> =
                    sqlx::query_scalar("SELECT id FROM project WHERE id = ?")
                        .bind(project_id.as_str())
                        .fetch_optional(&mut *conn)
                        .await?;
                if project.is_none() {
                    return Err(StorageError::NotFound("project"));
                }
                let id = ThreadId::generate();
                sqlx::query(
                    "INSERT INTO planning_thread
                       (id, project_id, title, status, next_entry_ordinal, created_at)
                     VALUES (?,?,?, 'Open', 1, ?)",
                )
                .bind(id.as_str())
                .bind(project_id.as_str())
                .bind(&title)
                .bind(&ts)
                .execute(&mut *conn)
                .await?;

                append_event(
                    conn,
                    &DurableEvent::new("PlanningThreadCreated", Actor::user(&ctx.principal_id))
                        .with_project(&project_id)
                        .with_thread(&id)
                        .with_payload(serde_json::json!({ "title": title })),
                    &ts,
                )
                .await?;

                record_command(
                    conn,
                    &ctx,
                    "Project",
                    &scope_key,
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

    /// Internal write: no CommandRecord. Entries appended while a turn streams
    /// are produced by the daemon, not commanded by a client. Spec section 5.2.
    ///
    /// The entry's fields arrive as one named struct rather than five positional
    /// `&str`s. See `NewThreadEntry` for why.
    pub async fn append_thread_entry(
        &self,
        thread_id: &ThreadId,
        entry: NewThreadEntry<'_>,
    ) -> Result<ThreadEntry, StorageError> {
        let (thread_id, kind, author, body, refs, ts) = (
            thread_id.clone(),
            entry.kind.to_string(),
            entry.author.clone(),
            entry.body.to_string(),
            entry.refs.to_vec(),
            now(),
        );
        let refs_json = serde_json::to_string(&refs)?;
        self.write_txn(move |conn| {
            Box::pin(async move {
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
                       (id, thread_id, ordinal, kind, author_kind, author_id, body, refs_json, created_at)
                     VALUES (?,?,?,?,?,?,?,?,?)",
                )
                .bind(id.as_str())
                .bind(thread_id.as_str())
                .bind(ordinal)
                .bind(&kind)
                .bind(&author.kind)
                .bind(&author.id)
                .bind(&body)
                .bind(&refs_json)
                .bind(&ts)
                .execute(&mut *conn)
                .await?;

                append_event(
                    conn,
                    &DurableEvent::new("ThreadEntryAppended", author.clone())
                        .with_thread(&thread_id)
                        .with_payload(serde_json::json!({ "ordinal": ordinal, "kind": kind })),
                    &ts,
                )
                .await?;

                Ok(ThreadEntry {
                    id,
                    thread_id,
                    ordinal,
                    kind,
                    author,
                    body,
                    refs,
                    created_at: ts,
                })
            })
        })
        .await
    }

    pub async fn list_thread_entries(
        &self,
        thread_id: &ThreadId,
    ) -> Result<Vec<ThreadEntry>, StorageError> {
        type Row = (
            String,
            String,
            i64,
            String,
            String,
            String,
            String,
            String,
            String,
        );
        let rows: Vec<Row> = sqlx::query_as(
            "SELECT id, thread_id, ordinal, kind, author_kind, author_id, body, refs_json, created_at
                   FROM thread_entry WHERE thread_id = ? ORDER BY ordinal",
        )
        .bind(thread_id.as_str())
        .fetch_all(self.reader())
        .await?;
        rows.into_iter()
            .map(|r| {
                Ok(ThreadEntry {
                    id: ThreadEntryId::from_stored(r.0),
                    thread_id: ThreadId::from_stored(r.1),
                    ordinal: r.2,
                    kind: r.3,
                    author: Actor { kind: r.4, id: r.5 },
                    body: r.6,
                    refs: serde_json::from_str(&r.7)?,
                    created_at: r.8,
                })
            })
            .collect()
    }

    /// `NotFound` when the thread does not exist, so a turn on an unknown
    /// thread is refused before an operation is created for it.
    pub async fn turn_context(&self, thread_id: &ThreadId) -> Result<TurnContext, StorageError> {
        let (directory, session): (Option<String>, Option<String>) = sqlx::query_as(
            "SELECT p.directory, t.harness_session_id
               FROM planning_thread t JOIN project p ON p.id = t.project_id
              WHERE t.id = ?",
        )
        .bind(thread_id.as_str())
        .fetch_optional(self.reader())
        .await?
        .ok_or(StorageError::NotFound("planning_thread"))?;
        Ok(TurnContext {
            project_directory: directory.map(PathBuf::from),
            harness_session_id: session,
        })
    }

    /// Records the harness session this thread's later turns resume. Written
    /// once: when two first turns race, the first to reach its turn-end wins
    /// and the other's session is left unrecorded rather than replacing a
    /// session a later turn may already have resumed. Returns whether it was
    /// recorded.
    ///
    /// Internal write, like `append_thread_entry`: no CommandRecord, and no
    /// journal event — a harness session id is the harness's continuity, not
    /// something a client renders or replays.
    pub async fn record_harness_session(
        &self,
        thread_id: &ThreadId,
        session_id: &str,
    ) -> Result<bool, StorageError> {
        let (thread_id, session_id) = (thread_id.clone(), session_id.to_string());
        self.write_txn(move |conn| {
            Box::pin(async move {
                let done = sqlx::query(
                    "UPDATE planning_thread SET harness_session_id = ?
                      WHERE id = ? AND harness_session_id IS NULL",
                )
                .bind(&session_id)
                .bind(thread_id.as_str())
                .execute(&mut *conn)
                .await?;
                Ok(done.rows_affected() == 1)
            })
        })
        .await
    }

    pub async fn list_threads_for_project(
        &self,
        project_id: &ProjectId,
    ) -> Result<Vec<PlanningThread>, StorageError> {
        let rows: Vec<(String, String, String, String, String)> = sqlx::query_as(
            "SELECT id, project_id, title, status, created_at
               FROM planning_thread WHERE project_id = ? ORDER BY created_at, id",
        )
        .bind(project_id.as_str())
        .fetch_all(self.reader())
        .await?;
        Ok(rows
            .into_iter()
            .map(|r| PlanningThread {
                id: ThreadId::from_stored(r.0),
                project_id: ProjectId::from_stored(r.1),
                title: r.2,
                status: r.3,
                created_at: r.4,
            })
            .collect())
    }
}

async fn load_thread(
    conn: &mut SqliteConnection,
    id: &ThreadId,
) -> Result<PlanningThread, StorageError> {
    let r: (String, String, String, String, String) = sqlx::query_as(
        "SELECT id, project_id, title, status, created_at FROM planning_thread WHERE id = ?",
    )
    .bind(id.as_str())
    .fetch_optional(&mut *conn)
    .await?
    .ok_or(StorageError::NotFound("planning_thread"))?;
    Ok(PlanningThread {
        id: ThreadId::from_stored(r.0),
        project_id: ProjectId::from_stored(r.1),
        title: r.2,
        status: r.3,
        created_at: r.4,
    })
}
