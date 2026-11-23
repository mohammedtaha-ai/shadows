//! CodeRecord's translation for the Planner: where a topic is in the project.
use super::{
    refusal::{Refusal, answer},
    server::Shadows,
};
use rmcp::{
    handler::server::{tool::Extension, wrapper::Parameters},
    model::CallToolResult,
    tool, tool_router,
};
use shadows_core::{CodeRecord, Grant};

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub(super) struct CodeWhereArgs {
    /// One short thing, one to three words: a name the person used (login), a path
    /// (backend/auth), a symbol, or an element id or name. Never a sentence: call again for
    /// each thing.
    topic: String,
}

/// The tool's description. Its budget is CodeRecord's, read from its constants, so the
/// text cannot drift from what `code_where` spends.
fn code_where_description() -> String {
    format!(
        "Ask where something is in this project before you name a file, a table or a screen. \
         Ask one short thing per call, one to three words: a name the person used (login), a \
         path (backend/auth), a symbol, or an element id or name. Make several calls rather \
         than one with a sentence: every word of a topic must meet in one file or one symbol, \
         so an empty answer to a long topic is not \"nothing there\"; ask again with each \
         thing alone. Shadows answers from its record first: each file with the plans and \
         tasks that made it, its symbols and their contracts, the elements and their states. \
         Then the language servers confirm: each symbol's line now and who uses it, by part \
         and layer. Within {asks} asks and {secs} seconds; what they did not reach is marked \
         unchecked. Files no contract names are listed as not in the record. Ask it before \
         you read a file yourself; read only what it does not answer.",
        asks = CodeRecord::WHERE_MAX_ASKS,
        secs = CodeRecord::WHERE_LIVE_SECS,
    )
}

#[tool_router(router = record_tool_router, vis = "pub(super)")]
impl Shadows {
    #[tool(description = code_where_description())]
    async fn code_where(
        &self,
        Extension(grant): Extension<Grant>,
        Parameters(args): Parameters<CodeWhereArgs>,
    ) -> CallToolResult {
        answer(
            self.core
                .code_record()
                .code_where(&grant.project_id, &args.topic)
                .await
                .map_err(Refusal::from),
        )
    }
}
