use sqlx::SqliteConnection;

use super::project::{classify, record_command};
use super::{Storage, StorageError, events::append_event, now};
use crate::command::CommandContext;
use crate::events::{Actor, DurableEvent};
use crate::thread::{EntryRef, PlanningThread, ThreadEntry};

impl Storage {
    pub async fn create_planning_thread(
        &self,
        ctx: &CommandContext,
        project_id: &str,
        title: &str,
    ) -> Result<PlanningThread, StorageError> {
        let (ctx, project_id, title, ts) = (
            ctx.clone(),
            project_id.to_string(),
            title.to_string(),
            now(),
        );
        self.write_txn(move |conn| {
            Box::pin(async move {
                if let Some(id) = classify(conn, &ctx, "Project", &project_id).await? {
                    return load_thread(conn, &id).await;
                }
                let id = uuid::Uuid::new_v4().to_string();
                sqlx::query(
                    "INSERT INTO planning_thread
                       (id, project_id, title, status, next_entry_ordinal, created_at)
                     VALUES (?,?,?, 'Open', 1, ?)",
                )
                .bind(&id)
                .bind(&project_id)
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
                    &project_id,
                    "PlanningThread",
                    &id,
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
    pub async fn append_thread_entry(
        &self,
        thread_id: &str,
        kind: &str,
        author_kind: &str,
        author_id: &str,
        body: &str,
        refs: &[EntryRef],
    ) -> Result<ThreadEntry, StorageError> {
        let (thread_id, kind, author_kind, author_id, body, refs, ts) = (
            thread_id.to_string(),
            kind.to_string(),
            author_kind.to_string(),
            author_id.to_string(),
            body.to_string(),
            refs.to_vec(),
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
                .bind(&thread_id)
                .fetch_optional(&mut *conn)
                .await?
                .ok_or(StorageError::NotFound("planning_thread"))?;

                let id = uuid::Uuid::new_v4().to_string();
                sqlx::query(
                    "INSERT INTO thread_entry
                       (id, thread_id, ordinal, kind, author_kind, author_id, body, refs_json, created_at)
                     VALUES (?,?,?,?,?,?,?,?,?)",
                )
                .bind(&id)
                .bind(&thread_id)
                .bind(ordinal)
                .bind(&kind)
                .bind(&author_kind)
                .bind(&author_id)
                .bind(&body)
                .bind(&refs_json)
                .bind(&ts)
                .execute(&mut *conn)
                .await?;

                append_event(
                    conn,
                    &DurableEvent::new(
                        "ThreadEntryAppended",
                        Actor {
                            kind: author_kind.clone(),
                            id: author_id.clone(),
                        },
                    )
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
                    author_kind,
                    author_id,
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
        thread_id: &str,
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
        .bind(thread_id)
        .fetch_all(self.reader())
        .await?;
        rows.into_iter()
            .map(|r| {
                Ok(ThreadEntry {
                    id: r.0,
                    thread_id: r.1,
                    ordinal: r.2,
                    kind: r.3,
                    author_kind: r.4,
                    author_id: r.5,
                    body: r.6,
                    refs: serde_json::from_str(&r.7)?,
                    created_at: r.8,
                })
            })
            .collect()
    }

    pub async fn list_threads_for_project(
        &self,
        project_id: &str,
    ) -> Result<Vec<PlanningThread>, StorageError> {
        let rows: Vec<(String, String, String, String, String)> = sqlx::query_as(
            "SELECT id, project_id, title, status, created_at
               FROM planning_thread WHERE project_id = ? ORDER BY created_at, id",
        )
        .bind(project_id)
        .fetch_all(self.reader())
        .await?;
        Ok(rows
            .into_iter()
            .map(|r| PlanningThread {
                id: r.0,
                project_id: r.1,
                title: r.2,
                status: r.3,
                created_at: r.4,
            })
            .collect())
    }
}

async fn load_thread(
    conn: &mut SqliteConnection,
    id: &str,
) -> Result<PlanningThread, StorageError> {
    let r: (String, String, String, String, String) = sqlx::query_as(
        "SELECT id, project_id, title, status, created_at FROM planning_thread WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(&mut *conn)
    .await?
    .ok_or(StorageError::NotFound("planning_thread"))?;
    Ok(PlanningThread {
        id: r.0,
        project_id: r.1,
        title: r.2,
        status: r.3,
        created_at: r.4,
    })
}
