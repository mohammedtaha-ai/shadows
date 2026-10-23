//! The gates that the roadmap places on hand outcome edits and on plan writes.

use crate::design::{DesignAnchor, WorkspaceOp};
use crate::{ProjectId, StorageError};
use sqlx::SqliteConnection;

/// A hand outcome edit, or a plan link to an outcome, needs an approved roadmap.
impl RoadmapGateRows {
    pub(in crate::design) async fn check_hand_edit(
        conn: &mut SqliteConnection,
        project: &ProjectId,
        ops: &[WorkspaceOp],
    ) -> Result<(), crate::CoreError> {
        let touches_outcomes = ops.iter().any(|op| {
            matches!(
                op,
                WorkspaceOp::OutcomeCreate { .. }
                    | WorkspaceOp::OutcomePut { .. }
                    | WorkspaceOp::OutcomeMove { .. }
                    | WorkspaceOp::OutcomePartPut { .. }
                    | WorkspaceOp::OutcomePartRemove { .. }
                    | WorkspaceOp::PlanLinkPut {
                        anchor: DesignAnchor::Outcome(_),
                        ..
                    }
                    | WorkspaceOp::PlanLinkRemove {
                        anchor: DesignAnchor::Outcome(_),
                        ..
                    }
            )
        });
        if touches_outcomes && roadmap_revision(conn, project).await? == 0 {
            return Err(crate::design::DesignError::StageBlocked(
                "the first roadmap comes from the Planner: ask it to propose one".into(),
            )
            .into());
        }
        Ok(())
    }
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
impl crate::design::Design {
    pub(crate) async fn require_roadmap_in(
        conn: &mut SqliteConnection,
        project: &ProjectId,
    ) -> Result<(), crate::CoreError> {
        if roadmap_revision(conn, project).await? == 0 {
            return Err(crate::design::DesignError::StageBlocked(
                "no roadmap approved: propose the roadmap first".into(),
            )
            .into());
        }
        Ok(())
    }
}

pub(in crate::design) struct RoadmapGateRows;
