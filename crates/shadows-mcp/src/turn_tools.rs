//! Turn service MCP translation.
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
use shadows_core::{Grant, GrantKind, QuestionOption};
use shadows_core::{Place, PlanShow, WorkflowId};

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

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct PersonAskOption {
    /// The button's text, 1 to 80 characters.
    label: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct PersonAskArgs {
    /// What the person must decide, 1 to 500 characters.
    question: String,
    /// 2 to 5 options; the person clicks one.
    options: Vec<PersonAskOption>,
}

#[tool_router(router = turn_tool_router, vis = "pub(super)")]
impl Shadows {
    #[tool(
        description = "Show the person the plan, or one task of it, while they talk with you: \
                       inline in the conversation, side in a panel beside it, or page on its own \
                       page. Changes no plan."
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
                .turns()
                .show_plan(&grant, args)
                .await
                .map_err(Refusal::from),
        )
    }

    #[tool(
        description = "Ask the person to choose: a question with 2 to 5 options, drawn as buttons \
                       in the conversation. End your turn and wait: their click comes back as \
                       their next message, `Answer to \"<question>\": <label>`."
    )]
    async fn person_ask(
        &self,
        Extension(grant): Extension<Grant>,
        Parameters(args): Parameters<PersonAskArgs>,
    ) -> CallToolResult {
        let thread = match (grant.kind, &grant.thread_id) {
            (GrantKind::Thread, Some(thread)) => thread.clone(),
            _ => return answer::<()>(Err(Refusal::scope("only the Planner asks the person"))),
        };
        let options = args
            .options
            .into_iter()
            .map(|option| QuestionOption {
                label: option.label,
            })
            .collect();
        answer(
            self.core
                .turns()
                .ask(&thread, args.question, options)
                .await
                .map(|question_id| shadows_core::Asked { question_id })
                .map_err(Refusal::from),
        )
    }
}
