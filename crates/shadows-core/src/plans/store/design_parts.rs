//! Plan-to-part associations inside the plan command transaction.
#[cfg(feature = "test-support")]
use super::PlansStore;
use crate::design::PartId;
use crate::plans::{PlanId, Plans};
use crate::{ProjectId, StorageError};
use sqlx::SqliteConnection;
impl Plans {
    pub(crate) async fn plan_has_part_in(
        conn: &mut SqliteConnection,
        project: &ProjectId,
        plan: &PlanId,
    ) -> Result<bool, StorageError> {
        let linked: Option<i64> = sqlx::query_scalar(
            "SELECT 1 FROM design_part_plan WHERE project_id=? AND plan_id=? LIMIT 1",
        )
        .bind(project.as_str())
        .bind(plan.as_str())
        .fetch_optional(&mut *conn)
        .await?;
        Ok(linked.is_some())
    }

    pub(crate) async fn link_plan_parts_in(
        conn: &mut SqliteConnection,
        project: &ProjectId,
        plan: &PlanId,
        parts: &[PartId],
    ) -> Result<(), StorageError> {
        for part in parts {
            crate::design::Design::require_part_in(conn, project, part).await?;
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
