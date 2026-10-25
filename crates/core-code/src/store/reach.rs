//! Live one-way project reach inside a caller's database snapshot.

use sqlx::SqliteConnection;

use crate::Code;
use core_db::StorageError;
use core_projects::{ProjectId, Projects};

impl Code {
    /// Live same-project or direct-link reach in the caller's snapshot.
    pub async fn project_reachable_in(
        conn: &mut SqliteConnection,
        source: &ProjectId,
        target: &ProjectId,
    ) -> Result<bool, StorageError> {
        for project in [source, target] {
            match Projects::require_live_in(conn, project).await {
                Ok(()) => {}
                Err(StorageError::NotFound(_)) => return Ok(false),
                Err(error) => return Err(error),
            }
        }
        if source == target {
            return Ok(true);
        }
        Ok(sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM project_link
              WHERE project_id = ? AND linked_project_id = ?)",
        )
        .bind(source.as_str())
        .bind(target.as_str())
        .fetch_one(&mut *conn)
        .await?)
    }

    /// Resolves a live project within the origin's one-way scope in its snapshot.
    pub async fn selected_project_in(
        conn: &mut SqliteConnection,
        origin: &ProjectId,
        slug: Option<&str>,
    ) -> Result<ProjectId, crate::CodeError> {
        let selected = match slug {
            None => origin.clone(),
            Some(slug) => {
                Projects::by_slug_in(conn, slug)
                    .await?
                    .ok_or(crate::CodeError::GrantScope(
                        "outside what the grant allows".into(),
                    ))?
                    .id
            }
        };
        if !Self::project_reachable_in(conn, origin, &selected).await? {
            return Err(crate::CodeError::GrantScope(
                "outside what the grant allows".into(),
            ));
        }
        Ok(selected)
    }
}
