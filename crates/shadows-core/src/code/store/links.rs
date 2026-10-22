//! Link command persistence and the active limit.

use sqlx::SqliteConnection;

use super::CodeStore;
use crate::code::Code;
use crate::code::model::ProjectLink;
use crate::db::{Actor, CommandContext, StorageError, append_event, classify, now, record_command};
use crate::events::DurableEvent;
use crate::projects::{ProjectId, Projects};

impl CodeStore {
    /// How many projects are active at once: `code_setting.active_limit`.
    pub(in crate::code) async fn code_active_limit(&self) -> Result<i64, StorageError> {
        Ok(
            sqlx::query_scalar("SELECT active_limit FROM code_setting WHERE id = 1")
                .fetch_one(self.storage.reader())
                .await?,
        )
    }

    /// The projects `project` reads, by the linked slug.
    pub(in crate::code) async fn code_links(
        &self,
        project: &ProjectId,
    ) -> Result<Vec<ProjectLink>, StorageError> {
        let mut snapshot = self.storage.reader().begin().await?;
        let rows: Vec<(String, String)> = sqlx::query_as(
            "SELECT linked_project_id, created_at FROM project_link
              WHERE project_id = ? ORDER BY linked_project_id",
        )
        .bind(project.as_str())
        .fetch_all(&mut *snapshot)
        .await?;
        let mut out = Vec::new();
        if !rows.is_empty() {
            let slug = Projects::slug_in(&mut snapshot, project).await?;
            for (id, created_at) in rows {
                let linked = Projects::slug_in(&mut snapshot, &ProjectId::from_stored(id)).await?;
                out.push(ProjectLink {
                    project: slug.clone(),
                    linked,
                    created_at,
                });
            }
        }
        out.sort_by(|a, b| a.linked.cmp(&b.linked));
        snapshot.commit().await?;
        Ok(out)
    }

    /// Links `project` to `linked`, or answers the link already there. `None`
    /// when `linked` is no project; NotFound when `project` is none. A replay
    /// answers the link it made, even once removed (`replayed_link`). Only a
    /// new link is a `ProjectLinked` event.
    pub(in crate::code) async fn put_code_link(
        &self,
        ctx: &CommandContext,
        project: &ProjectId,
        linked: &ProjectId,
    ) -> Result<Option<ProjectLink>, StorageError> {
        let (ctx, project, linked, ts) = (ctx.clone(), project.clone(), linked.clone(), now());
        self.storage
            .write_txn(move |conn| {
                Box::pin(async move {
                    if let Some(done) = classify(conn, &ctx, "Project", project.as_str()).await? {
                        let done = ProjectId::from_stored(done);
                        return replayed_link(conn, &ctx, &project, &done).await.map(Some);
                    }
                    Projects::require_live_in(conn, &project).await?;
                    match Projects::require_live_in(conn, &linked).await {
                        Ok(()) => {}
                        Err(StorageError::NotFound(_)) => return Ok(None),
                        Err(error) => return Err(error),
                    }
                    let made = Code::put_link_in(conn, &project, &linked, &ts).await?;
                    if made {
                        let event =
                            DurableEvent::new("ProjectLinked", Actor::user(&ctx.principal_id))
                                .with_project(&project)
                                .with_payload(serde_json::json!({ "linked": linked }));
                        append_event(conn, &event, &ts).await?;
                        crate::plans::notify_project_dependencies_in(
                            conn,
                            &project,
                            &Actor::user(&ctx.principal_id),
                            &ts,
                        )
                        .await?;
                    }
                    let (scope, key) = ("Project", project.as_str());
                    record_command(conn, &ctx, scope, key, "ProjectLink", linked.as_str(), &ts)
                        .await?;
                    load_link(conn, &project, &linked).await.map(Some)
                })
            })
            .await
    }

    /// Removes the link; `false` when there is none. A replay answers `true`.
    pub(in crate::code) async fn remove_code_link(
        &self,
        ctx: &CommandContext,
        project: &ProjectId,
        linked: &ProjectId,
    ) -> Result<bool, StorageError> {
        let (ctx, project, linked, ts) = (ctx.clone(), project.clone(), linked.clone(), now());
        self.storage
            .write_txn(move |conn| {
                Box::pin(async move {
                    if classify(conn, &ctx, "Project", project.as_str())
                        .await?
                        .is_some()
                    {
                        return Ok(true);
                    }
                    if !Code::remove_link_in(conn, &project, &linked).await? {
                        return Ok(false);
                    }
                    let event =
                        DurableEvent::new("ProjectUnlinked", Actor::user(&ctx.principal_id))
                            .with_project(&project)
                            .with_payload(serde_json::json!({ "linked": linked }));
                    append_event(conn, &event, &ts).await?;
                    crate::plans::notify_project_dependencies_in(
                        conn,
                        &project,
                        &Actor::user(&ctx.principal_id),
                        &ts,
                    )
                    .await?;
                    let (scope, key) = ("Project", project.as_str());
                    record_command(conn, &ctx, scope, key, "ProjectLink", linked.as_str(), &ts)
                        .await?;
                    Ok(true)
                })
            })
            .await
    }

    /// Sets `code_setting.active_limit`, already checked 1..=20. A replay
    /// answers the limit it set, which its outcome ref holds.
    pub(in crate::code) async fn set_code_active_limit(
        &self,
        ctx: &CommandContext,
        limit: u32,
    ) -> Result<u32, StorageError> {
        let (ctx, ts) = (ctx.clone(), now());
        self.storage
            .write_txn(move |conn| {
                Box::pin(async move {
                    if let Some(done) = classify(conn, &ctx, "Code", "settings").await? {
                        return done
                            .parse()
                            .map_err(|_| StorageError::NotFound("active limit"));
                    }
                    sqlx::query("UPDATE code_setting SET active_limit = ? WHERE id = 1")
                        .bind(i64::from(limit))
                        .execute(&mut *conn)
                        .await?;
                    let event =
                        DurableEvent::new("CodeActiveLimitSet", Actor::user(&ctx.principal_id))
                            .with_payload(serde_json::json!({ "active_limit": limit }));
                    append_event(conn, &event, &ts).await?;
                    let set = limit.to_string();
                    record_command(conn, &ctx, "Code", "settings", "CodeSetting", &set, &ts)
                        .await?;
                    Ok(limit)
                })
            })
            .await
    }
}

impl Code {
    /// Inserts a link in the caller's transaction. The caller owns validation,
    /// command replay, events and dependency notifications. True only for a new row.
    pub(crate) async fn put_link_in(
        conn: &mut SqliteConnection,
        project: &ProjectId,
        linked: &ProjectId,
        ts: &str,
    ) -> Result<bool, StorageError> {
        Ok(sqlx::query(
            "INSERT INTO project_link (project_id, linked_project_id, created_at)
             VALUES (?,?,?) ON CONFLICT DO NOTHING",
        )
        .bind(project.as_str())
        .bind(linked.as_str())
        .bind(ts)
        .execute(&mut *conn)
        .await?
        .rows_affected()
            == 1)
    }

    /// Removes a link in the caller's transaction. True only when a row existed.
    pub(crate) async fn remove_link_in(
        conn: &mut SqliteConnection,
        project: &ProjectId,
        linked: &ProjectId,
    ) -> Result<bool, StorageError> {
        Ok(
            sqlx::query("DELETE FROM project_link WHERE project_id = ? AND linked_project_id = ?")
                .bind(project.as_str())
                .bind(linked.as_str())
                .execute(&mut *conn)
                .await?
                .rows_affected()
                == 1,
        )
    }

    /// Removes links in both directions in the caller's project-removal write.
    pub(crate) async fn delete_code_links_in(
        conn: &mut SqliteConnection,
        project: &ProjectId,
    ) -> Result<(), StorageError> {
        sqlx::query("DELETE FROM project_link WHERE project_id = ? OR linked_project_id = ?")
            .bind(project.as_str())
            .bind(project.as_str())
            .execute(&mut *conn)
            .await?;
        Ok(())
    }
}

async fn load_link(
    conn: &mut SqliteConnection,
    project: &ProjectId,
    linked: &ProjectId,
) -> Result<ProjectLink, StorageError> {
    let created_at = sqlx::query_scalar(
        "SELECT created_at FROM project_link WHERE project_id = ? AND linked_project_id = ?",
    )
    .bind(project.as_str())
    .bind(linked.as_str())
    .fetch_optional(&mut *conn)
    .await?
    .ok_or(StorageError::NotFound("project link"))?;
    Ok(ProjectLink {
        project: Projects::slug_in(conn, project).await?,
        linked: Projects::slug_in(conn, linked).await?,
        created_at,
    })
}

/// A retry must answer a committed command even after its link is gone.
async fn replayed_link(
    conn: &mut SqliteConnection,
    ctx: &CommandContext,
    project: &ProjectId,
    linked: &ProjectId,
) -> Result<ProjectLink, StorageError> {
    match load_link(conn, project, linked).await {
        Err(StorageError::NotFound(_)) => {}
        found => return found,
    }
    let created_at = sqlx::query_scalar(
        "SELECT recorded_at FROM command_record
          WHERE principal_kind = ? AND principal_id = ?
            AND command_scope_kind = 'Project' AND command_scope_key = ?
            AND command_id = ?",
    )
    .bind(&ctx.principal_kind)
    .bind(&ctx.principal_id)
    .bind(project.as_str())
    .bind(&ctx.command_id)
    .fetch_one(&mut *conn)
    .await?;
    Ok(ProjectLink {
        project: Projects::slug_in(conn, project).await?,
        linked: Projects::slug_in(conn, linked).await?,
        created_at,
    })
}
