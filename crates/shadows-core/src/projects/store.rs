//! One job: a project's rows — the project itself and the modes it allows.

use std::collections::BTreeMap;

use sqlx::SqliteConnection;

use super::directory::ProjectDirectory;
use super::model::{Project, ProjectId};
use crate::code::{delete_code_index_in, delete_code_links_in};
use crate::command::CommandContext;
use crate::db::{Storage, StorageError, append_event, classify, now, record_command};
use crate::events::{Actor, DurableEvent};
use crate::grants::revoke_project_grants_in;
use shadows_agent::policy;

impl Storage {
    pub async fn create_project(
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
            now(),
        );
        self.write_txn(move |conn| {
            Box::pin(async move {
                if let Some(existing_id) = classify(conn, &ctx, "Global", "").await? {
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

                append_event(
                    conn,
                    &DurableEvent::new("ProjectCreated", Actor::user(&ctx.principal_id))
                        .with_project(&id)
                        .with_payload(serde_json::json!({
                            "slug": slug, "name": name, "directory": directory,
                        })),
                    &ts,
                )
                .await?;

                record_command(conn, &ctx, "Global", "", "Project", id.as_str(), &ts).await?;
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
    pub async fn list_projects(&self) -> Result<Vec<Project>, StorageError> {
        let rows: Vec<ProjectRow> = sqlx::query_as(
            "SELECT p.id, p.slug, p.name, p.directory, p.created_at
               FROM project p
               LEFT JOIN durable_event e
                 ON e.project_id = p.id AND e.kind = 'ProjectCreated'
              WHERE p.removed_at IS NULL
              ORDER BY e.seq, p.id",
        )
        .fetch_all(self.reader())
        .await?;
        let mut modes = all_modes(self.reader()).await?;
        Ok(rows
            .into_iter()
            .map(|row| {
                let allowed = modes.remove(&row.0).unwrap_or_default();
                project(row, allowed)
            })
            .collect())
    }

    /// The project, NotFound once removed (§4.2).
    pub async fn get_project(&self, id: &ProjectId) -> Result<Project, StorageError> {
        let mut conn = self.reader().acquire().await?;
        live_project(&mut conn, id).await?;
        load_project(&mut conn, id).await
    }

    /// Removes the project (§4.2, §15.4): it stays a row, since the journal
    /// references it, but is read nowhere again. Refused while it holds a
    /// planning thread. One write: its index, its links both ways and its
    /// live project grants go with it. A replay answers the project as the
    /// command left it.
    pub async fn remove_project(
        &self,
        ctx: &CommandContext,
        project_id: &ProjectId,
    ) -> Result<Project, StorageError> {
        let (ctx, id, ts) = (ctx.clone(), project_id.clone(), now());
        self.write_txn(move |conn| {
            Box::pin(async move {
                if classify(conn, &ctx, "Project", id.as_str())
                    .await?
                    .is_some()
                {
                    return load_project(conn, &id).await;
                }
                live_project(conn, &id).await?;
                let threads: i64 = sqlx::query_scalar(
                    "SELECT COUNT(*) FROM planning_thread \
                         WHERE project_id = ? AND removed_at IS NULL",
                )
                .bind(id.as_str())
                .fetch_one(&mut *conn)
                .await?;
                if threads > 0 {
                    return Err(StorageError::ProjectHasThreads);
                }
                sqlx::query("UPDATE project SET removed_at = ? WHERE id = ?")
                    .bind(&ts)
                    .bind(id.as_str())
                    .execute(&mut *conn)
                    .await?;
                let actor = Actor::user(&ctx.principal_id);
                crate::plans::notify_project_dependencies_in(conn, &id, &actor, &ts).await?;
                delete_code_index_in(conn, &id).await?;
                delete_code_links_in(conn, &id).await?;
                revoke_project_grants_in(conn, &id, actor.clone(), &ts).await?;
                append_event(
                    conn,
                    &DurableEvent::new("ProjectRemoved", actor).with_project(&id),
                    &ts,
                )
                .await?;
                let key = id.as_str();
                record_command(conn, &ctx, "Project", key, "Project", key, &ts).await?;
                load_project(conn, &id).await
            })
        })
        .await
    }

    /// Replaces the modes the project allows for each harness named in
    /// `modes` (spec §12.5); harnesses not named keep theirs. An idempotent
    /// command: a replay answers the project as it now stands. The caller
    /// checks every mode is in its harness's policy.
    pub async fn set_project_modes(
        &self,
        ctx: &CommandContext,
        project_id: &ProjectId,
        modes: &BTreeMap<String, Vec<String>>,
    ) -> Result<Project, StorageError> {
        let (ctx, id, modes, ts) = (ctx.clone(), project_id.clone(), modes.clone(), now());
        self.write_txn(move |conn| {
            Box::pin(async move {
                if classify(conn, &ctx, "Project", id.as_str())
                    .await?
                    .is_some()
                {
                    return load_project(conn, &id).await;
                }
                live_project(conn, &id).await?;
                replace_modes(conn, &id, &modes).await?;
                append_event(
                    conn,
                    &DurableEvent::new("ProjectModesChanged", Actor::user(&ctx.principal_id))
                        .with_project(&id)
                        .with_payload(serde_json::json!({ "allowed_modes": modes })),
                    &ts,
                )
                .await?;
                record_command(
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

type ProjectRow = (String, String, String, Option<String>, String);

fn project(
    (id, slug, name, directory, created_at): ProjectRow,
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
    }
}

/// NotFound unless the project exists and is not removed (§4.2).
async fn live_project(conn: &mut SqliteConnection, id: &ProjectId) -> Result<(), StorageError> {
    let live: Option<i64> =
        sqlx::query_scalar("SELECT 1 FROM project WHERE id = ? AND removed_at IS NULL")
            .bind(id.as_str())
            .fetch_optional(&mut *conn)
            .await?;
    live.map(|_| ()).ok_or(StorageError::NotFound("project"))
}

/// The project's row, removed or not: what a replay answers.
async fn load_project(
    conn: &mut SqliteConnection,
    id: &ProjectId,
) -> Result<Project, StorageError> {
    let row: ProjectRow =
        sqlx::query_as("SELECT id, slug, name, directory, created_at FROM project WHERE id = ?")
            .bind(id.as_str())
            .fetch_optional(&mut *conn)
            .await?
            .ok_or(StorageError::NotFound("project"))?;
    let modes = project_modes(conn, id).await?;
    Ok(project(row, modes))
}
