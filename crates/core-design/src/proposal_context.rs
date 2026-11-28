//! Shared authority for proposals.
use super::Design;
use crate::{DesignError, Grant, OperationId};
use core_db::{Anchor, CommandContext};
use core_grants::Writer;
impl Design {
    pub(super) async fn proposal_writer(
        &self,
        grant: &Grant,
        scope_message: &str,
        running_message: &str,
    ) -> Result<(Writer, Option<OperationId>), DesignError> {
        let writer = Writer::from_grant(grant, Some("thread grant needs its conversation"))?;
        let turn = match &writer {
            Writer::Planner { thread, .. } => Some(
                self.operations
                    .running_for(thread)
                    .await?
                    .ok_or_else(|| DesignError::InvalidCommand(running_message.into()))?,
            ),
            Writer::External { .. } => None,
            Writer::Person => return Err(DesignError::GrantScope(scope_message.into())),
        };
        Ok((writer, turn))
    }

    pub(super) fn external_command_id(command_id: Option<String>) -> Result<String, DesignError> {
        command_id
            .map(|id| id.trim().to_owned())
            .filter(|id| !id.is_empty())
            .ok_or_else(|| {
                DesignError::InvalidCommand("an external agent names its command_id".into())
            })
    }

    pub(super) fn proposal_command(
        writer: &Writer,
        turn: Option<&OperationId>,
        command_id: Option<String>,
        kind: &str,
        args: &impl serde::Serialize,
    ) -> Result<CommandContext, DesignError> {
        let fp = CommandContext::fingerprint(kind, &serde_json::to_value(args)?);
        let (principal_kind, principal_id) = writer.principal();
        let command_id = match turn {
            Some(turn) => Anchor::derived_id(Anchor::Operation(turn.as_str()), &fp),
            None => Self::external_command_id(command_id)?,
        };
        Ok(CommandContext {
            agent: None,
            principal_kind: principal_kind.into(),
            principal_id,
            command_id,
            command_kind: kind.into(),
            command_schema_ver: 1,
            request_fingerprint: fp,
        })
    }
}
