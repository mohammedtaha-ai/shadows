//! One job: the planning thread row — creating it, the harness it runs on,
//! the session its turns continue. Its entries are `entry.rs`.

use sqlx::SqliteConnection;

#[cfg(feature = "test-support")]
use crate::threads::TurnContext;
use crate::threads::model::{CreatedTitle, PlanningThread, ThreadId};
use core_db::{Actor, CommandContext, JournalEntry, Storage, StorageError};
use core_projects::ProjectId;

impl super::ThreadsStore {
    /// `harness` is the thread's CLI; the caller checks it is one
    /// Shadows knows.
    pub(in crate::threads) async fn create_planning_thread(
        &self,
        ctx: &CommandContext,
        project_id: &ProjectId,
        title: &str,
        harness: &str,
        profile: Option<&str>,
    ) -> Result<PlanningThread, crate::CoreError> {
        let (ctx, project_id, title, harness, ts) = (
            ctx.clone(),
            project_id.clone(),
            title.to_string(),
            harness.to_string(),
            Storage::now(),
        );
        let profile = profile.map(str::to_owned);
        self.storage
            .write_txn_as(move |conn| {
                Box::pin(async move {
                    let scope_key = project_id.as_str().to_string();
                    if let Some(id) =
                        CommandContext::classify(conn, &ctx, "Project", &scope_key).await?
                    {
                        return Ok(super::ThreadsStore::load_thread(
                            conn,
                            &ThreadId::from_stored(id),
                        )
                        .await?);
                    }
                    if let Some(profile) = &profile {
                        crate::agents::Agents::require_live_in(conn, profile).await?;
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
                    sqlx::query("UPDATE planning_thread SET profile=? WHERE id=?")
                        .bind(&profile)
                        .bind(id.as_str())
                        .execute(&mut *conn)
                        .await
                        .map_err(StorageError::from)?;
                    CommandContext::record_command(
                        conn,
                        &ctx,
                        "Project",
                        &scope_key,
                        "PlanningThread",
                        id.as_str(),
                        &ts,
                    )
                    .await?;
                    Ok(super::ThreadsStore::load_thread(conn, &id).await?)
                })
            })
            .await
    }

    /// `NotFound` when the thread does not exist, so a turn on an unknown
    /// thread is refused before an operation is created for it.
    #[cfg(feature = "test-support")]
    pub(in crate::threads) async fn turn_context(
        &self,
        thread_id: &ThreadId,
    ) -> Result<TurnContext, StorageError> {
        let mut conn = self.storage.reader().begin().await?;
        crate::threads::Threads::turn_context_in(&mut conn, thread_id).await
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
    pub(in crate::threads) async fn record_harness_session(
        &self,
        thread_id: &ThreadId,
        session_id: &str,
    ) -> Result<bool, StorageError> {
        let (thread_id, session_id) = (thread_id.clone(), session_id.to_string());
        self.storage
            .write_txn(move |conn| {
                Box::pin(async move {
                    crate::threads::Threads::record_harness_session_in(
                        conn,
                        &thread_id,
                        &session_id,
                    )
                    .await
                })
            })
            .await
    }

    pub(in crate::threads) async fn list_threads_for_project(
        &self,
        project_id: &ProjectId,
    ) -> Result<Vec<PlanningThread>, StorageError> {
        // Oldest first, by the `PlanningThreadCreated` event's sequence, for
        // the reason `list_projects` gives.
        let rows: Vec<ThreadRow> = sqlx::query_as(
            "SELECT t.id, t.project_id, t.title, t.status, t.created_at, t.harness_kind,
                    t.forked_from_thread, t.removed_at, t.profile
               FROM planning_thread t
               LEFT JOIN durable_event e
                 ON e.thread_id = t.id AND e.kind = 'PlanningThreadCreated'
              WHERE t.project_id = ? AND t.removed_at IS NULL
              ORDER BY e.seq, t.id",
        )
        .bind(project_id.as_str())
        .fetch_all(self.storage.reader())
        .await?;
        Ok(rows.into_iter().map(Self::into_thread).collect())
    }
}

/// Creates an open thread in `project_id`, named by `named_by`, and
/// journals `PlanningThreadCreated` by `actor`, inside the caller's
/// transaction. `NotFound`, not the foreign key's constraint failure: a
/// thread asked for under a project that does not exist names something
/// missing, not a conflict with what is stored.
async fn insert_thread(
    conn: &mut SqliteConnection,
    project_id: &ProjectId,
    title: &str,
    named_by: CreatedTitle,
    harness: &str,
    actor: Actor,
    ts: &str,
) -> Result<ThreadId, StorageError> {
    core_projects::Projects::require_live_in(conn, project_id).await?;
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
    JournalEntry::append_event(
        conn,
        core_db::JournalEntry {
            event_id: uuid::Uuid::new_v4().to_string(),
            kind: "PlanningThreadCreated".into(),
            actor,
            causation: None,
            correlation_id: None,
            project_id: Some(project_id.as_str().to_owned()),
            thread_id: Some(id.as_str().to_owned()),
            operation_id: None,
            payload_json: serde_json::json!({ "title": title, "harness": harness }).to_string(),
        },
        ts,
    )
    .await?;
    Ok(id)
}

pub(super) type ThreadRow = (
    String,
    String,
    String,
    String,
    String,
    String,
    Option<String>,
    Option<String>,
    Option<String>,
);

impl super::ThreadsStore {
    pub(super) fn into_thread(r: ThreadRow) -> PlanningThread {
        PlanningThread {
            id: ThreadId::from_stored(r.0),
            project_id: ProjectId::from_recorded(r.1),
            title: r.2,
            status: r.3,
            created_at: r.4,
            harness: r.5,
            forked_from_thread: r.6.map(ThreadId::from_stored),
            removed_at: r.7,
            profile: r.8,
        }
    }
}

impl super::ThreadsStore {
    pub(in crate::threads::store) async fn load_thread(
        conn: &mut SqliteConnection,
        id: &ThreadId,
    ) -> Result<PlanningThread, StorageError> {
        let row: ThreadRow = sqlx::query_as(
            "SELECT id, project_id, title, status, created_at, harness_kind, \
         forked_from_thread, removed_at, profile
           FROM planning_thread WHERE id = ?",
        )
        .bind(id.as_str())
        .fetch_optional(&mut *conn)
        .await?
        .ok_or(StorageError::NotFound("planning_thread"))?;
        Ok(Self::into_thread(row))
    }
}

impl super::ThreadsStore {
    pub(in crate::threads) async fn list_threads_for_profile(
        &self,
        slug: &str,
    ) -> Result<Vec<PlanningThread>, StorageError> {
        let mut conn = self.storage.reader().acquire().await?;
        crate::threads::Threads::list_for_profile_in(&mut conn, slug).await
    }
}

impl crate::threads::Threads {
    pub(crate) async fn profile_in(
        conn: &mut SqliteConnection,
        thread: &ThreadId,
    ) -> Result<Option<String>, StorageError> {
        Ok(
            sqlx::query_scalar("SELECT profile FROM planning_thread WHERE id=?")
                .bind(thread.as_str())
                .fetch_one(conn)
                .await?,
        )
    }
}

impl crate::threads::Threads {
    /// Updates and journals the thread row inside Harness's guarded command write.
    pub(crate) async fn set_harness_in(
        conn: &mut SqliteConnection,
        thread: &ThreadId,
        project: &ProjectId,
        harness: &str,
        actor: Actor,
        ts: &str,
    ) -> Result<PlanningThread, crate::threads::ThreadsError> {
        sqlx::query("UPDATE planning_thread SET harness_kind = ? WHERE id = ?")
            .bind(harness)
            .bind(thread.as_str())
            .execute(&mut *conn)
            .await
            .map_err(|e| match e.as_database_error() {
                Some(db) if db.message().contains("harness_locked") => {
                    crate::threads::ThreadsError::HarnessLocked
                }
                _ => e.into(),
            })?;
        JournalEntry::append_event(
            conn,
            core_db::JournalEntry {
                event_id: uuid::Uuid::new_v4().to_string(),
                kind: "PlanningThreadHarnessChanged".into(),
                actor,
                causation: None,
                correlation_id: None,
                project_id: Some(project.as_str().to_owned()),
                thread_id: Some(thread.as_str().to_owned()),
                operation_id: None,
                payload_json: serde_json::json!({ "harness": harness }).to_string(),
            },
            ts,
        )
        .await?;
        Ok(super::ThreadsStore::load_thread(conn, thread).await?)
    }
}
