//! Part read entry points.
use super::{Design, PartId, PartPage, PartView};
use crate::{CoreError, ErrorCode, ProjectId};

impl Design {
    pub async fn part(&self, project: &ProjectId, id: &PartId) -> Result<PartView, CoreError> {
        Ok(self.storage.design_part(project, id).await?)
    }
    pub async fn parts(
        &self,
        project: &ProjectId,
        parent: Option<&PartId>,
        after: Option<&PartId>,
    ) -> Result<PartPage, CoreError> {
        self.storage
            .design_parts(project, parent, after)
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
