//! Plan-to-part associations inside the plan command transaction.
#[cfg(feature = "test-support")]
use super::PlansStore;
use crate::{PlanId, Plans};
use crate::{ProjectId, StorageError};
use core_design::PartId;
use sqlx::SqliteConnection;
impl Plans {
    pub async fn link_plan_parts_in(
        conn: &mut SqliteConnection,
        project: &ProjectId,
        plan: &PlanId,
        parts: &[PartId],
    ) -> Result<(), StorageError> {
        for part in parts {
            core_design::Design::require_part_in(conn, project, part).await?;
            sqlx::query(
                "INSERT INTO design_part_plan(project_id,part_id,plan_id) VALUES (?,?,?)
             ON CONFLICT DO NOTHING",
            )
            .bind(project.as_str())
            .bind(part.as_str())
            .bind(plan.as_str())
            .execute(&mut *conn)
            .await?;
        }
        Ok(())
    }
}

#[cfg(feature = "test-support")]
impl PlansStore {
    pub async fn link_plan_part_for_tests(
        &self,
        p: &ProjectId,
        plan: &PlanId,
        part: &PartId,
    ) -> Result<(), StorageError> {
        let (p, plan, part) = (p.clone(), plan.clone(), part.clone());
        self.storage
            .write_txn(move |conn| {
                Box::pin(async move {
                    Plans::link_plan_parts_in(conn, &p, &plan, std::slice::from_ref(&part)).await
                })
            })
            .await
    }
}

impl Plans {
    pub(super) async fn plan_parts_in(
        conn: &mut SqliteConnection,
        project: &ProjectId,
        plan: &PlanId,
    ) -> Result<Vec<PartId>, StorageError> {
        let ids: Vec<String> = sqlx::query_scalar(
            "SELECT part_id FROM design_part_plan WHERE project_id=? AND plan_id=?
             ORDER BY part_id",
        )
        .bind(project.as_str())
        .bind(plan.as_str())
        .fetch_all(conn)
        .await?;
        Ok(ids.into_iter().map(PartId::from_recorded).collect())
    }
}

impl Plans {
    /// A plan's parts: its saved links, then each part a task writes into,
    /// each once.
    pub(crate) fn effective_parts(
        explicit: &[PartId],
        tasks: &[crate::task_context::TaskOwnership],
    ) -> Vec<PartId> {
        let mut parts = explicit.to_vec();
        for owner in tasks
            .iter()
            .flat_map(|task| &task.writes)
            .flat_map(|scope| &scope.owners)
        {
            if !parts.contains(&owner.part_id) {
                parts.push(owner.part_id.clone());
            }
        }
        parts
    }

    /// The approval preconditions `parts` and `outcomes` leave unmet, part first.
    pub(crate) fn missing_delivery_requirements(
        parts: &[PartId],
        outcomes: &[core_design::OutcomeId],
    ) -> Vec<crate::model::DeliveryRequirement> {
        let mut missing = Vec::new();
        if parts.is_empty() {
            missing.push(crate::model::DeliveryRequirement::Part);
        }
        if outcomes.is_empty() {
            missing.push(crate::model::DeliveryRequirement::Outcome);
        }
        missing
    }
}

impl Plans {
    /// The delivery projection of the version `plan` as the caller's
    /// snapshot sees it: its outcomes, its effective parts, each task's
    /// ownership and, for an Active plan's Draft, what approval still needs.
    pub(super) async fn plan_delivery_in(
        conn: &mut SqliteConnection,
        plan: &crate::Plan,
    ) -> Result<crate::PlanDelivery, crate::PlansError> {
        let project = &plan.project_id;
        let layout = core_design::Design::layout_in(conn, project).await?;
        let tasks: Vec<crate::TaskDelivery> = plan
            .tasks
            .iter()
            .map(|t| crate::TaskDelivery {
                number: t.content.number,
                ownership: crate::TaskOwnership::resolve(&layout, &t.content),
            })
            .collect();
        let ownership: Vec<_> = tasks.iter().map(|t| t.ownership.clone()).collect();
        let explicit = Self::plan_parts_in(conn, project, &plan.plan_id).await?;
        let parts = Self::effective_parts(&explicit, &ownership);
        let mut task_parts = Vec::new();
        for owner in ownership
            .iter()
            .flat_map(|o| o.reads.iter().chain(&o.writes))
            .flat_map(|scope| &scope.owners)
        {
            if !task_parts.contains(&owner.part_id) {
                task_parts.push(owner.part_id.clone());
            }
        }
        let outcomes = Self::delivery_outcomes_in(conn, project, &plan.plan_id).await?;
        let drafting = plan.state == crate::WorkflowState::Draft
            && plan.plan_state == crate::PlanState::Active;
        let approval_missing = if drafting {
            let ids: Vec<_> = outcomes.iter().map(|o| o.outcome_id.clone()).collect();
            Self::missing_delivery_requirements(&parts, &ids)
        } else {
            Vec::new()
        };
        let named = |ids: Vec<PartId>| {
            ids.into_iter()
                .map(|id| crate::DeliveryPart {
                    title: layout
                        .parts
                        .iter()
                        .find(|p| p.id == id)
                        .map(|p| p.title.clone())
                        .unwrap_or_default(),
                    part_id: id,
                })
                .collect::<Vec<_>>()
        };
        Ok(crate::PlanDelivery {
            map_revision: layout.map_revision,
            outcomes,
            parts: named(parts),
            task_parts: named(task_parts),
            tasks,
            approval_missing,
        })
    }
}
