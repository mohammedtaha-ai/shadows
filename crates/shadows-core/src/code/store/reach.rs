//! Live one-way project reach inside a caller's database snapshot.

use sqlx::SqliteConnection;

use crate::db::StorageError;
use crate::projects::ProjectId;

pub(crate) async fn project_reachable_in(
    conn: &mut SqliteConnection,
    source: &ProjectId,
    target: &ProjectId,
) -> Result<bool, StorageError> {
    Ok(sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM project p JOIN project q
           WHERE p.id=? AND q.id=? AND p.removed_at IS NULL AND q.removed_at IS NULL
             AND (p.id=q.id OR EXISTS(SELECT 1 FROM project_link l
               WHERE l.project_id=p.id AND l.linked_project_id=q.id)))",
    )
    .bind(source.as_str())
    .bind(target.as_str())
    .fetch_one(&mut *conn)
    .await?)
}

pub(crate) async fn selected_project_in(
    conn: &mut SqliteConnection,
    origin: &ProjectId,
    slug: Option<&str>,
) -> Result<ProjectId, StorageError> {
    let selected = match slug {
        None => origin.clone(),
        Some(slug) => {
            let id: String =
                sqlx::query_scalar("SELECT id FROM project WHERE slug=? AND removed_at IS NULL")
                    .bind(slug)
                    .fetch_optional(&mut *conn)
                    .await?
                    .ok_or(StorageError::GrantScope)?;
            ProjectId::from_stored(id)
        }
    };
    if !project_reachable_in(conn, origin, &selected).await? {
        return Err(StorageError::GrantScope);
    }
    Ok(selected)
}
