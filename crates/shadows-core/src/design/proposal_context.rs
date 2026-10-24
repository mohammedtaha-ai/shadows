//! Shared identity and authority for Planner proposals.
use super::Design;
use crate::{CoreError, ErrorCode, Grant, OperationId};
use core_db::{Anchor, CommandContext};
use core_grants::Writer;
impl Design {
    pub(super) async fn proposal_writer(
        &self,
        grant: &Grant,
        scope_message: &str,
        running_message: &str,
    ) -> Result<(Writer, OperationId), CoreError> {
        let writer = Writer::from_grant(grant, Some("thread grant needs its conversation"))?;
        let Writer::Planner { thread, .. } = &writer else {
            return Err(CoreError::Refused {
                code: ErrorCode::GrantScope,
                message: scope_message.into(),
            });
        };
        let turn =
            self.operations
                .running_for(thread)
                .await?
                .ok_or_else(|| CoreError::Refused {
                    code: ErrorCode::InvalidCommand,
                    message: running_message.into(),
                })?;
        Ok((writer, turn))
    }
    pub(super) fn proposal_command(
        writer: &Writer,
        turn: &OperationId,
        kind: &str,
        args: &impl serde::Serialize,
    ) -> Result<CommandContext, CoreError> {
        let fp = CommandContext::fingerprint(kind, &serde_json::to_value(args)?);
        let (principal_kind, principal_id) = writer.principal();
        Ok(CommandContext {
            agent: None,
            principal_kind: principal_kind.into(),
            principal_id,
            command_id: Anchor::derived_id(Anchor::Operation(turn.as_str()), &fp),
            command_kind: kind.into(),
            command_schema_ver: 1,
            request_fingerprint: fp,
        })
    }
}
