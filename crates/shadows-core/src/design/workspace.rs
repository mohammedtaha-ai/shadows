//! A grant's read of its project's workspace, so a Planner plans from it.
use super::{Design, WorkspaceView};
use crate::{CoreError, Grant};

impl Design {
    /// The vision with the top-level parts and outcomes, each a first page
    /// with its cursor. Vision lists include stable item ids; this read writes nothing.
    pub async fn workspace_for(&self, grant: &Grant) -> Result<WorkspaceView, CoreError> {
        self.store
            .check_agreement_grant(
                &super::agreement_scope::AgreementScope::writer(grant)?,
                &grant.project_id,
            )
            .await?;
        let project = &grant.project_id;
        Ok(WorkspaceView {
            vision: self.vision(project).await?,
            parts: self.parts(project, None, None).await?,
            outcomes: self.outcomes(project, None, None).await?,
        })
    }
}
