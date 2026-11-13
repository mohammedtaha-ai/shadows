//! Code service MCP translation.
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
use shadows_core::Grant;
use shadows_core::{Asker, CodebaseError, InventoryQuery, LspOperation, LspQuestion};

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct LspArgs {
    /// goToDefinition, findReferences, hover, documentSymbol, workspaceSymbol,
    /// goToImplementation, incomingCalls or outgoingCalls.
    operation: String,
    /// The file, relative to the project's folder. Every operation but
    /// workspaceSymbol needs it.
    #[serde(default)]
    file: Option<String>,
    /// One-based, as an editor shows it. documentSymbol and workspaceSymbol take none.
    #[serde(default)]
    line: Option<u32>,
    /// One-based, as an editor shows it.
    #[serde(default)]
    character: Option<u32>,
    /// What workspaceSymbol looks for.
    #[serde(default)]
    query: Option<String>,
    /// A linked project's slug, to ask its server. Leave it out for this project.
    #[serde(default)]
    project: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct FilesArgs {
    /// A folder relative to the project's folder. Leave it out for the whole project.
    #[serde(default)]
    prefix: Option<String>,
    /// The `next` value of the page before this one.
    #[serde(default)]
    after: Option<String>,
    /// How many entries, 1 to 200. Leave it out for 100.
    #[serde(default)]
    limit: Option<u16>,
}

#[tool_router(router = code_tool_router, vis = "pub(super)")]
impl Shadows {
    #[tool(
        description = "Ask the project's language server: goToDefinition, findReferences, \
                       hover, documentSymbol, workspaceSymbol, goToImplementation, \
                       incomingCalls, outgoingCalls. Lines and characters are one-based, as \
                       an editor shows them."
    )]
    async fn code_lsp(
        &self,
        Extension(grant): Extension<Grant>,
        Parameters(args): Parameters<LspArgs>,
    ) -> CallToolResult {
        let operation: Result<LspOperation, _> =
            serde_json::from_value(serde_json::Value::String(args.operation.clone()));
        let Ok(operation) = operation else {
            let refused = CodebaseError::InvalidCommand(format!(
                "{} is not an operation code_lsp knows",
                args.operation
            ));
            return answer::<()>(Err(Refusal::from(refused)));
        };
        let question = LspQuestion {
            operation,
            file: args.file,
            line: args.line,
            character: args.character,
            query: args.query,
            project: args.project,
        };
        let asked = self
            .core
            .codebase()
            .lsp(Asker::Grant(&grant.project_id), question);
        answer(asked.await.map_err(Refusal::from))
    }

    #[tool(
        description = "The files the project's folder really holds, page by page: files, \
                       folders, and what is left out with its reason (dependencies, ignored, \
                       secret, link). Give prefix for one folder, and after with a page's next \
                       for the page that follows. INVENTORY_STALE means the folder changed: \
                       start again without after. A page's state: ready is a complete \
                       listing; incomplete means some entries could not be read; missing \
                       means the project has no folder or it is not there; indexing means \
                       the listing is not complete yet: ask again, never conclude the folder \
                       is empty."
    )]
    async fn project_files(
        &self,
        Extension(grant): Extension<Grant>,
        Parameters(args): Parameters<FilesArgs>,
    ) -> CallToolResult {
        let default = InventoryQuery::default();
        let query = InventoryQuery {
            prefix: args.prefix.unwrap_or(default.prefix),
            after: args.after,
            limit: args.limit.unwrap_or(default.limit),
        };
        let asked = self
            .core
            .codebase()
            .inventory(Asker::Grant(&grant.project_id), query);
        answer(asked.await.map_err(Refusal::from))
    }

    #[tool(
        description = "Read the project's folder again now, and answer the first page of \
                       its files. Use it when files were added or removed."
    )]
    async fn project_refresh(&self, Extension(grant): Extension<Grant>) -> CallToolResult {
        let asked = self
            .core
            .codebase()
            .refresh_inventory(Asker::Grant(&grant.project_id));
        answer(asked.await.map_err(Refusal::from))
    }
}
