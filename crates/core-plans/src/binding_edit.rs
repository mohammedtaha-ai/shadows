//! Person adoption of exact agreement versions through the plan edit transaction.
use super::{EditOutcome, PlanOp, Plans, WorkflowId};
use crate::PlansError;
use core_db::CommandContext;
use core_grants::Writer;

impl Plans {
    pub async fn edit_bindings(
        &self,
        command_id: String,
        workflow: &WorkflowId,
        expected_revision: i64,
        ops: Vec<PlanOp>,
    ) -> Result<EditOutcome, PlansError> {
        if ops
            .iter()
            .any(|op| !matches!(op, PlanOp::BindingPut { .. } | PlanOp::BindingRemove { .. }))
        {
            return Err(PlansError::InvalidCommand(
                "this operation changes agreement bindings only".into(),
            ));
        }
        let ctx = CommandContext::user(
            command_id,
            "PlanBindingsEdit",
            serde_json::json!({
                "workflow_id":workflow,"expected_revision":expected_revision,"ops":ops
            }),
        );
        self.store
            .edit_plan(
                &ctx,
                &Writer::Person,
                None,
                workflow,
                expected_revision,
                &ops,
            )
            .await
            .map_err(PlansError::invalid_constraint)
    }
}
