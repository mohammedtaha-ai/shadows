//! Project discovery for a connection and its per-call routing argument.

use std::sync::Arc;

use rmcp::{
    handler::server::tool::Extension,
    model::{CallToolRequestParams, CallToolResult, Tool},
    tool, tool_router,
};
use serde_json::{Value, json};
use shadows_core::{CoreError, ExternalConnection, ProjectId};

use super::{
    refusal::{Refusal, answer},
    server::Shadows,
};

pub(super) const PROJECT_LIST: &str = "project_list";

#[tool_router(router = external_tool_router, vis = "pub(super)")]
impl Shadows {
    #[tool(
        description = "List the live projects this connection may access. Pass a returned id as project_id to the other tools."
    )]
    async fn project_list(
        &self,
        Extension(connection): Extension<ExternalConnection>,
    ) -> CallToolResult {
        answer(self.core.projects().list().await.map(|projects| {
            projects.into_iter()
                .filter(|project| connection.project_ids.contains(&project.id))
                .map(|project| json!({ "id": project.id, "name": project.name, "slug": project.slug }))
                .collect::<Vec<_>>()
        }).map_err(|error| Refusal::from(CoreError::from(error))))
    }
}

pub(super) fn external_schema(mut tool: Tool) -> Tool {
    if tool.name != PROJECT_LIST {
        match tool.name.as_ref() {
            "workflow_list" => tool.description = Some("List the latest version of each plan in the selected project. Set archived to include archived plans.".into()),
            "workflow_get" => tool.description = Some("Read a plan version in the selected project, including its goal, tasks, links, revision, state and approval blockers.".into()),
            _ => {}
        }
        let schema = Arc::make_mut(&mut tool.input_schema);
        let properties = schema.entry("properties").or_insert_with(|| json!({}));
        if let Some(properties) = properties.as_object_mut() {
            properties.remove("project");
            properties.insert(
                "project_id".into(),
                json!({
                    "type": "string",
                    "description": "The project id returned by project_list."
                }),
            );
        }
        let required = schema.entry("required").or_insert_with(|| json!([]));
        if let Some(required) = required.as_array_mut() {
            required.push(json!("project_id"));
        }
    }
    tool
}

pub(super) fn take_project(request: &mut CallToolRequestParams) -> Result<ProjectId, Refusal> {
    let args = request.arguments.get_or_insert_with(Default::default);
    if args.remove("project").is_some_and(|value| !value.is_null()) {
        return Err(Refusal::scope(
            "name the target only by project_id, as project_list returns it",
        ));
    }
    match args.remove("project_id") {
        Some(Value::String(id)) if !id.is_empty() => Ok(ProjectId::from_recorded(id)),
        _ => Err(Refusal::scope("project_id from project_list is required")),
    }
}
