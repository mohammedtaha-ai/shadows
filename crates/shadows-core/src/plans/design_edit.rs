//! Atomic workspace edits with plan-owned associations.
use super::{DesignOp, Plans};
use crate::{CoreError, ProjectId};
use core_db::CommandContext;
use core_design::{Design, DesignChange, WorkspaceOp};

impl Plans {
    pub async fn edit_design(
        &self,
        command_id: String,
        project: &ProjectId,
        expected_revision: i64,
        ops: Vec<DesignOp>,
    ) -> Result<DesignChange, CoreError> {
        let mut ops: Vec<WorkspaceOp> = ops.into_iter().map(Into::into).collect();
        Design::normalize_ops(&mut ops)?;
        let ctx = CommandContext::user(
            command_id,
            "DesignEdit",
            serde_json::json!({
                "project":project,"expected_revision":expected_revision,"ops":ops
            }),
        );
        self.edit_workspace_with(ctx, project, expected_revision, ops)
            .await
    }
    pub(super) async fn edit_workspace_with(
        &self,
        ctx: CommandContext,
        project: &ProjectId,
        expected_revision: i64,
        ops: Vec<WorkspaceOp>,
    ) -> Result<DesignChange, CoreError> {
        let project = project.clone();
        self.store
            .storage
            .write_txn_as::<_, _, CoreError>(move |conn| {
                Box::pin(async move {
                    if let Some(change) = Design::replay_edit_in(conn, &ctx, &project).await? {
                        return Ok(change);
                    }
                    let mut facts = std::collections::BTreeMap::new();
                    for op in &ops {
                        if let WorkspaceOp::PlanLinkPut { plan, .. }
                        | WorkspaceOp::PlanLinkRemove { plan, .. } = op
                        {
                            let error = match Self::check_design_plan_in(
                                conn,
                                &project,
                                &super::PlanId::from_stored(plan.clone()),
                            )
                            .await
                            {
                                Ok(()) => None,
                                Err(crate::StorageError::Constraint(message)) => Some(message),
                                Err(error) => return Err(error.into()),
                            };
                            facts.insert(plan.clone(), error);
                        }
                    }
                    let change = Design::edit_workspace_in(
                        conn,
                        &ctx,
                        &project,
                        expected_revision,
                        ops.clone(),
                        &facts,
                    )
                    .await?;
                    Self::apply_design_links_in(conn, &project, &ops).await?;
                    Ok(change)
                })
            })
            .await
            .map_err(CoreError::invalid_constraint)
    }
}
