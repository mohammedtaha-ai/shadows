//! Plan service MCP translation.
use super::{
    refusal::{Refusal, answer},
    server::Shadows,
};
use rmcp::{
    handler::server::{tool::Extension, wrapper::Parameters},
    model::CallToolResult,
    tool, tool_router,
};
use serde::Deserialize;
use serde_json::json;
use shadows_core::Grant;
use shadows_core::{DraftStart, PlanEdit, PlanOp, WorkflowId};

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct PlanArgs {
    /// Own project by default; a linked project's slug for a read only.
    #[serde(default)]
    project: Option<String>,
    /// The plan version; workflow_list lists each plan's latest version.
    #[serde(default)]
    #[schemars(with = "Option<String>")]
    workflow_id: Option<WorkflowId>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct PlanApproveArgs {
    #[schemars(with = "String")]
    workflow_id: WorkflowId,
    expected_revision: i64,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct TaskArgs {
    /// As for `workflow_get`.
    #[serde(default)]
    #[schemars(with = "Option<String>")]
    workflow_id: Option<WorkflowId>,
    /// The task's number: 4 for T4.
    number: u32,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct DraftStartArgs {
    /// The first version's title. Needed only when starting a new plan.
    #[serde(default)]
    title: Option<String>,
    /// The first version's goal. Needed only when starting a new plan.
    #[serde(default)]
    goal: Option<String>,
    /// A plan to start its next version, or none for a new plan.
    #[serde(default)]
    #[schemars(with = "Option<String>")]
    plan_id: Option<shadows_core::PlanId>,
    /// Why the plan changes, required for every version after the first.
    #[serde(default)]
    reason: Option<String>,
    /// External agents: the ref `draft_prepare` answered. The same ref always
    /// answers the same plan; a new plan takes a new ref.
    #[serde(default)]
    draft_ref: Option<String>,
    /// parts: the part ids this plan changes; a plan cannot be approved until
    /// it is tied to at least one part.
    #[serde(default)]
    #[schemars(with = "Vec<String>")]
    parts: Vec<shadows_core::PartId>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct WorkflowListArgs {
    /// Own project by default; a linked project's slug for a read only.
    #[serde(default)]
    project: Option<String>,
    /// Include archived plans alongside Active plans.
    #[serde(default)]
    archived: bool,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct PlanEditArgs {
    /// As for `workflow_get`.
    #[serde(default)]
    #[schemars(with = "Option<String>")]
    workflow_id: Option<WorkflowId>,
    /// The revision the edit was built on, as `workflow_get` answered it.
    expected_revision: i64,
    /// Applied together or not at all; the revision moves once.
    ops: Vec<PlanOp>,
    /// Leave it out: Shadows derives one, so a retry of the same edit
    /// answers the first result.
    #[serde(default)]
    command_id: Option<String>,
}

#[tool_router(router = plan_tool_router, vis = "pub(super)")]
impl Shadows {
    #[tool(
        description = "Approve a plan only at the person's request. Records Planner attribution."
    )]
    async fn plan_approve(
        &self,
        Extension(grant): Extension<Grant>,
        Parameters(args): Parameters<PlanApproveArgs>,
    ) -> CallToolResult {
        answer(
            self.core
                .plans()
                .approve_as_planner(&grant, &args.workflow_id, args.expected_revision)
                .await
                .map_err(Refusal::from),
        )
    }
    #[tool(
        description = "List plans: each one's latest version. Use project for a linked \
                       project's slug, archived to include archived plans."
    )]
    async fn workflow_list(
        &self,
        Extension(grant): Extension<Grant>,
        Parameters(args): Parameters<WorkflowListArgs>,
    ) -> CallToolResult {
        answer(
            self.core
                .plans()
                .list_for(&grant, args.archived, args.project.as_deref())
                .await
                .map_err(Refusal::from),
        )
    }

    #[tool(
        description = "Read a plan version: title, goal, tasks, links, revision, state, \
                       and what blocks approval. Use project for a linked project's slug."
    )]
    async fn workflow_get(
        &self,
        Extension(grant): Extension<Grant>,
        Parameters(args): Parameters<PlanArgs>,
    ) -> CallToolResult {
        answer(
            self.core
                .plans()
                .get_for(&grant, args.workflow_id.as_ref(), args.project.as_deref())
                .await
                .map_err(Refusal::from),
        )
    }

    #[tool(description = "Read one task of a plan version by its number.")]
    async fn task_get(
        &self,
        Extension(grant): Extension<Grant>,
        Parameters(args): Parameters<TaskArgs>,
    ) -> CallToolResult {
        answer(
            self.core
                .plans()
                .task_for(&grant, args.workflow_id.as_ref(), args.number)
                .await
                .map_err(Refusal::from),
        )
    }

    #[tool(
        description = "Get a draft_ref for draft_start. Call it once for each new plan you \
                       intend; a ref not used within an hour expires."
    )]
    async fn draft_prepare(&self, Extension(grant): Extension<Grant>) -> CallToolResult {
        let issued = self.core.plans().prepare_draft(&grant).await;
        answer(
            issued
                .map(|r| json!({ "draft_ref": r }))
                .map_err(Refusal::from),
        )
    }

    #[tool(
        description = "Start a plan version to edit. Give plan_id to continue that plan, \
                       or leave it out to create a new plan from a title and goal. \
                       A later version needs a reason; an existing Draft is returned unchanged. \
                       parts: the part ids this plan changes; a plan cannot be approved until \
                       it is tied to at least one part."
    )]
    async fn draft_start(
        &self,
        Extension(grant): Extension<Grant>,
        Parameters(args): Parameters<DraftStartArgs>,
    ) -> CallToolResult {
        let args = DraftStart {
            title: args.title,
            goal: args.goal,
            plan_id: args.plan_id,
            reason: args.reason,
            draft_ref: args.draft_ref,
            parts: args.parts,
        };
        answer(
            self.core
                .plans()
                .start_draft(&grant, args)
                .await
                .map(|started| json!(started))
                .map_err(Refusal::from),
        )
    }

    #[tool(
        description = "Edit a draft plan version at expected_revision with a list of \
                       operations: plan_put, task_add, task_update, task_remove, link_put, \
                       link_remove. Put changes made together in one call."
    )]
    async fn plan_edit(
        &self,
        Extension(grant): Extension<Grant>,
        Parameters(args): Parameters<PlanEditArgs>,
    ) -> CallToolResult {
        let args = PlanEdit {
            workflow_id: args.workflow_id,
            expected_revision: args.expected_revision,
            ops: args.ops,
            command_id: args.command_id,
        };
        answer(
            self.core
                .plans()
                .edit(&grant, args)
                .await
                .map(|outcome| json!(outcome))
                .map_err(Refusal::from),
        )
    }
}
