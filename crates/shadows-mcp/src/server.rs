//! One job: the `rmcp` handler — which tools a request's grant may see and
//! call.
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
use shadows_core::GrantKind;

use super::{
    auth::Credential,
    external_tools::{PROJECT_LIST, external_schema, take_project},
    refusal::{Refusal, answer},
};

/// Every tool once, and each grant kind's share of them, built once per
/// daemon: a stateless server makes a handler for every request.
#[derive(Clone)]
pub(super) struct Tools(Arc<Routers>);

struct Routers {
    every: ToolRouter<Shadows>,
    thread: ToolRouter<Shadows>,
    project: ToolRouter<Shadows>,
    external: ToolRouter<Shadows>,
    executor: ToolRouter<Shadows>,
}

impl Tools {
    pub(super) fn new() -> Self {
        let every = Shadows::plan_tool_router()
            + Shadows::code_tool_router()
            + Shadows::turn_tool_router()
            + Shadows::agreement_tool_router()
            + Shadows::workspace_tool_router()
            + Shadows::decision_tool_router()
            + Shadows::element_tool_router()
            + Shadows::state_tool_router()
            + Shadows::record_tool_router()
            + Shadows::run_tool_router()
            + Shadows::external_tool_router();
        let only = |names: &[&str]| {
            let mut router = every.clone();
            for tool in every.list_all() {
                if !names.contains(&tool.name.as_ref()) {
                    router.remove_route(&tool.name);
                }
            }
            router
        };
        let (thread, project, executor) = (
            only(shadows_core::Grants::tools_for(GrantKind::Thread)),
            only(shadows_core::Grants::tools_for(GrantKind::Project)),
            only(shadows_core::Grants::tools_for(GrantKind::Executor)),
        );
        let external = project.clone() + Shadows::external_tool_router();
        Self(Arc::new(Routers {
            every,
            thread,
            project,
            external,
            executor,
        }))
    }

    fn of(&self, kind: GrantKind) -> &ToolRouter<Shadows> {
        match kind {
            GrantKind::Thread => &self.0.thread,
            GrantKind::Project => &self.0.project,
            GrantKind::Executor => &self.0.executor,
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
fn grant_of(context: &RequestContext<RoleServer>) -> Result<Credential, ErrorData> {
    context
        .extensions
        .get::<Parts>()
        .and_then(|parts| parts.extensions.get::<Credential>())
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
            tools: match grant {
                Credential::Scoped(grant) => self.tools.of(grant.kind).list_all(),
                Credential::External(_) => self
                    .tools
                    .0
                    .external
                    .list_all()
                    .into_iter()
                    .map(external_schema)
                    .collect(),
            },
            meta: None,
            next_cursor: None,
            ttl_ms: hints.then_some(0),
            cache_scope: hints.then_some(CacheScope::Private),
        })
    }

    async fn call_tool(
        &self,
        mut request: CallToolRequestParams,
        mut context: RequestContext<RoleServer>,
    ) -> Result<CallToolResponse, ErrorData> {
        let grant = grant_of(&context)?;
        let router = match grant {
            Credential::Scoped(grant) => {
                let kind = grant.kind;
                context.extensions.insert(grant);
                self.tools.of(kind)
            }
            Credential::External(connection) => {
                let router = &self.tools.0.external;
                if request.name == PROJECT_LIST {
                    context.extensions.insert(connection);
                } else if router.get(&request.name).is_some() {
                    let project = match take_project(&mut request) {
                        Ok(project) => project,
                        Err(error) => {
                            return Ok(CallToolResponse::Complete(answer::<()>(Err(error))));
                        }
                    };
                    let grant = match self
                        .core
                        .grants()
                        .external_project(&connection.id, &project)
                        .await
                    {
                        Ok(grant) => grant,
                        Err(error) => {
                            return Ok(CallToolResponse::Complete(answer::<()>(Err(
                                Refusal::from(shadows_core::CoreError::from(error)),
                            ))));
                        }
                    };
                    context.extensions.insert(grant);
                }
                router
            }
        };
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
