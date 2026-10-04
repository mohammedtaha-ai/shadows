//! One job: the `rmcp` handler — which tools a request's grant may see and
//! call (spec §13.6's table).
//!
//! The grant reaches a request through the extensions `rmcp` hands a handler:
//! `RequestContext::extensions` holds the request's `http::request::Parts`,
//! whose own `extensions` hold the `Grant` that `auth.rs` put there. A tool
//! outside the grant's list is unknown to that client: listing leaves it out,
//! and calling it answers `rmcp`'s own "tool not found".

use std::sync::Arc;

use axum::http::request::Parts;
use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::handler::server::tool::ToolCallContext;
use rmcp::model::{
    CacheScope, CallToolRequestParams, CallToolResponse, Implementation, ListToolsResult,
    PaginatedRequestParams, ProtocolVersion, ResultType, ServerCapabilities, ServerConfig, Tool,
};
use rmcp::service::RequestContext;
use rmcp::{ErrorData, RoleServer, ServerHandler};

use shadows_core::AppCore;
use shadows_core::{Grant, GrantKind};

/// The internal Planner's tools: plans belong to the project now. Only it
/// shows a plan; an external agent cannot move a person's screen.
const THREAD_TOOLS: [&str; 12] = [
    "agreement_list",
    "agreement_get",
    "agreement_start",
    "agreement_edit",
    "agreement_review",
    "workflow_list",
    "workflow_get",
    "task_get",
    "draft_start",
    "plan_edit",
    "plan_show",
    "workspace_get",
];

/// An external agent's tools, within its project: the plan tools, and the
/// code tools over the project and the projects it links to (§15.7).
const PROJECT_TOOLS: [&str; 15] = [
    "agreement_list",
    "agreement_get",
    "agreement_start",
    "agreement_edit",
    "agreement_review",
    "workflow_list",
    "workflow_get",
    "task_get",
    "draft_prepare",
    "draft_start",
    "plan_edit",
    "where_is",
    "who_uses",
    "outline",
    "workspace_get",
];

/// Every tool once, and each grant kind's share of them, built once per
/// daemon: a stateless server makes a handler for every request.
#[derive(Clone)]
pub(super) struct Tools(Arc<Routers>);

struct Routers {
    every: ToolRouter<Shadows>,
    thread: ToolRouter<Shadows>,
    project: ToolRouter<Shadows>,
}

impl Tools {
    pub(super) fn new() -> Self {
        let every = Shadows::tool_router()
            + Shadows::agreement_tool_router()
            + Shadows::workspace_tool_router();
        let only = |names: &[&str]| {
            let mut router = every.clone();
            for tool in every.list_all() {
                if !names.contains(&tool.name.as_ref()) {
                    router.remove_route(&tool.name);
                }
            }
            router
        };
        let (thread, project) = (only(&THREAD_TOOLS), only(&PROJECT_TOOLS));
        Self(Arc::new(Routers {
            every,
            thread,
            project,
        }))
    }

    fn of(&self, kind: GrantKind) -> &ToolRouter<Shadows> {
        match kind {
            GrantKind::Thread => &self.0.thread,
            GrantKind::Project => &self.0.project,
        }
    }
}

/// One request's handler: the state every tool reaches, and the tool sets.
#[derive(Clone)]
pub(super) struct Shadows {
    pub(super) core: Arc<AppCore>,
    tools: Tools,
}

impl Shadows {
    pub(super) fn new(core: Arc<AppCore>, tools: Tools) -> Self {
        Self { core, tools }
    }
}

/// The grant `auth.rs` admitted this request with. Every request reaching a
/// handler passed it, so its absence is the daemon's own defect.
fn grant_of(context: &RequestContext<RoleServer>) -> Result<Grant, ErrorData> {
    context
        .extensions
        .get::<Parts>()
        .and_then(|parts| parts.extensions.get::<Grant>())
        .cloned()
        .ok_or_else(|| {
            tracing::error!("mcp.request_without_grant");
            ErrorData::internal_error("the request carries no grant", None)
        })
}

impl ServerHandler for Shadows {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new("shadows", env!("CARGO_PKG_VERSION")))
    }

    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, ErrorData> {
        let grant = grant_of(&context)?;
        // The list differs by grant, so a cache may keep it for this client only.
        let hints = context
            .protocol_version()
            .is_some_and(|v| v >= ProtocolVersion::V_2026_07_28);
        Ok(ListToolsResult {
            result_type: Some(ResultType::COMPLETE),
            tools: self.tools.of(grant.kind).list_all(),
            meta: None,
            next_cursor: None,
            ttl_ms: hints.then_some(0),
            cache_scope: hints.then_some(CacheScope::Private),
        })
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        mut context: RequestContext<RoleServer>,
    ) -> Result<CallToolResponse, ErrorData> {
        let grant = grant_of(&context)?;
        let router = self.tools.of(grant.kind);
        // Put where a tool's `Extension<Grant>` argument reads it.
        context.extensions.insert(grant);
        router
            .call(ToolCallContext::new(self, request, context))
            .await
    }

    /// Any tool's definition, whatever the grant: `rmcp` reads it to check a
    /// call's parameters, never to list them.
    fn get_tool(&self, name: &str) -> Option<Tool> {
        self.tools.0.every.get(name).cloned()
    }
}
