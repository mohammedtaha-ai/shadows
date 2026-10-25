//! Snapshot reads of project parts with bounded child pages.
use super::DesignStore;
use crate::{Part, PartContent, PartDetails, PartId, PartPage, ProjectId};
use core_db::StorageError;
use sqlx::{FromRow, SqliteConnection};

#[derive(FromRow)]
struct PartRow {
    id: String,
    parent_id: Option<String>,
    revision: i64,
    ordinal: i64,
    content_json: String,
}
impl PartRow {
    fn domain(self) -> Result<Part, StorageError> {
        Ok(Part {
            id: PartId::from_stored(self.id),
            parent: self.parent_id.map(PartId::from_stored),
            revision: self.revision,
            ordinal: self.ordinal,
            content: serde_json::from_str::<PartContent>(&self.content_json)?,
        })
    }
}

impl PartsRows {
    pub(super) async fn revision(
        conn: &mut SqliteConnection,
        project: &ProjectId,
    ) -> Result<i64, StorageError> {
        core_projects::Projects::require_live_in(conn, project).await?;
        Ok(
            sqlx::query_scalar("SELECT revision FROM design_workspace WHERE project_id=?")
                .bind(project.as_str())
                .fetch_optional(conn)
                .await?
                .unwrap_or(0),
        )
    }
}

impl PartsRows {
    pub(super) async fn load(
        conn: &mut SqliteConnection,
        project: &ProjectId,
        id: &PartId,
    ) -> Result<Part, StorageError> {
        sqlx::query_as::<_, PartRow>(
            "SELECT id,parent_id,revision,ordinal,content_json FROM design_part \
         WHERE project_id=? AND id=?",
        )
        .bind(project.as_str())
        .bind(id.as_str())
        .fetch_optional(conn)
        .await?
        .ok_or(StorageError::NotFound("part"))?
        .domain()
    }
}

impl DesignStore {
    pub async fn design_parts(
        &self,
        project: &ProjectId,
        parent: Option<&PartId>,
        after: Option<&PartId>,
    ) -> Result<PartPage, StorageError> {
        let mut tx = self.storage.reader().begin().await?;
        let revision = PartsRows::revision(&mut tx, project).await?;
        if let Some(id) = parent {
            PartsRows::load(&mut tx, project, id).await?;
        }
        let cursor = if let Some(id) = after {
            let part = PartsRows::load(&mut tx, project, id).await?;
            if part.parent.as_ref() != parent {
                return Err(StorageError::Constraint(format!(
                    "part cursor {id} is outside the requested parent"
                )));
            }
            Some(part)
        } else {
            None
        };
        let rows = sqlx::query_as::<_, PartRow>(
            "SELECT id,parent_id,revision,ordinal,content_json FROM design_part \
             WHERE project_id=? AND parent_id IS ? \
             AND (? IS NULL OR (ordinal,id) > (?,?)) \
             ORDER BY ordinal,id LIMIT 51",
        )
        .bind(project.as_str())
        .bind(parent.map(PartId::as_str))
        .bind(after.map(PartId::as_str))
        .bind(cursor.as_ref().map(|p| p.ordinal))
        .bind(after.map(PartId::as_str))
        .fetch_all(&mut *tx)
        .await?;
        let has_more = rows.len() > 50;
        let items = rows
            .into_iter()
            .take(50)
            .map(PartRow::domain)
            .collect::<Result<Vec<_>, _>>()?;
        let next = if has_more {
            items.last().map(|p| p.id.clone())
        } else {
            None
        };
        Ok(PartPage {
            revision,
            items,
            next,
        })
    }
}

impl crate::Design {
    pub async fn design_part_in(
        tx: &mut SqliteConnection,
        project: &ProjectId,
        id: &PartId,
    ) -> Result<PartDetails, StorageError> {
        let revision = PartsRows::revision(tx, project).await?;
        let part = PartsRows::load(tx, project, id).await?;
        let mut ancestors = Vec::new();
        let mut parent = part.parent.clone();
        while let Some(id) = parent {
            let node = PartsRows::load(tx, project, &id).await?;
            parent = node.parent.clone();
            ancestors.push(node);
        }
        ancestors.reverse();
        Ok(PartDetails {
            revision,
            part,
            ancestors,
        })
    }
}

impl crate::Design {
    pub async fn require_part_in(
        conn: &mut SqliteConnection,
        project: &ProjectId,
        id: &PartId,
    ) -> Result<(), StorageError> {
        PartsRows::load(conn, project, id)
            .await
            .map(|_| ())
            .map_err(|error| match error {
                StorageError::NotFound(_) => StorageError::Constraint(format!(
                    "part {id} does not belong to project {project}"
                )),
                other => other,
            })
    }
}

impl crate::Design {
    pub async fn revision_in(
        conn: &mut SqliteConnection,
        project: &ProjectId,
    ) -> Result<i64, StorageError> {
        PartsRows::revision(conn, project).await
    }
    pub async fn part_in(
        conn: &mut SqliteConnection,
        project: &ProjectId,
        id: &PartId,
    ) -> Result<Part, StorageError> {
        PartsRows::load(conn, project, id).await
    }
}

pub(crate) struct PartsRows;
