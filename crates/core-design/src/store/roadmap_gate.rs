//! The gates that the roadmap places on hand outcome edits and on plan writes.

use crate::{DesignAnchor, WorkspaceOp};
use crate::{ProjectId, StorageError};
use sqlx::SqliteConnection;

/// A hand outcome edit, or a plan link to an outcome, needs an approved roadmap.
impl RoadmapGateRows {
    pub(crate) async fn check_hand_edit(
        conn: &mut SqliteConnection,
        project: &ProjectId,
        ops: &[WorkspaceOp],
    ) -> Result<(), crate::DesignError> {
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
            return Err(crate::DesignError::StageBlocked(
                "the first roadmap comes from the Planner: ask it to propose one".into(),
            ));
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
impl crate::Design {
    pub async fn require_roadmap_in(
        conn: &mut SqliteConnection,
        project: &ProjectId,
    ) -> Result<(), crate::DesignError> {
        if roadmap_revision(conn, project).await? == 0 {
            return Err(crate::DesignError::StageBlocked(
                "no roadmap approved: propose the roadmap first".into(),
            ));
        }
        Ok(())
    }
}

pub(crate) struct RoadmapGateRows;
