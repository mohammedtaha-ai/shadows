//! One job: each MCP tool's arguments, and the one `Plans` call it makes
//! (spec §13.5–§13.6, §14.5).
//!
//! What a grant reaches, who writes under it, and which command id a call
//! carries are `Plans`' rules, not a tool's. `draft_start` and `plan_edit`
//! answer their outcome through `json!`, as they did before `Plans`, so the
//! text a client reads keeps its key order.

use rmcp::handler::server::tool::Extension;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use rmcp::{tool, tool_router};
use serde::Deserialize;
use serde_json::json;

use super::refusal::{Refusal, answer};
use super::server::Shadows;

use shadows_core::Grant;
use shadows_core::{DraftStart, Place, PlanEdit, PlanOp, PlanShow, WorkflowId};

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct PlanArgs {
    /// The plan version. The Planner leaves it out for its conversation's
    /// latest version, or names an older version of the same conversation
    /// when reading it. An external agent must name one.
    #[serde(default)]
    #[schemars(with = "Option<String>")]
    workflow_id: Option<WorkflowId>,
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
    /// The first version's title. Needed only when there is no plan yet.
    #[serde(default)]
    title: Option<String>,
    /// The first version's goal. Needed only when there is no plan yet.
    #[serde(default)]
    goal: Option<String>,
    /// External agents: an approved plan in the project, to start its next
    /// version from. Without it, a new conversation and plan are created.
    /// The Planner may leave it out; a version it names must be its
    /// conversation's latest, the one draft_start starts from anyway.
    #[serde(default)]
    #[schemars(with = "Option<String>")]
    from_workflow_id: Option<WorkflowId>,
    /// External agents: the ref `draft_prepare` answered. The same ref always
    /// answers the same plan; a new plan takes a new ref.
    #[serde(default)]
    draft_ref: Option<String>,
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

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct PlanShowArgs {
    /// As for `workflow_get`.
    #[serde(default)]
    #[schemars(with = "Option<String>")]
    workflow_id: Option<WorkflowId>,
    /// A task to show, by its number: 4 for T4. Leave it out to show the plan.
    #[serde(default)]
    task_number: Option<u32>,
    /// `inline` as a card in the conversation, `side` in a panel beside it,
    /// `page` on the plan's own page. Every place also leaves a card.
    place: Place,
}

#[tool_router(vis = "pub(super)")]
impl Shadows {
    #[tool(description = "List the plans in this project: each one's latest version.")]
    async fn workflow_list(&self, Extension(grant): Extension<Grant>) -> CallToolResult {
        answer(
            self.core
                .plans()
                .list_for(&grant)
                .await
                .map_err(Refusal::from),
        )
    }

    #[tool(
        description = "Read a plan version: title, goal, tasks, links, revision, state, and what blocks approval."
    )]
    async fn workflow_get(
        &self,
        Extension(grant): Extension<Grant>,
        Parameters(args): Parameters<PlanArgs>,
    ) -> CallToolResult {
        answer(
            self.core
                .plans()
                .get_for(&grant, args.workflow_id.as_ref())
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
        description = "Get a draft_ref for draft_start. Call it once for each new plan you intend; a ref not used within an hour expires."
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
        description = "Start a plan version to edit. With no plan yet, creates version 1 from a title and a goal; after an approved version, creates the next version as its copy; with a draft already there, answers it."
    )]
    async fn draft_start(
        &self,
        Extension(grant): Extension<Grant>,
        Parameters(args): Parameters<DraftStartArgs>,
    ) -> CallToolResult {
        let args = DraftStart {
            title: args.title,
            goal: args.goal,
            from_workflow_id: args.from_workflow_id,
            draft_ref: args.draft_ref,
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
        description = "Edit a draft plan version at expected_revision with a list of operations: plan_put, task_add, task_update, task_remove, link_put, link_remove. Put changes made together in one call."
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

    #[tool(
        description = "Show the person the plan, or one task of it, while they talk with you: inline in the conversation, side in a panel beside it, or page on its own page. Changes no plan."
    )]
    async fn plan_show(
        &self,
        Extension(grant): Extension<Grant>,
        Parameters(args): Parameters<PlanShowArgs>,
    ) -> CallToolResult {
        let args = PlanShow {
            workflow_id: args.workflow_id,
            task_number: args.task_number,
            place: args.place,
        };
        answer(
            self.core
                .plans()
                .show(&grant, args)
                .await
                .map_err(Refusal::from),
        )
    }
}
