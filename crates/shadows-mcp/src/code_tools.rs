//! Code service MCP translation.
use super::{
    refusal::{Refusal, text},
    server::Shadows,
};
use rmcp::{
    handler::server::{tool::Extension, wrapper::Parameters},
    model::CallToolResult,
    tool, tool_router,
};
use serde::Deserialize;
use shadows_core::Grant;
use shadows_core::{Answer, Asker, IndexState};

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
        let asked = self
            .core
            .code()
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
        let asked = self
            .core
            .code()
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
            .code()
            .outline(Asker::Grant(&grant.project_id), only, &args.path);
        text(asked.await.map(|a| lines(&a, false)).map_err(Refusal::from))
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
