//! Part read entry points with normalization for workspace edits.
use super::{Design, DesignAnchor, DesignOp, PartId, PartPage, PartView};
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
        Ok(self.storage.design_parts(project, parent, after).await?)
    }
}

pub(super) fn normalize(ops: &mut [DesignOp]) -> Result<(), CoreError> {
    let invalid = |detail: String| CoreError::Refused {
        code: ErrorCode::InvalidCommand,
        message: detail,
    };
    let valid_id = |id: &str| {
        uuid::Uuid::parse_str(id)
            .map(|_| ())
            .map_err(|_| invalid(format!("invalid design reference {id}: expected UUID")))
    };
    for op in ops {
        match op {
            DesignOp::VisionPut { .. } => {}
            DesignOp::PartCreate {
                id,
                parent,
                before,
                content,
            } => {
                valid_id(id.as_str())?;
                for reference in [parent.as_ref(), before.as_ref()].into_iter().flatten() {
                    valid_id(reference.as_str())?;
                }
                content.title = content.title.trim().into();
                if content.title.is_empty() {
                    return Err(invalid(format!("part {id} needs a title")));
                }
            }
            DesignOp::PartPut { id, content } => {
                valid_id(id.as_str())?;
                content.title = content.title.trim().into();
                if content.title.is_empty() {
                    return Err(invalid(format!("part {id} needs a title")));
                }
            }
            DesignOp::PartMove { id, parent, before } => {
                valid_id(id.as_str())?;
                for reference in [parent.as_ref(), before.as_ref()].into_iter().flatten() {
                    valid_id(reference.as_str())?;
                }
            }
            DesignOp::PlanLinkPut {
                anchor: DesignAnchor::Part(id),
                plan,
            }
            | DesignOp::PlanLinkRemove {
                anchor: DesignAnchor::Part(id),
                plan,
            } => {
                valid_id(id.as_str())?;
                valid_id(plan.as_str())?;
            }
        }
    }
    Ok(())
}
