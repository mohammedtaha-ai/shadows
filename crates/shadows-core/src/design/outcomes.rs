//! Roadmap read entry points.
use super::{Design, OutcomeId, OutcomePage, OutcomeView};
use crate::{CoreError, ErrorCode, ProjectId};
impl Design {
    pub async fn outcome(
        &self,
        project: &ProjectId,
        id: &OutcomeId,
    ) -> Result<OutcomeView, CoreError> {
        Ok(self.storage.design_outcome(project, id).await?)
    }
    pub async fn outcomes(
        &self,
        project: &ProjectId,
        parent: Option<&OutcomeId>,
        after: Option<&OutcomeId>,
    ) -> Result<OutcomePage, CoreError> {
        self.storage
            .design_outcomes(project, parent, after)
            .await
            .map_err(|error| match error {
                crate::db::StorageError::Constraint(message) => CoreError::Refused {
                    code: ErrorCode::InvalidCommand,
                    message,
                },
                other => other.into(),
            })
    }
}
