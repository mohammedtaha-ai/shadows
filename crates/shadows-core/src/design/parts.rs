//! Part read entry points.
use super::{Design, PartId, PartPage};
use crate::{CoreError, ErrorCode, ProjectId};

impl Design {
    pub async fn parts(
        &self,
        project: &ProjectId,
        parent: Option<&PartId>,
        after: Option<&PartId>,
    ) -> Result<PartPage, CoreError> {
        self.store
            .design_parts(project, parent, after)
            .await
            .map_err(|error| match error {
                core_db::StorageError::Constraint(message) => CoreError::Refused {
                    code: ErrorCode::InvalidCommand,
                    message,
                },
                other => other.into(),
            })
    }
}
