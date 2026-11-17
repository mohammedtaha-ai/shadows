//! Design's project decision translation for the Planner.
use super::{
    refusal::{Refusal, answer},
    server::Shadows,
};
use rmcp::{
    handler::server::{tool::Extension, wrapper::Parameters},
    model::CallToolResult,
    tool, tool_router,
};
use shadows_core::{DecisionInput, Grant};

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub(super) struct DecisionProposeArgs {
    /// Unique per proposal; repeating it with the same input returns the same revision.
    command_id: String,
    /// The decision to revise, such as D1. Leave it out for a new decision.
    id: Option<String>,
    /// With `id`: the version of its latest revision, which this one follows.
    expected_version: Option<i64>,
    /// 1 to 200 characters.
    title: String,
    /// What is decided, 1 to 4,000 characters.
    decision: String,
    /// Why, 1 to 4,000 characters.
    rationale: String,
    /// Where it came from (a document, a conversation), up to 1,000 characters.
    #[serde(default)]
    source: String,
}

#[tool_router(router = decision_tool_router, vis = "pub(super)")]
impl Shadows {
    #[tool(
        description = "Propose a project decision (a library, a data shape, a layout rule) or \
            a new revision of one. It appears as a card the person accepts; you never accept it. \
            Pin accepted revisions in a task contract's `decisions`."
    )]
    async fn decision_propose(
        &self,
        Extension(grant): Extension<Grant>,
        Parameters(args): Parameters<DecisionProposeArgs>,
    ) -> CallToolResult {
        let input = DecisionInput {
            id: args.id,
            expected_version: args.expected_version,
            title: args.title,
            decision: args.decision,
            rationale: args.rationale,
            source: args.source,
        };
        answer(
            self.core
                .design()
                .propose_decision(&grant, args.command_id, input)
                .await
                .map_err(Refusal::from),
        )
    }

    #[tool(
        description = "List this project's decisions: every revision, by id then version, \
            with its state (proposed, accepted, superseded). Read only."
    )]
    async fn decision_list(&self, Extension(grant): Extension<Grant>) -> CallToolResult {
        answer(
            self.core
                .design()
                .decisions(&grant.project_id)
                .await
                .map_err(Refusal::from),
        )
    }
}
