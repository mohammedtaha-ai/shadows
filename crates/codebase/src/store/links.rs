//! Link persistence.

use sqlx::SqliteConnection;

use super::CodeStore;
use crate::Codebase;
use crate::model::ProjectLink;
use core_db::{CommandContext, StorageError};
use core_projects::{ProjectId, Projects};

impl CodeStore {
    /// The projects `project` reads, by the linked slug.
    pub(in super::super) async fn code_links(
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
                let linked =
                    Projects::slug_in(&mut snapshot, &ProjectId::from_recorded(id)).await?;
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
}

impl Codebase {
    /// Inserts a link in the caller's transaction. The caller owns validation,
    /// command replay, events and dependency notifications. True only for a new row.
    pub async fn put_link_in(
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
    pub async fn remove_link_in(
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
    pub async fn delete_code_links_in(
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

impl Codebase {
    pub async fn load_link_in(
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
}

impl Codebase {
    pub async fn replayed_link_in(
        conn: &mut SqliteConnection,
        ctx: &CommandContext,
        project: &ProjectId,
        linked: &ProjectId,
    ) -> Result<ProjectLink, StorageError> {
        match Self::load_link_in(conn, project, linked).await {
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
}
