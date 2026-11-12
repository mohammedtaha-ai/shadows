//! Code service MCP translation.
use super::{
    refusal::{Refusal, answer, text},
    server::Shadows,
};
use rmcp::{
    handler::server::{tool::Extension, wrapper::Parameters},
    model::CallToolResult,
    tool, tool_router,
};
use serde::Deserialize;
use shadows_core::Grant;
use shadows_core::{Answer, Asker, IndexState, InventoryQuery};

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct NameArgs {
    /// The exact name: a function, method, type, trait, class, interface or constant.
    name: String,
    /// A linked project's slug, to ask only it.
    /// Leave it out to ask this project and every linked one.
    #[serde(default)]
    project: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct OutlineArgs {
    /// A file or folder, relative to the project's folder. "" for the whole project.
    path: String,
    /// As for where_is.
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
        description = "Where a name is defined: project, file, line, kind and signature. \
                       Never the code; open the file if you need it."
    )]
    async fn where_is(
        &self,
        Extension(grant): Extension<Grant>,
        Parameters(args): Parameters<NameArgs>,
    ) -> CallToolResult {
        let only = args.project.as_deref();
        let asked =
            self.core
                .codebase()
                .definitions(Asker::Grant(&grant.project_id), only, &args.name);
        text(asked.await.map(|a| lines(&a, false)).map_err(Refusal::from))
    }

    #[tool(description = "Where a name is used, matched by name only: \
                       two things with the same name are not told apart.")]
    async fn who_uses(
        &self,
        Extension(grant): Extension<Grant>,
        Parameters(args): Parameters<NameArgs>,
    ) -> CallToolResult {
        let only = args.project.as_deref();
        let asked =
            self.core
                .codebase()
                .references(Asker::Grant(&grant.project_id), only, &args.name);
        text(asked.await.map(|a| lines(&a, true)).map_err(Refusal::from))
    }

    #[tool(
        description = "The definitions in a file or folder: file, line, kind, name and signature."
    )]
    async fn outline(
        &self,
        Extension(grant): Extension<Grant>,
        Parameters(args): Parameters<OutlineArgs>,
    ) -> CallToolResult {
        let only = args.project.as_deref();
        let asked = self
            .core
            .codebase()
            .outline(Asker::Grant(&grant.project_id), only, &args.path);
        text(asked.await.map(|a| lines(&a, false)).map_err(Refusal::from))
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

fn lines(answer: &Answer, references: bool) -> String {
    let mut out = Vec::new();
    for h in &answer.hits {
        let at = format!("{}: {}:{} {} {}", h.project, h.path, h.line, h.kind, h.name);
        out.push(match (&h.signature, references) {
            (_, true) => format!("{at} (matched by name)"),
            (Some(signature), false) => format!("{at} — {signature}"),
            (None, false) => at,
        });
    }
    if answer.hits.is_empty() {
        out.push("nothing found".into());
    }
    if answer.more {
        out.push("more results: narrow the name or the path".into());
    }
    if !answer.suggestions.is_empty() {
        out.push(format!("did you mean: {}", answer.suggestions.join(", ")));
    }
    for s in &answer.status {
        let state = match &s.state {
            IndexState::Ready => "ready".to_string(),
            IndexState::Indexing { done, found } => format!("indexing {done}/{found}"),
            IndexState::Inactive => "inactive".into(),
            IndexState::NoDirectory => "not indexed: no directory".into(),
            IndexState::DirectoryMissing => "directory missing".into(),
        };
        let skipped: Vec<String> = s
            .skipped
            .iter()
            .map(|k| format!("{} {}", k.count, k.reason))
            .collect();
        if skipped.is_empty() {
            out.push(format!("{}: {state}", s.project));
        } else {
            out.push(format!(
                "{}: {state}; skipped {}",
                s.project,
                skipped.join(", ")
            ));
        }
    }
    out.join("\n")
}
