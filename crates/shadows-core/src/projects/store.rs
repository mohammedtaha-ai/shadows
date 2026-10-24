//! One job: a project's rows — the project itself and the modes it allows.

use std::collections::BTreeMap;
use std::sync::Arc;

use sqlx::SqliteConnection;

use super::Projects;
use super::directory::ProjectDirectory;
use super::model::{Project, ProjectId};
use crate::events::DurableEvent;
use core_db::{Actor, CommandContext, JournalEntry, Storage, StorageError};
use shadows_agent::policy;

/// The service's SQL, over the shared storage handle.
#[derive(Clone)]
pub(super) struct ProjectsStore {
    storage: Arc<Storage>,
}

impl ProjectsStore {
    pub(super) fn new(storage: Arc<Storage>) -> Self {
        Self { storage }
    }

    pub(super) async fn create(
        &self,
        ctx: &CommandContext,
        slug: &str,
        name: &str,
        directory: &ProjectDirectory,
        default_modes: &BTreeMap<String, Vec<String>>,
    ) -> Result<Project, StorageError> {
        let (ctx, slug, name, directory, modes, ts) = (
            ctx.clone(),
            slug.to_string(),
            name.to_string(),
            directory.as_str().to_string(),
            default_modes.clone(),
            Storage::now(),
        );
        self.storage
            .write_txn(move |conn| {
                Box::pin(async move {
                    if let Some(existing_id) =
                        CommandContext::classify(conn, &ctx, "Global", "").await?
                    {
                        return load_project(conn, &ProjectId::from_stored(existing_id)).await;
                    }
                    let id = ProjectId::generate();
                    sqlx::query(
                        "INSERT INTO project (id, slug, name, directory, created_at) \
                     VALUES (?,?,?,?,?)",
                    )
                    .bind(id.as_str())
                    .bind(&slug)
                    .bind(&name)
                    .bind(&directory)
                    .bind(&ts)
                    .execute(&mut *conn)
                    .await?;
                    replace_modes(conn, &id, &modes).await?;

                    JournalEntry::append_event(
                        conn,
                        DurableEvent::new("ProjectCreated", Actor::user(&ctx.principal_id))
                            .with_project(&id)
                            .with_payload(serde_json::json!({
                                "slug": slug, "name": name, "directory": directory,
                            }))
                            .journal_entry(),
                        &ts,
                    )
                    .await?;

                    CommandContext::record_command(
                        conn,
                        &ctx,
                        "Global",
                        "",
                        "Project",
                        id.as_str(),
                        &ts,
                    )
                    .await?;
                    load_project(conn, &id).await
                })
            })
            .await
    }

    /// Oldest first, by the durable sequence of each project's `ProjectCreated`
    /// event, written in the transaction that created the row — not by
    /// `created_at`, whose RFC 3339 text with trimmed zeros does not sort in
    /// time order within a second (CLAUDE.md: ordering is explicit). `LEFT`
    /// so a project could never be hidden by a missing event; there is none.
    pub(super) async fn list(&self) -> Result<Vec<Project>, StorageError> {
        let rows: Vec<ProjectRow> = sqlx::query_as(
            "SELECT p.id, p.slug, p.name, p.directory, p.created_at, p.assistant
               FROM project p
               LEFT JOIN durable_event e
                 ON e.project_id = p.id AND e.kind = 'ProjectCreated'
              WHERE p.removed_at IS NULL
              ORDER BY e.seq, p.id",
        )
        .fetch_all(self.storage.reader())
        .await?;
        with_modes(self.storage.reader(), rows).await
    }

    /// Live projects, most recently used first: by the durable sequence of the
    /// newest `OperationCreated` among the project's threads, or of its
    /// `ProjectCreated` for one with no operation. Read from the journal only:
    /// a thread belongs to the project named by its `PlanningThreadCreated`
    /// event, so no table another service owns is joined. The sequence, not
    /// `created_at`, whose RFC 3339 text with trimmed zeros does not sort in
    /// time order within a second (CLAUDE.md: ordering is explicit).
    pub(super) async fn by_recent_activity(&self) -> Result<Vec<Project>, StorageError> {
        let rows: Vec<ProjectRow> = sqlx::query_as(
            "SELECT p.id, p.slug, p.name, p.directory, p.created_at, p.assistant
               FROM project p
              WHERE p.removed_at IS NULL
              ORDER BY COALESCE(
                (SELECT MAX(o.seq) FROM durable_event o
                  WHERE o.kind = 'OperationCreated'
                    AND o.thread_id IN (
                      SELECT c.thread_id FROM durable_event c
                       WHERE c.project_id = p.id AND c.kind = 'PlanningThreadCreated')),
                (SELECT MAX(e.seq) FROM durable_event e
                  WHERE e.project_id = p.id AND e.kind = 'ProjectCreated'),
                0) DESC, p.id",
        )
        .fetch_all(self.storage.reader())
        .await?;
        with_modes(self.storage.reader(), rows).await
    }

    /// Marks the live project `slug` as AYLUL's home. Idempotent; `None`
    /// when no live project has that slug.
    pub(super) async fn mark_assistant(&self, slug: &str) -> Result<Option<Project>, StorageError> {
        let slug = slug.to_string();
        self.storage
            .write_txn(move |conn| {
                Box::pin(async move {
                    let id: Option<String> = sqlx::query_scalar(
                        "SELECT id FROM project WHERE slug = ? AND removed_at IS NULL",
                    )
                    .bind(&slug)
                    .fetch_optional(&mut *conn)
                    .await?;
                    let Some(id) = id else { return Ok(None) };
                    let id = ProjectId::from_stored(id);
                    sqlx::query("UPDATE project SET assistant = 1 WHERE id = ?")
                        .bind(id.as_str())
                        .execute(&mut *conn)
                        .await?;
                    Ok(Some(load_project(conn, &id).await?))
                })
            })
            .await
    }

    /// Brings back the removed project `slug` when it was AYLUL's (a slug
    /// stays taken after a removal). `None` when there is no such project.
    pub(super) async fn restore_assistant(
        &self,
        slug: &str,
    ) -> Result<Option<Project>, StorageError> {
        let slug = slug.to_string();
        self.storage
            .write_txn(move |conn| {
                Box::pin(async move {
                    let id: Option<String> = sqlx::query_scalar(
                        "SELECT id FROM project \
                     WHERE slug = ? AND assistant = 1 AND removed_at IS NOT NULL",
                    )
                    .bind(&slug)
                    .fetch_optional(&mut *conn)
                    .await?;
                    let Some(id) = id else { return Ok(None) };
                    let id = ProjectId::from_stored(id);
                    sqlx::query("UPDATE project SET removed_at = NULL WHERE id = ?")
                        .bind(id.as_str())
                        .execute(&mut *conn)
                        .await?;
                    Ok(Some(load_project(conn, &id).await?))
                })
            })
            .await
    }

    /// The project, NotFound once removed.
    pub(super) async fn get(&self, id: &ProjectId) -> Result<Project, StorageError> {
        let mut conn = self.storage.reader().acquire().await?;
        Projects::require_live_in(&mut conn, id).await?;
        load_project(&mut conn, id).await
    }

    pub(super) async fn by_slug(&self, slug: &str) -> Result<Option<Project>, StorageError> {
        let mut conn = self.storage.reader().acquire().await?;
        Projects::by_slug_in(&mut conn, slug).await
    }

    /// Replaces the modes the project allows for each harness named in
    /// `modes`; harnesses not named keep theirs. An idempotent
    /// command: a replay answers the project as it now stands. The caller
    /// checks every mode is in its harness's policy.
    pub(super) async fn set_modes(
        &self,
        ctx: &CommandContext,
        project_id: &ProjectId,
        modes: &BTreeMap<String, Vec<String>>,
    ) -> Result<Project, StorageError> {
        let (ctx, id, modes, ts) = (
            ctx.clone(),
            project_id.clone(),
            modes.clone(),
            Storage::now(),
        );
        self.storage
            .write_txn(move |conn| {
                Box::pin(async move {
                    if CommandContext::classify(conn, &ctx, "Project", id.as_str())
                        .await?
                        .is_some()
                    {
                        return load_project(conn, &id).await;
                    }
                    Projects::require_live_in(conn, &id).await?;
                    replace_modes(conn, &id, &modes).await?;
                    JournalEntry::append_event(
                        conn,
                        DurableEvent::new("ProjectModesChanged", Actor::user(&ctx.principal_id))
                            .with_project(&id)
                            .with_payload(serde_json::json!({ "allowed_modes": modes }))
                            .journal_entry(),
                        &ts,
                    )
                    .await?;
                    CommandContext::record_command(
                        conn,
                        &ctx,
                        "Project",
                        id.as_str(),
                        "Project",
                        id.as_str(),
                        &ts,
                    )
                    .await?;
                    load_project(conn, &id).await
                })
            })
            .await
    }
}

/// Deletes and reinserts the project's rows for each harness in `modes`.
async fn replace_modes(
    conn: &mut SqliteConnection,
    id: &ProjectId,
    modes: &BTreeMap<String, Vec<String>>,
) -> Result<(), StorageError> {
    for (harness, list) in modes {
        sqlx::query("DELETE FROM project_mode WHERE project_id = ? AND harness_kind = ?")
            .bind(id.as_str())
            .bind(harness)
            .execute(&mut *conn)
            .await?;
        for mode in list {
            sqlx::query(
                "INSERT OR IGNORE INTO project_mode (project_id, harness_kind, mode_id)
                 VALUES (?,?,?)",
            )
            .bind(id.as_str())
            .bind(harness)
            .bind(mode)
            .execute(&mut *conn)
            .await?;
        }
    }
    Ok(())
}

/// The rows as projects, each with the modes it allows.
async fn with_modes(
    pool: &sqlx::SqlitePool,
    rows: Vec<ProjectRow>,
) -> Result<Vec<Project>, StorageError> {
    let mut modes = all_modes(pool).await?;
    Ok(rows
        .into_iter()
        .map(|row| {
            let allowed = modes.remove(&row.0).unwrap_or_default();
            project(row, allowed)
        })
        .collect())
}

/// Every known harness with no mode: the base each project's rows fill in.
fn empty_modes() -> BTreeMap<String, Vec<String>> {
    policy::KNOWN
        .iter()
        .map(|kind| (kind.to_string(), Vec::new()))
        .collect()
}

fn group(rows: Vec<(String, String, String)>) -> BTreeMap<String, BTreeMap<String, Vec<String>>> {
    let mut by_project: BTreeMap<String, BTreeMap<String, Vec<String>>> = BTreeMap::new();
    for (project, harness, mode) in rows {
        by_project
            .entry(project)
            .or_insert_with(empty_modes)
            .entry(harness)
            .or_default()
            .push(mode);
    }
    by_project
}

async fn all_modes(
    pool: &sqlx::SqlitePool,
) -> Result<BTreeMap<String, BTreeMap<String, Vec<String>>>, StorageError> {
    // The set is read in an explicit key order (never `rowid`); that order
    // means nothing.
    let rows = sqlx::query_as(
        "SELECT project_id, harness_kind, mode_id FROM project_mode
          ORDER BY project_id, harness_kind, mode_id",
    )
    .fetch_all(pool)
    .await?;
    Ok(group(rows))
}

async fn project_modes(
    conn: &mut SqliteConnection,
    id: &ProjectId,
) -> Result<BTreeMap<String, Vec<String>>, StorageError> {
    let rows = sqlx::query_as(
        "SELECT project_id, harness_kind, mode_id FROM project_mode
          WHERE project_id = ? ORDER BY project_id, harness_kind, mode_id",
    )
    .bind(id.as_str())
    .fetch_all(&mut *conn)
    .await?;
    Ok(group(rows).remove(id.as_str()).unwrap_or_else(empty_modes))
}

type ProjectRow = (String, String, String, Option<String>, String, bool);

fn project(
    (id, slug, name, directory, created_at, assistant): ProjectRow,
    allowed_modes: BTreeMap<String, Vec<String>>,
) -> Project {
    let allowed_modes = if allowed_modes.is_empty() {
        empty_modes()
    } else {
        allowed_modes
    };
    Project {
        id: ProjectId::from_stored(id),
        slug,
        name,
        directory,
        created_at,
        allowed_modes,
        assistant,
    }
}

/// The project's row, removed or not: what a replay answers.
async fn load_project(
    conn: &mut SqliteConnection,
    id: &ProjectId,
) -> Result<Project, StorageError> {
    let row: ProjectRow = sqlx::query_as(
        "SELECT id, slug, name, directory, created_at, assistant FROM project WHERE id = ?",
    )
    .bind(id.as_str())
    .fetch_optional(&mut *conn)
    .await?
    .ok_or(StorageError::NotFound("project"))?;
    let modes = project_modes(conn, id).await?;
    Ok(project(row, modes))
}

impl Projects {
    /// A live project's metadata in the caller's snapshot.
    pub(crate) async fn get_in(
        conn: &mut SqliteConnection,
        id: &ProjectId,
    ) -> Result<Project, StorageError> {
        Self::require_live_in(conn, id).await?;
        load_project(conn, id).await
    }

    /// The live project holding a slug in the caller's snapshot.
    pub(crate) async fn by_slug_in(
        conn: &mut SqliteConnection,
        slug: &str,
    ) -> Result<Option<Project>, StorageError> {
        let id: Option<String> = sqlx::query_scalar("SELECT id FROM project WHERE slug = ?")
            .bind(slug)
            .fetch_optional(&mut *conn)
            .await?;
        match id {
            Some(id) => match Self::get_in(conn, &ProjectId::from_stored(id)).await {
                Ok(project) => Ok(Some(project)),
                Err(StorageError::NotFound(_)) => Ok(None),
                Err(error) => Err(error),
            },
            None => Ok(None),
        }
    }

    /// A retained project's slug, including removed rows needed by command replays.
    pub(crate) async fn slug_in(
        conn: &mut SqliteConnection,
        id: &ProjectId,
    ) -> Result<String, StorageError> {
        sqlx::query_scalar("SELECT slug FROM project WHERE id = ?")
            .bind(id.as_str())
            .fetch_optional(&mut *conn)
            .await?
            .ok_or(StorageError::NotFound("project"))
    }

    /// Proves `project` is live inside the caller's transaction or connection:
    /// `NotFound("project")` for an unknown or removed one. The one liveness
    /// check: this store's own reads and writes use it, and other services
    /// call it instead of reading the `project` table.
    pub(crate) async fn require_live_in(
        conn: &mut SqliteConnection,
        project: &ProjectId,
    ) -> Result<(), StorageError> {
        let known: Option<i64> =
            sqlx::query_scalar("SELECT 1 FROM project WHERE id = ? AND removed_at IS NULL")
                .bind(project.as_str())
                .fetch_optional(&mut *conn)
                .await?;
        known.map(|_| ()).ok_or(StorageError::NotFound("project"))
    }
}

impl Projects {
    /// Retained project metadata for plan previews that also describe removed projects.
    pub(crate) async fn retained_metadata_in(
        conn: &mut SqliteConnection,
        id: &ProjectId,
    ) -> Result<Option<(String, Option<String>, bool)>, StorageError> {
        Ok(
            sqlx::query_as("SELECT name,directory,removed_at IS NOT NULL FROM project WHERE id=?")
                .bind(id.as_str())
                .fetch_optional(conn)
                .await?,
        )
    }
    pub(crate) async fn is_live_in(
        conn: &mut SqliteConnection,
        id: &ProjectId,
    ) -> Result<bool, StorageError> {
        match Self::require_live_in(conn, id).await {
            Ok(()) => Ok(true),
            Err(StorageError::NotFound(_)) => Ok(false),
            Err(error) => Err(error),
        }
    }
}

impl Projects {
    /// Retained metadata is needed to answer a replay after removal.
    pub(crate) async fn retained_in(
        conn: &mut SqliteConnection,
        id: &ProjectId,
    ) -> Result<Project, StorageError> {
        load_project(conn, id).await
    }

    /// Soft removal keeps the row referenced by the journal and reserves its slug.
    pub(crate) async fn mark_removed_in(
        conn: &mut SqliteConnection,
        id: &ProjectId,
        ts: &str,
    ) -> Result<(), StorageError> {
        Self::require_live_in(conn, id).await?;
        sqlx::query("UPDATE project SET removed_at = ? WHERE id = ?")
            .bind(ts)
            .bind(id.as_str())
            .execute(conn)
            .await?;
        Ok(())
    }
}
