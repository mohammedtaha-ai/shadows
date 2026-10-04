//! Snapshot reads of project parts with bounded child pages.
use crate::{
    ProjectId,
    db::{Storage, StorageError},
    design::{Part, PartContent, PartId, PartPage, PartView},
    plans::PlanId,
};
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

pub(super) async fn revision(
    conn: &mut SqliteConnection,
    project: &ProjectId,
) -> Result<i64, StorageError> {
    sqlx::query_scalar(
        "SELECT COALESCE(w.revision,0) FROM project p \
         LEFT JOIN design_workspace w ON w.project_id=p.id \
         WHERE p.id=? AND p.removed_at IS NULL",
    )
    .bind(project.as_str())
    .fetch_optional(conn)
    .await?
    .ok_or(StorageError::NotFound("project"))
}

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

impl Storage {
    pub async fn design_part(
        &self,
        project: &ProjectId,
        id: &PartId,
    ) -> Result<PartView, StorageError> {
        let mut tx = self.reader().begin().await?;
        let revision = revision(&mut tx, project).await?;
        let part = load(&mut tx, project, id).await?;
        let mut ancestors = Vec::new();
        let mut parent = part.parent.clone();
        while let Some(id) = parent {
            let node = load(&mut tx, project, &id).await?;
            parent = node.parent.clone();
            ancestors.push(node);
        }
        ancestors.reverse();
        let plans: Vec<String> = sqlx::query_scalar(
            "SELECT plan_id FROM design_part_plan \
             WHERE project_id=? AND part_id=? ORDER BY plan_id",
        )
        .bind(project.as_str())
        .bind(id.as_str())
        .fetch_all(&mut *tx)
        .await?;
        Ok(PartView {
            revision,
            part,
            ancestors,
            plans: plans.into_iter().map(PlanId::from_stored).collect(),
        })
    }

    pub async fn design_parts(
        &self,
        project: &ProjectId,
        parent: Option<&PartId>,
        after: Option<&PartId>,
    ) -> Result<PartPage, StorageError> {
        let mut tx = self.reader().begin().await?;
        let revision = revision(&mut tx, project).await?;
        if let Some(id) = parent {
            load(&mut tx, project, id).await?;
        }
        let cursor = if let Some(id) = after {
            let part = load(&mut tx, project, id).await?;
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
