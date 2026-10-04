//! A grant's read of its project's workspace, so a Planner plans from it.
use super::{Design, WorkspaceView};
use crate::{CoreError, Grant};

impl Design {
    /// The vision with the top-level parts and outcomes, each a first page
    /// with its cursor (§18.10). Read only: no tool writes the workspace.
    pub async fn workspace_for(&self, grant: &Grant) -> Result<WorkspaceView, CoreError> {
        self.storage
            .check_agreement_grant(&super::agreement_scope::writer(grant)?, &grant.project_id)
            .await?;
        let project = &grant.project_id;
        Ok(WorkspaceView {
            vision: self.vision(project).await?,
            parts: self.parts(project, None, None).await?,
            outcomes: self.outcomes(project, None, None).await?,
        })
    }
}
