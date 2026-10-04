//! Snapshot reads of roadmap outcomes.
use crate::{
    ProjectId,
    db::{Storage, StorageError},
    design::{Outcome, OutcomeId, OutcomePage, OutcomeView, PartId},
    plans::PlanId,
};
use sqlx::{FromRow, SqliteConnection};
#[derive(FromRow)]
struct OutcomeRow {
    id: String,
    parent_id: Option<String>,
    revision: i64,
    ordinal: i64,
    content_json: String,
}
impl OutcomeRow {
    fn domain(self) -> Result<Outcome, StorageError> {
        Ok(Outcome {
            id: OutcomeId::from_stored(self.id),
            parent: self.parent_id.map(OutcomeId::from_stored),
            revision: self.revision,
            ordinal: self.ordinal,
            content: serde_json::from_str(&self.content_json)?,
        })
    }
}
pub(super) async fn load(
    conn: &mut SqliteConnection,
    project: &ProjectId,
    id: &OutcomeId,
) -> Result<Outcome, StorageError> {
    sqlx::query_as::<_,OutcomeRow>("SELECT id,parent_id,revision,ordinal,content_json FROM design_outcome WHERE project_id=? AND id=?").bind(project.as_str()).bind(id.as_str()).fetch_optional(conn).await?.ok_or(StorageError::NotFound("outcome"))?.domain()
}
impl Storage {
    pub async fn design_outcome(
        &self,
        project: &ProjectId,
        id: &OutcomeId,
    ) -> Result<OutcomeView, StorageError> {
        let mut tx = self.reader().begin().await?;
        let revision = super::parts::revision(&mut tx, project).await?;
        let outcome = load(&mut tx, project, id).await?;
        let mut ancestors = Vec::new();
        let mut parent = outcome.parent.clone();
        while let Some(id) = parent {
            let node = load(&mut tx, project, &id).await?;
            parent = node.parent.clone();
            ancestors.push(node);
        }
        ancestors.reverse();
        let plans:Vec<String>=sqlx::query_scalar("SELECT plan_id FROM design_outcome_plan WHERE project_id=? AND outcome_id=? ORDER BY plan_id").bind(project.as_str()).bind(id.as_str()).fetch_all(&mut *tx).await?;
        let parts:Vec<String>=sqlx::query_scalar("SELECT part_id FROM design_outcome_part WHERE project_id=? AND outcome_id=? ORDER BY part_id").bind(project.as_str()).bind(id.as_str()).fetch_all(&mut *tx).await?;
        Ok(OutcomeView {
            revision,
            outcome,
            ancestors,
            parts: parts.into_iter().map(PartId::from_stored).collect(),
            plans: plans.into_iter().map(PlanId::from_stored).collect(),
        })
    }
    pub async fn design_outcomes(
        &self,
        project: &ProjectId,
        parent: Option<&OutcomeId>,
        after: Option<&OutcomeId>,
    ) -> Result<OutcomePage, StorageError> {
        let mut tx = self.reader().begin().await?;
        let revision = super::parts::revision(&mut tx, project).await?;
        if let Some(id) = parent {
            load(&mut tx, project, id).await?;
        }
        let cursor = if let Some(id) = after {
            let item = load(&mut tx, project, id).await?;
            if item.parent.as_ref() != parent {
                return Err(StorageError::Constraint(format!(
                    "outcome cursor {id} is outside the requested parent"
                )));
            }
            Some(item)
        } else {
            None
        };
        let rows=sqlx::query_as::<_,OutcomeRow>("SELECT id,parent_id,revision,ordinal,content_json FROM design_outcome WHERE project_id=? AND parent_id IS ? AND (? IS NULL OR (ordinal,id) > (?,?)) ORDER BY ordinal,id LIMIT 51").bind(project.as_str()).bind(parent.map(OutcomeId::as_str)).bind(after.map(OutcomeId::as_str)).bind(cursor.as_ref().map(|p|p.ordinal)).bind(after.map(OutcomeId::as_str)).fetch_all(&mut *tx).await?;
        let has_more = rows.len() > 50;
        let items = rows
            .into_iter()
            .take(50)
            .map(OutcomeRow::domain)
            .collect::<Result<Vec<_>, _>>()?;
        let next = if has_more {
            items.last().map(|o| o.id.clone())
        } else {
            None
        };
        Ok(OutcomePage {
            revision,
            items,
            next,
        })
    }
}
