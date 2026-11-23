//! Design's project element translation for the Planner.
use super::{
    refusal::{Refusal, answer},
    server::Shadows,
};
use rmcp::{
    handler::server::{tool::Extension, wrapper::Parameters},
    model::CallToolResult,
    tool, tool_router,
};
use shadows_core::{ElementContent, ElementDelete, ElementInput, Grant, OutcomeId, PartId};

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub(super) struct ElementPutArgs {
    command_id: String,
    id: Option<String>,
    expected_revision: Option<i64>,
    #[schemars(with = "String")]
    part_id: PartId,
    #[schemars(with = "Vec<String>")]
    outcomes: Vec<OutcomeId>,
    content: ElementContent,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub(super) struct ElementDeleteArgs {
    command_id: String,
    id: String,
    expected_revision: i64,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub(super) struct ElementsArgs {
    id: Option<String>,
}

#[tool_router(router = element_tool_router, vis = "pub(super)")]
impl Shadows {
    #[tool(
        description = "Write a table or a screen of the project's design, or a new revision of \
            one: a table in a database part, a screen in a frontend part, serving at least one \
            roadmap outcome. Leave id out for a new element; with id, give the expected_revision \
            of its latest revision. Pin the revision a task builds in its contract's elements."
    )]
    async fn design_element_put(
        &self,
        Extension(grant): Extension<Grant>,
        Parameters(args): Parameters<ElementPutArgs>,
    ) -> CallToolResult {
        let input = ElementInput {
            id: args.id,
            expected_revision: args.expected_revision,
            part_id: args.part_id,
            outcomes: args.outcomes,
            content: args.content,
        };
        answer(
            self.core
                .design()
                .put_element_for(&grant, args.command_id, input)
                .await
                .map_err(Refusal::from),
        )
    }

    #[tool(
        description = "Mark a table or a screen as deleted in the project's design. This writes \
            its final revision for a task with action delete. Refused while another element \
            refers to it."
    )]
    async fn design_element_delete(
        &self,
        Extension(grant): Extension<Grant>,
        Parameters(args): Parameters<ElementDeleteArgs>,
    ) -> CallToolResult {
        let input = ElementDelete {
            id: args.id,
            expected_revision: args.expected_revision,
        };
        answer(
            self.core
                .design()
                .delete_element_for(&grant, args.command_id, input)
                .await
                .map_err(Refusal::from),
        )
    }

    #[tool(
        description = "List this project's tables and screens at their latest revision, or every \
            revision of one by id. Read only."
    )]
    async fn design_elements(
        &self,
        Extension(grant): Extension<Grant>,
        Parameters(args): Parameters<ElementsArgs>,
    ) -> CallToolResult {
        let design = self.core.design();
        let result = match args.id {
            Some(id) => design.element_revisions(&grant.project_id, &id).await,
            None => design.elements(&grant.project_id).await,
        };
        answer(result.map_err(Refusal::from))
    }
}
