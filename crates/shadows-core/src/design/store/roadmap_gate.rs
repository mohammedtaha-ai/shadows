//! The gates that the roadmap places on hand outcome edits and on plan writes (§23.6).

use crate::design::{DesignAnchor, DesignOp, PartId};
use crate::plans::PlanId;
use crate::{ProjectId, StorageError};
use sqlx::SqliteConnection;

/// A hand outcome edit, or a plan link to an outcome, needs an approved roadmap.
pub(super) async fn check_hand_edit(
    conn: &mut SqliteConnection,
    project: &ProjectId,
    ops: &[DesignOp],
) -> Result<(), StorageError> {
    let touches_outcomes = ops.iter().any(|op| {
        matches!(
            op,
            DesignOp::OutcomeCreate { .. }
                | DesignOp::OutcomePut { .. }
                | DesignOp::OutcomeMove { .. }
                | DesignOp::OutcomePartPut { .. }
                | DesignOp::OutcomePartRemove { .. }
                | DesignOp::PlanLinkPut {
                    anchor: DesignAnchor::Outcome(_),
                    ..
                }
                | DesignOp::PlanLinkRemove {
                    anchor: DesignAnchor::Outcome(_),
                    ..
                }
        )
    });
    if touches_outcomes && roadmap_revision(conn, project).await? == 0 {
        return Err(StorageError::StageBlocked(
            "the first roadmap comes from the Planner: ask it to propose one".into(),
        ));
    }
    Ok(())
}

async fn roadmap_revision(
    conn: &mut SqliteConnection,
    project: &ProjectId,
) -> Result<i64, StorageError> {
    let revision: Option<i64> =
        sqlx::query_scalar("SELECT roadmap_revision FROM design_workspace WHERE project_id=?")
            .bind(project.as_str())
            .fetch_optional(&mut *conn)
            .await?;
    Ok(revision.unwrap_or(0))
}

/// Plans calls this inside every plan write's transaction: no plan work before a roadmap.
pub(crate) async fn require_roadmap_in(
    conn: &mut SqliteConnection,
    project: &ProjectId,
) -> Result<(), StorageError> {
    if roadmap_revision(conn, project).await? == 0 {
        return Err(StorageError::StageBlocked(
            "no roadmap approved: propose the roadmap first".into(),
        ));
    }
    Ok(())
}

/// Whether `plan` is linked to at least one part of `project`.
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

/// Ties `plan` to `parts`. The plan write's own event covers the change.
pub(crate) async fn link_plan_parts_in(
    conn: &mut SqliteConnection,
    project: &ProjectId,
    plan: &PlanId,
    parts: &[PartId],
) -> Result<(), StorageError> {
    for part in parts {
        let known: Option<i64> =
            sqlx::query_scalar("SELECT 1 FROM design_part WHERE project_id=? AND id=?")
                .bind(project.as_str())
                .bind(part.as_str())
                .fetch_optional(&mut *conn)
                .await?;
        if known.is_none() {
            return Err(StorageError::Constraint(format!(
                "part {part} does not belong to project {project}"
            )));
        }
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
