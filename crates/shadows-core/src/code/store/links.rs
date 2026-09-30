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
    /// answers the link it made. Only a new link is a `ProjectLinked` event.
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
                    return load_link(conn, &project, &done).await.map(Some);
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

async fn is_project(conn: &mut SqliteConnection, id: &ProjectId) -> Result<bool, StorageError> {
    let known: Option<i64> = sqlx::query_scalar("SELECT 1 FROM project WHERE id = ?")
        .bind(id.as_str())
        .fetch_optional(&mut *conn)
        .await?;
    Ok(known.is_some())
}
