//! Plan-owned workspace associations.
use crate::plans::Plans;
use crate::{ProjectId, StorageError};
use core_design::{DesignAnchor, WorkspaceOp};
use sqlx::SqliteConnection;

impl Plans {
    pub(crate) async fn apply_design_links_in(
        conn: &mut SqliteConnection,
        project: &ProjectId,
        ops: &[WorkspaceOp],
    ) -> Result<(), StorageError> {
        for op in ops {
            let (anchor, plan, remove) = match op {
                WorkspaceOp::PlanLinkPut { anchor, plan } => (anchor, plan, false),
                WorkspaceOp::PlanLinkRemove { anchor, plan } => (anchor, plan, true),
                _ => continue,
            };
            let (id, query) = match (anchor, remove) {
                (DesignAnchor::Part(id), false) => (
                    id.as_str(),
                    sqlx::query(
                        "INSERT OR IGNORE INTO design_part_plan(project_id,part_id,plan_id) \
                     VALUES (?,?,?)",
                    ),
                ),
                (DesignAnchor::Part(id), true) => (
                    id.as_str(),
                    sqlx::query(
                        "DELETE FROM design_part_plan \
                         WHERE project_id=? AND part_id=? AND plan_id=?",
                    ),
                ),
                (DesignAnchor::Outcome(id), false) => (
                    id.as_str(),
                    sqlx::query(
                        "INSERT OR IGNORE INTO design_outcome_plan(project_id,outcome_id,plan_id) \
                     VALUES (?,?,?)",
                    ),
                ),
                (DesignAnchor::Outcome(id), true) => (
                    id.as_str(),
                    sqlx::query(
                        "DELETE FROM design_outcome_plan \
                     WHERE project_id=? AND outcome_id=? AND plan_id=?",
                    ),
                ),
            };
            query
                .bind(project.as_str())
                .bind(id)
                .bind(plan)
                .execute(&mut *conn)
                .await?;
        }
        Ok(())
    }
}

impl Plans {
    /// The project's outcomes that at least one plan links to.
    pub(crate) async fn outcomes_with_plans_in(
        conn: &mut SqliteConnection,
        project: &ProjectId,
    ) -> Result<std::collections::BTreeSet<String>, StorageError> {
        let rows: Vec<String> = sqlx::query_scalar(
            "SELECT DISTINCT outcome_id FROM design_outcome_plan \
             WHERE project_id=? ORDER BY outcome_id",
        )
        .bind(project.as_str())
        .fetch_all(conn)
        .await?;
        Ok(rows.into_iter().collect())
    }

    pub(crate) async fn anchor_plans_in(
        conn: &mut SqliteConnection,
        project: &ProjectId,
        anchor: &DesignAnchor,
    ) -> Result<Vec<crate::plans::PlanId>, StorageError> {
        let (query, id) = match anchor {
            DesignAnchor::Part(id) => (
                sqlx::query_scalar::<_, String>(
                    "SELECT plan_id FROM design_part_plan \
                 WHERE project_id=? AND part_id=? ORDER BY plan_id",
                ),
                id.as_str(),
            ),
            DesignAnchor::Outcome(id) => (
                sqlx::query_scalar::<_, String>(
                    "SELECT plan_id FROM design_outcome_plan \
                 WHERE project_id=? AND outcome_id=? ORDER BY plan_id",
                ),
                id.as_str(),
            ),
        };
        let rows: Vec<String> = query
            .bind(project.as_str())
            .bind(id)
            .fetch_all(conn)
            .await?;
        Ok(rows
            .into_iter()
            .map(crate::plans::PlanId::from_stored)
            .collect())
    }
}
