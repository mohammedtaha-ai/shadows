//! One job: the rows of the links and the active limit (spec §15.6). Each
//! command is one `write_txn`, as `save_planner_instructions` is: `classify`
//! for a replay, the change, its event, `record_command`.

use sqlx::SqliteConnection;

use crate::code::model::ProjectLink;
use crate::command::CommandContext;
use crate::db::{Storage, StorageError, append_event, classify, now, record_command};
use crate::events::{Actor, DurableEvent};
use crate::projects::ProjectId;

/// A link's slugs and date: the reader's, the read one's, when.
type LinkRow = (String, String, String);

/// The select every link read shares; each adds its `WHERE`.
macro_rules! link_select {
    () => {
        "SELECT p.slug, q.slug, l.created_at FROM project_link l
           JOIN project p ON p.id = l.project_id
           JOIN project q ON q.id = l.linked_project_id"
    };
}

impl Storage {
    /// How many projects are active at once: `code_setting.active_limit`.
    pub(in crate::code) async fn code_active_limit(&self) -> Result<i64, StorageError> {
        Ok(
            sqlx::query_scalar("SELECT active_limit FROM code_setting WHERE id = 1")
                .fetch_one(self.reader())
                .await?,
        )
    }

    /// The projects `project` reads, by the linked slug.
    pub(in crate::code) async fn code_links(
        &self,
        project: &ProjectId,
    ) -> Result<Vec<ProjectLink>, StorageError> {
        let rows: Vec<LinkRow> = sqlx::query_as(concat!(
            link_select!(),
            " WHERE l.project_id = ? ORDER BY q.slug"
        ))
        .bind(project.as_str())
        .fetch_all(self.reader())
        .await?;
        Ok(rows.into_iter().map(into_link).collect())
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
        self.write_txn(move |conn| {
            Box::pin(async move {
                if let Some(done) = classify(conn, &ctx, "Project", project.as_str()).await? {
                    let done = ProjectId::from_stored(done);
                    return replayed_link(conn, &ctx, &project, &done).await.map(Some);
                }
                if !is_project(conn, &project).await? {
                    return Err(StorageError::NotFound("project"));
                }
                if !is_project(conn, &linked).await? {
                    return Ok(None);
                }
                let made = sqlx::query(
                    "INSERT INTO project_link (project_id, linked_project_id, created_at)
                     VALUES (?,?,?) ON CONFLICT DO NOTHING",
                )
                .bind(project.as_str())
                .bind(linked.as_str())
                .bind(&ts)
                .execute(&mut *conn)
                .await?
                .rows_affected();
                if made == 1 {
                    let event = DurableEvent::new("ProjectLinked", Actor::user(&ctx.principal_id))
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
                record_command(conn, &ctx, scope, key, "ProjectLink", linked.as_str(), &ts).await?;
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
        self.write_txn(move |conn| {
            Box::pin(async move {
                if classify(conn, &ctx, "Project", project.as_str())
                    .await?
                    .is_some()
                {
                    return Ok(true);
                }
                let gone = sqlx::query(
                    "DELETE FROM project_link WHERE project_id = ? AND linked_project_id = ?",
                )
                .bind(project.as_str())
                .bind(linked.as_str())
                .execute(&mut *conn)
                .await?
                .rows_affected();
                if gone == 0 {
                    return Ok(false);
                }
                let event = DurableEvent::new("ProjectUnlinked", Actor::user(&ctx.principal_id))
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
                record_command(conn, &ctx, scope, key, "ProjectLink", linked.as_str(), &ts).await?;
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
        self.write_txn(move |conn| {
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
                let event = DurableEvent::new("CodeActiveLimitSet", Actor::user(&ctx.principal_id))
                    .with_payload(serde_json::json!({ "active_limit": limit }));
                append_event(conn, &event, &ts).await?;
                let set = limit.to_string();
                record_command(conn, &ctx, "Code", "settings", "CodeSetting", &set, &ts).await?;
                Ok(limit)
            })
        })
        .await
    }
}

/// Inside the caller's write: the project's links, the ones it reads and
/// the ones reading it. Projects' `remove_project` calls it (§15.4); a
/// removed link journals no `ProjectUnlinked`, the `ProjectRemoved` says it.
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

fn into_link((project, linked, created_at): LinkRow) -> ProjectLink {
    ProjectLink {
        project,
        linked,
        created_at,
    }
}

async fn load_link(
    conn: &mut SqliteConnection,
    project: &ProjectId,
    linked: &ProjectId,
) -> Result<ProjectLink, StorageError> {
    let row: LinkRow = sqlx::query_as(concat!(
        link_select!(),
        " WHERE l.project_id = ? AND l.linked_project_id = ?"
    ))
    .bind(project.as_str())
    .bind(linked.as_str())
    .fetch_optional(&mut *conn)
    .await?
    .ok_or(StorageError::NotFound("project link"))?;
    Ok(into_link(row))
}

/// What a replayed `ProjectLinkPut` answers (§14.9: what it answered first).
/// The link as it stands; once removed, the link as the command left it,
/// dated when the command was recorded — which is the link's own date unless
/// the command found the link already there. Never NotFound: the command
/// happened, and a client retrying a lost answer must not read it as failed.
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
    let row: LinkRow = sqlx::query_as(
        "SELECT p.slug, q.slug, c.recorded_at FROM command_record c
           JOIN project p ON p.id = c.command_scope_key
           JOIN project q ON q.id = ?
          WHERE c.principal_kind = ? AND c.principal_id = ?
            AND c.command_scope_kind = 'Project' AND c.command_scope_key = ?
            AND c.command_id = ?",
    )
    .bind(linked.as_str())
    .bind(&ctx.principal_kind)
    .bind(&ctx.principal_id)
    .bind(project.as_str())
    .bind(&ctx.command_id)
    .fetch_one(&mut *conn)
    .await?;
    Ok(into_link(row))
}

async fn is_project(conn: &mut SqliteConnection, id: &ProjectId) -> Result<bool, StorageError> {
    let known: Option<i64> =
        sqlx::query_scalar("SELECT 1 FROM project WHERE id = ? AND removed_at IS NULL")
            .bind(id.as_str())
            .fetch_optional(&mut *conn)
            .await?;
    Ok(known.is_some())
}
