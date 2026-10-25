//! Roadmap read entry points.
use super::{Design, OutcomeId, OutcomePage};
use crate::{DesignError, ProjectId};
impl Design {
    pub async fn outcomes(
        &self,
        project: &ProjectId,
        parent: Option<&OutcomeId>,
        after: Option<&OutcomeId>,
    ) -> Result<OutcomePage, DesignError> {
        self.store
            .design_outcomes(project, parent, after)
            .await
            .map_err(|error| match error {
                core_db::StorageError::Constraint(message) => DesignError::InvalidCommand(message),
                other => other.into(),
            })
    }
}
