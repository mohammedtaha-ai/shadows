//! Person adoption of exact agreement versions through the plan edit transaction.
use super::{EditOutcome, PlanOp, Plans, WorkflowId};
use crate::{CoreError, ErrorCode, app::user_command, command::Writer};

impl Plans {
    pub async fn edit_bindings(
        &self,
        command_id: String,
        workflow: &WorkflowId,
        expected_revision: i64,
        ops: Vec<PlanOp>,
    ) -> Result<EditOutcome, CoreError> {
        if ops
            .iter()
            .any(|op| !matches!(op, PlanOp::BindingPut { .. } | PlanOp::BindingRemove { .. }))
        {
            return Err(CoreError::Refused {
                code: ErrorCode::InvalidCommand,
                message: "this operation changes agreement bindings only".into(),
            });
        }
        let ctx = user_command(
            command_id,
            "PlanBindingsEdit",
            serde_json::json!({
                "workflow_id":workflow,"expected_revision":expected_revision,"ops":ops
            }),
        );
        self.storage
            .edit_plan(
                &ctx,
                &Writer::Person,
                None,
                workflow,
                expected_revision,
                &ops,
            )
            .await
            .map_err(|e| match e {
                crate::StorageError::Constraint(message) => CoreError::Refused {
                    code: ErrorCode::InvalidCommand,
                    message,
                },
                other => other.into(),
            })
    }
}
