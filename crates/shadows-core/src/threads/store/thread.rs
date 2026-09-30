//! One job: the planning thread row — creating it, the harness it runs on,
//! the session its turns continue. Its entries are `entry.rs`.

use std::path::PathBuf;

use sqlx::SqliteConnection;

use crate::command::CommandContext;
use crate::db::{Storage, StorageError, append_event, classify, now, record_command};
use crate::events::{Actor, DurableEvent};
use crate::projects::ProjectId;
use crate::threads::model::{CreatedTitle, PlanningThread, ThreadId, TurnContext};

impl Storage {
    /// `harness` is the thread's CLI (spec §12.6); the caller checks it is one
    /// Shadows knows.
    pub async fn create_planning_thread(
        &self,
        ctx: &CommandContext,
        project_id: &ProjectId,
        title: &str,
        harness: &str,
    ) -> Result<PlanningThread, StorageError> {
        let (ctx, project_id, title, harness, ts) = (
            ctx.clone(),
            project_id.clone(),
            title.to_string(),
            harness.to_string(),
            now(),
        );
        self.write_txn(move |conn| {
            Box::pin(async move {
                let scope_key = project_id.as_str().to_string();
                if let Some(id) = classify(conn, &ctx, "Project", &scope_key).await? {
                    return load_thread(conn, &ThreadId::from_stored(id)).await;
                }
                let actor = Actor::user(&ctx.principal_id);
                let id = insert_thread(
                    conn,
                    &project_id,
                    &title,
                    CreatedTitle::Client,
                    &harness,
                    actor,
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

    /// Changes the thread's harness while it has no operation (spec §12.6).
    /// An idempotent command: a replay answers the thread as it now stands and
    /// changes nothing, even once the harness is locked. A new command on a
    /// thread with an operation, or on a fork (locked from birth, §12.9), is
    /// `HarnessLocked` — checked here, and again by the
    /// `planning_thread_harness_locked` trigger below the application.
    pub async fn set_thread_harness(
        &self,
        ctx: &CommandContext,
        thread: &ThreadId,
        harness: &str,
    ) -> Result<PlanningThread, StorageError> {
        let (ctx, thread, harness, ts) = (ctx.clone(), thread.clone(), harness.to_string(), now());
        self.write_txn(move |conn| {
            Box::pin(async move {
                if classify(conn, &ctx, "Thread", thread.as_str())
                    .await?
                    .is_some()
                {
                    return load_thread(conn, &thread).await;
                }
                let current = load_thread(conn, &thread).await?;
                if current.forked_from_thread.is_some() || has_operation(conn, &thread).await? {
                    return Err(StorageError::HarnessLocked);
                }
                sqlx::query("UPDATE planning_thread SET harness_kind = ? WHERE id = ?")
                    .bind(&harness)
                    .bind(thread.as_str())
                    .execute(&mut *conn)
                    .await
                    .map_err(|e| match e.as_database_error() {
                        Some(db) if db.message().contains("harness_locked") => {
                            StorageError::HarnessLocked
                        }
                        _ => e.into(),
                    })?;
                append_event(
                    conn,
                    &DurableEvent::new(
                        "PlanningThreadHarnessChanged",
                        Actor::user(&ctx.principal_id),
                    )
                    .with_project(&current.project_id)
                    .with_thread(&thread)
                    .with_payload(serde_json::json!({ "harness": harness })),
                    &ts,
                )
                .await?;
                record_command(
                    conn,
                    &ctx,
                    "Thread",
                    thread.as_str(),
                    "PlanningThread",
                    thread.as_str(),
                    &ts,
                )
                .await?;
                load_thread(conn, &thread).await
            })
        })
        .await
    }

    /// `NotFound` when the thread does not exist, so a turn on an unknown
    /// thread is refused before an operation is created for it.
    pub async fn turn_context(&self, thread_id: &ThreadId) -> Result<TurnContext, StorageError> {
        type Row = (
            Option<String>,
            Option<String>,
            String,
            String,
            Option<String>,
        );
        let (directory, session, harness, project_id, fork_session): Row = sqlx::query_as(
            "SELECT p.directory, t.harness_session_id, t.harness_kind, t.project_id,
                    t.fork_session_id
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
            harness,
            project_id: ProjectId::from_stored(project_id),
            fork_session_id: fork_session,
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
        // Oldest first, by the `PlanningThreadCreated` event's sequence, for
        // the reason `list_projects` gives.
        let rows: Vec<ThreadRow> = sqlx::query_as(
            "SELECT t.id, t.project_id, t.title, t.status, t.created_at, t.harness_kind,
                    t.forked_from_thread
               FROM planning_thread t
               LEFT JOIN durable_event e
                 ON e.thread_id = t.id AND e.kind = 'PlanningThreadCreated'
              WHERE t.project_id = ?
              ORDER BY e.seq, t.id",
        )
        .bind(project_id.as_str())
        .fetch_all(self.reader())
        .await?;
        Ok(rows.into_iter().map(into_thread).collect())
    }
}

/// Creates an open thread in `project_id`, named by `named_by` (§4.2), and
/// journals `PlanningThreadCreated` by `actor`, inside the caller's
/// transaction. `NotFound`, not the foreign key's constraint failure: a
/// thread asked for under a project that does not exist names something
/// missing, not a conflict with what is stored.
pub(crate) async fn insert_thread(
    conn: &mut SqliteConnection,
    project_id: &ProjectId,
    title: &str,
    named_by: CreatedTitle,
    harness: &str,
    actor: Actor,
    ts: &str,
) -> Result<ThreadId, StorageError> {
    let project: Option<String> =
        sqlx::query_scalar("SELECT id FROM project WHERE id = ? AND removed_at IS NULL")
            .bind(project_id.as_str())
            .fetch_optional(&mut *conn)
            .await?;
    if project.is_none() {
        return Err(StorageError::NotFound("project"));
    }
    let id = ThreadId::generate();
    sqlx::query(
        "INSERT INTO planning_thread
           (id, project_id, title, title_source, status, next_entry_ordinal, harness_kind,
            created_at)
         VALUES (?,?,?,?, 'Open', 1, ?, ?)",
    )
    .bind(id.as_str())
    .bind(project_id.as_str())
    .bind(title)
    .bind(named_by.as_str())
    .bind(harness)
    .bind(ts)
    .execute(&mut *conn)
    .await?;
    append_event(
        conn,
        &DurableEvent::new("PlanningThreadCreated", actor)
            .with_project(project_id)
            .with_thread(&id)
            .with_payload(serde_json::json!({ "title": title, "harness": harness })),
        ts,
    )
    .await?;
    Ok(id)
}

/// Whether any operation — running or ended — exists on the thread. Its first
/// one fixes the harness (spec §12.6).
async fn has_operation(
    conn: &mut SqliteConnection,
    thread: &ThreadId,
) -> Result<bool, StorageError> {
    let found: Option<i64> =
        sqlx::query_scalar("SELECT 1 FROM operation WHERE thread_id = ? LIMIT 1")
            .bind(thread.as_str())
            .fetch_optional(&mut *conn)
            .await?;
    Ok(found.is_some())
}

type ThreadRow = (
    String,
    String,
    String,
    String,
    String,
    String,
    Option<String>,
);

fn into_thread(r: ThreadRow) -> PlanningThread {
    PlanningThread {
        id: ThreadId::from_stored(r.0),
        project_id: ProjectId::from_stored(r.1),
        title: r.2,
        status: r.3,
        created_at: r.4,
        harness: r.5,
        forked_from_thread: r.6.map(ThreadId::from_stored),
    }
}

pub(super) async fn load_thread(
    conn: &mut SqliteConnection,
    id: &ThreadId,
) -> Result<PlanningThread, StorageError> {
    let row: ThreadRow = sqlx::query_as(
        "SELECT id, project_id, title, status, created_at, harness_kind, forked_from_thread
           FROM planning_thread WHERE id = ?",
    )
    .bind(id.as_str())
    .fetch_optional(&mut *conn)
    .await?
    .ok_or(StorageError::NotFound("planning_thread"))?;
    Ok(into_thread(row))
}
