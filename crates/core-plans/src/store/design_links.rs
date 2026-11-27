//! Plan-owned workspace associations.
use crate::Plans;
use crate::{ProjectId, StorageError};
use core_design::{DesignAnchor, WorkspaceOp};
use sqlx::SqliteConnection;

impl Plans {
    pub async fn apply_design_links_in(
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
    pub async fn outcomes_with_plans_in(
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

    pub async fn anchor_plans_in(
        conn: &mut SqliteConnection,
        project: &ProjectId,
        anchor: &DesignAnchor,
    ) -> Result<Vec<crate::PlanId>, StorageError> {
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
        Ok(rows.into_iter().map(crate::PlanId::from_stored).collect())
    }
}

impl Plans {
    /// The outcomes `plan` is linked to now, ordered by outcome id.
    pub(crate) async fn plan_outcome_ids_in(
        conn: &mut SqliteConnection,
        project: &ProjectId,
        plan: &crate::PlanId,
    ) -> Result<Vec<core_design::OutcomeId>, StorageError> {
        let rows: Vec<String> = sqlx::query_scalar(
            "SELECT outcome_id FROM design_outcome_plan \
             WHERE project_id=? AND plan_id=? ORDER BY outcome_id",
        )
        .bind(project.as_str())
        .bind(plan.as_str())
        .fetch_all(conn)
        .await?;
        Ok(rows
            .into_iter()
            .map(core_design::OutcomeId::from_recorded)
            .collect())
    }

    /// The title of `plan`'s latest version, which names the plan to a person.
    pub(crate) async fn latest_title_in(
        conn: &mut SqliteConnection,
        plan: &crate::PlanId,
    ) -> Result<String, StorageError> {
        sqlx::query_scalar(
            "SELECT title FROM workflow WHERE plan_id = ? ORDER BY version DESC LIMIT 1",
        )
        .bind(plan.as_str())
        .fetch_optional(conn)
        .await?
        .ok_or(StorageError::NotFound("workflow"))
    }

    /// Whether `plan` is Active with an approved version: such a plan keeps
    /// at least one outcome link once it has one.
    pub(crate) async fn keeps_outcome_link_in(
        conn: &mut SqliteConnection,
        plan: &str,
    ) -> Result<bool, StorageError> {
        let found: Option<i64> = sqlx::query_scalar(
            "SELECT 1 FROM plan p JOIN workflow w ON w.plan_id = p.id \
             WHERE p.id = ? AND p.state = 'Active' AND w.state = 'Frozen' LIMIT 1",
        )
        .bind(plan)
        .fetch_optional(conn)
        .await?;
        Ok(found.is_some())
    }
}

#[cfg(feature = "test-support")]
impl super::PlansStore {
    pub async fn link_plan_outcome_for_tests(
        &self,
        p: &ProjectId,
        plan: &crate::PlanId,
        outcome: &core_design::OutcomeId,
    ) -> Result<(), StorageError> {
        let (p, plan, outcome) = (p.clone(), plan.clone(), outcome.clone());
        self.storage
            .write_txn(move |conn| {
                Box::pin(async move {
                    Plans::check_design_plan_in(conn, &p, &plan).await?;
                    core_design::Design::design_outcome_in(conn, &p, &outcome).await?;
                    let op = WorkspaceOp::PlanLinkPut {
                        anchor: DesignAnchor::Outcome(outcome),
                        plan: plan.to_string(),
                    };
                    Plans::apply_design_links_in(conn, &p, std::slice::from_ref(&op)).await
                })
            })
            .await
    }
}

impl Plans {
    /// The outcomes `plan` serves now with their titles, ordered by outcome id.
    pub(crate) async fn delivery_outcomes_in(
        conn: &mut SqliteConnection,
        project: &ProjectId,
        plan: &crate::PlanId,
    ) -> Result<Vec<crate::DeliveryOutcome>, StorageError> {
        let mut outcomes = Vec::new();
        for id in Self::plan_outcome_ids_in(conn, project, plan).await? {
            let details = core_design::Design::design_outcome_in(conn, project, &id).await?;
            outcomes.push(crate::DeliveryOutcome {
                outcome_id: id,
                title: details.outcome.content.title,
            });
        }
        Ok(outcomes)
    }
}
