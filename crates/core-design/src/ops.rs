//! Normalization of workspace edit operations.
use super::{DesignAnchor, WorkspaceOp};
use crate::DesignError;

impl Ops {
    pub(super) fn normalize(ops: &mut [WorkspaceOp]) -> Result<(), DesignError> {
        let invalid = |detail: String| DesignError::InvalidCommand(detail);
        let valid_id = |id: &str| {
            uuid::Uuid::parse_str(id)
                .map(|_| ())
                .map_err(|_| invalid(format!("invalid design reference {id}: expected UUID")))
        };
        for op in ops {
            match op {
                WorkspaceOp::VisionItemAdd { text, .. }
                | WorkspaceOp::VisionItemPut { text, .. } => {
                    if text.trim().is_empty() {
                        return Err(invalid("a vision item needs text".into()));
                    }
                }
                WorkspaceOp::VisionItemRemove { .. } => {}
                WorkspaceOp::PartCreate {
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
                    super::map_rules::MapRules::normalise(content).map_err(invalid)?;
                    if content.title.is_empty() {
                        return Err(invalid(format!("part {id} needs a title")));
                    }
                }
                WorkspaceOp::PartPut { id, content } => {
                    valid_id(id.as_str())?;
                    content.title = content.title.trim().into();
                    super::map_rules::MapRules::normalise(content).map_err(invalid)?;
                    if content.title.is_empty() {
                        return Err(invalid(format!("part {id} needs a title")));
                    }
                }
                WorkspaceOp::PartMove { id, parent, before } => {
                    valid_id(id.as_str())?;
                    for reference in [parent.as_ref(), before.as_ref()].into_iter().flatten() {
                        valid_id(reference.as_str())?;
                    }
                }
                WorkspaceOp::PlanLinkPut { anchor, plan }
                | WorkspaceOp::PlanLinkRemove { anchor, plan } => {
                    valid_id(match anchor {
                        DesignAnchor::Part(id) => id.as_str(),
                        DesignAnchor::Outcome(id) => id.as_str(),
                    })?;
                    valid_id(plan.as_str())?;
                }
                WorkspaceOp::OutcomeCreate {
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
                WorkspaceOp::OutcomePut { id, content } => {
                    valid_id(id.as_str())?;
                    content.title = content.title.trim().into();
                    if content.title.is_empty() {
                        return Err(invalid(format!("outcome {id} needs a title")));
                    }
                }
                WorkspaceOp::OutcomeMove { id, parent, before } => {
                    valid_id(id.as_str())?;
                    for reference in [parent.as_ref(), before.as_ref()].into_iter().flatten() {
                        valid_id(reference.as_str())?;
                    }
                }
                WorkspaceOp::OutcomePartPut { outcome, part }
                | WorkspaceOp::OutcomePartRemove { outcome, part } => {
                    valid_id(outcome.as_str())?;
                    valid_id(part.as_str())?;
                }
            }
        }
        Ok(())
    }
}

pub struct Ops;
