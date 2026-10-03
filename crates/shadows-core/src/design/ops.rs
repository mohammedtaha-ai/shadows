//! Normalization of workspace edit operations.
use super::{DesignAnchor, DesignOp};
use crate::{CoreError, ErrorCode};

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
            DesignOp::PlanLinkPut { anchor, plan } | DesignOp::PlanLinkRemove { anchor, plan } => {
                valid_id(match anchor {
                    DesignAnchor::Part(id) => id.as_str(),
                    DesignAnchor::Outcome(id) => id.as_str(),
                })?;
                valid_id(plan.as_str())?;
            }
            DesignOp::OutcomeCreate {
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
                    return Err(invalid(format!("outcome {id} needs a title")));
                }
            }
            DesignOp::OutcomePut { id, content } => {
                valid_id(id.as_str())?;
                content.title = content.title.trim().into();
                if content.title.is_empty() {
                    return Err(invalid(format!("outcome {id} needs a title")));
                }
            }
            DesignOp::OutcomeMove { id, parent, before } => {
                valid_id(id.as_str())?;
                for reference in [parent.as_ref(), before.as_ref()].into_iter().flatten() {
                    valid_id(reference.as_str())?;
                }
            }
            DesignOp::OutcomePartPut { outcome, part }
            | DesignOp::OutcomePartRemove { outcome, part } => {
                valid_id(outcome.as_str())?;
                valid_id(part.as_str())?;
            }
        }
    }
    Ok(())
}
