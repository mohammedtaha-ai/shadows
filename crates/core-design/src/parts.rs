//! Part read entry points.
use super::{Design, PartId, PartPage};
use crate::{DesignError, ProjectId};

impl Design {
    pub async fn parts(
        &self,
        project: &ProjectId,
        parent: Option<&PartId>,
        after: Option<&PartId>,
    ) -> Result<PartPage, DesignError> {
        self.store
            .design_parts(project, parent, after)
            .await
            .map_err(|error| match error {
                core_db::StorageError::Constraint(message) => DesignError::InvalidCommand(message),
                other => other.into(),
            })
    }
}
