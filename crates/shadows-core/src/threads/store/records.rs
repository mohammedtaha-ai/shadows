//! Thread records accessed inside another owner's transaction or read snapshot.
use crate::projects::{ProjectId, Projects};
use crate::threads::{PlanningThread, ThreadEntryId, ThreadId, Threads, TurnContext};
use core_db::StorageError;
use sqlx::SqliteConnection;
use std::path::PathBuf;

impl Threads {
    /// Historical metadata, including a removed thread.
    pub(crate) async fn get_in(
        conn: &mut SqliteConnection,
        thread: &ThreadId,
    ) -> Result<PlanningThread, StorageError> {
        super::ThreadsStore::load_thread(conn, thread).await
    }

    /// The project of a retained thread, including a removed one.
    pub(crate) async fn project_in(
        conn: &mut SqliteConnection,
        thread: &ThreadId,
    ) -> Result<ProjectId, StorageError> {
        let project: String =
            sqlx::query_scalar("SELECT project_id FROM planning_thread WHERE id=?")
                .bind(thread.as_str())
                .fetch_optional(conn)
                .await?
                .ok_or(StorageError::NotFound("planning_thread"))?;
        Ok(ProjectId::from_stored(project))
    }

    pub(crate) async fn is_live_in(
        conn: &mut SqliteConnection,
        thread: &ThreadId,
    ) -> Result<bool, StorageError> {
        let live: Option<i64> =
            sqlx::query_scalar("SELECT 1 FROM planning_thread WHERE id=? AND removed_at IS NULL")
                .bind(thread.as_str())
                .fetch_optional(conn)
                .await?;
        Ok(live.is_some())
    }

    pub(crate) async fn require_live_in(
        conn: &mut SqliteConnection,
        thread: &ThreadId,
    ) -> Result<(), StorageError> {
        if Self::is_live_in(conn, thread).await? {
            Ok(())
        } else {
            Err(StorageError::NotFound("planning_thread"))
        }
    }

    /// Facts for an opening or a turn, read from the caller's snapshot.
    pub(crate) async fn turn_context_in(
        conn: &mut SqliteConnection,
        thread: &ThreadId,
    ) -> Result<TurnContext, StorageError> {
        type Row = (
            Option<String>,
            String,
            String,
            Option<String>,
            Option<String>,
        );
        let (session, harness, project, fork_session, profile): Row = sqlx::query_as(
            "SELECT harness_session_id,harness_kind,project_id,fork_session_id,profile
             FROM planning_thread WHERE id=? AND removed_at IS NULL",
        )
        .bind(thread.as_str())
        .fetch_optional(&mut *conn)
        .await?
        .ok_or(StorageError::NotFound("planning_thread"))?;
        let project_id = ProjectId::from_stored(project);
        let project = Projects::get_in(conn, &project_id).await?;
        Ok(TurnContext {
            profile,
            project_directory: project.directory.map(PathBuf::from),
            harness_session_id: session,
            harness,
            project_id,
            fork_session_id: fork_session,
        })
    }

    pub(crate) async fn list_for_profile_in(
        conn: &mut SqliteConnection,
        slug: &str,
    ) -> Result<Vec<PlanningThread>, StorageError> {
        let rows: Vec<super::thread::ThreadRow> = sqlx::query_as(
            "SELECT id,project_id,title,status,created_at,harness_kind,
                    forked_from_thread,removed_at,profile
             FROM planning_thread WHERE profile=? AND removed_at IS NULL
             ORDER BY created_at DESC,id DESC",
        )
        .bind(slug)
        .fetch_all(conn)
        .await?;
        Ok(rows
            .into_iter()
            .map(super::ThreadsStore::into_thread)
            .collect())
    }

    /// The immutable card stored with an entry; a missing or null card keeps SQL's error.
    pub(crate) async fn card_in(
        conn: &mut SqliteConnection,
        entry: &ThreadEntryId,
    ) -> Result<String, StorageError> {
        Ok(
            sqlx::query_scalar("SELECT card_json FROM thread_entry WHERE id=?")
                .bind(entry.as_str())
                .fetch_one(conn)
                .await?,
        )
    }

    /// User entries retain operation provenance, even when copied by a fork.
    pub(crate) async fn user_operations_in(
        conn: &mut SqliteConnection,
        thread: &ThreadId,
    ) -> Result<Vec<String>, StorageError> {
        Ok(sqlx::query_scalar(
            "SELECT operation_id FROM thread_entry
             WHERE thread_id=? AND kind='UserMessage' AND operation_id IS NOT NULL
             ORDER BY ordinal DESC",
        )
        .bind(thread.as_str())
        .fetch_all(conn)
        .await?)
    }

    /// First completed turn wins the durable session id.
    pub(crate) async fn record_harness_session_in(
        conn: &mut SqliteConnection,
        thread: &ThreadId,
        session: &str,
    ) -> Result<bool, StorageError> {
        Ok(sqlx::query(
            "UPDATE planning_thread SET harness_session_id=?
                       WHERE id=? AND harness_session_id IS NULL",
        )
        .bind(session)
        .bind(thread.as_str())
        .execute(conn)
        .await?
        .rows_affected()
            == 1)
    }
}

impl Threads {
    /// The harness of a retained thread in the requested project.
    pub(crate) async fn harness_in(
        conn: &mut SqliteConnection,
        thread: &ThreadId,
        project: &ProjectId,
    ) -> Result<String, StorageError> {
        Ok(sqlx::query_scalar(
            "SELECT harness_kind FROM planning_thread WHERE id=? AND project_id=?",
        )
        .bind(thread.as_str())
        .bind(project.as_str())
        .fetch_one(conn)
        .await?)
    }
    /// The latest user request recorded for the opaque turn reference.
    pub(crate) async fn request_entry_in(
        conn: &mut SqliteConnection,
        thread: &ThreadId,
        turn: &str,
    ) -> Result<Option<ThreadEntryId>, StorageError> {
        let id: Option<String> = sqlx::query_scalar(
            "SELECT id FROM thread_entry WHERE thread_id=? AND operation_id=? AND \
            kind='UserMessage' ORDER BY ordinal DESC LIMIT \
            1",
        )
        .bind(thread.as_str())
        .bind(turn)
        .fetch_optional(conn)
        .await?;
        Ok(id.map(ThreadEntryId::from_stored))
    }
    /// Immutable entry provenance in the caller's snapshot.
    pub(crate) async fn entry_in(
        conn: &mut SqliteConnection,
        id: &ThreadEntryId,
    ) -> Result<(ThreadId, i64), StorageError> {
        let (thread, ordinal): (String, i64) =
            sqlx::query_as("SELECT thread_id,ordinal FROM thread_entry WHERE id=?")
                .bind(id.as_str())
                .fetch_one(conn)
                .await?;
        Ok((ThreadId::from_stored(thread), ordinal))
    }
}

impl Threads {
    pub(crate) async fn fork_operation_in(
        conn: &mut SqliteConnection,
        source: &ThreadId,
        at: &ThreadEntryId,
    ) -> Result<Option<String>, StorageError> {
        sqlx::query_scalar("SELECT operation_id FROM thread_entry WHERE id=? AND thread_id=?")
            .bind(at.as_str())
            .bind(source.as_str())
            .fetch_optional(conn)
            .await?
            .ok_or(StorageError::NotFound("thread_entry"))
    }
}

impl Threads {
    /// A project's live threads prevent removal, checked in the removal transaction.
    pub(crate) async fn has_live_project_threads_in(
        conn: &mut SqliteConnection,
        project: &ProjectId,
    ) -> Result<bool, StorageError> {
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM planning_thread WHERE project_id = ? AND removed_at IS NULL",
        )
        .bind(project.as_str())
        .fetch_one(conn)
        .await?;
        Ok(count > 0)
    }
}
