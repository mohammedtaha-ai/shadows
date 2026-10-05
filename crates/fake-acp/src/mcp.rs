//! Calling the session's MCP server.

use rmcp::ServiceExt;
use rmcp::model::CallToolRequestParams;
use rmcp::transport::StreamableHttpClientTransport;
use rmcp::transport::streamable_http_client::StreamableHttpClientTransportConfig;
use serde_json::{Value, json};

use crate::session::Setup;

/// `mcp <tool> <json args>`: one call on the session's MCP server with its
/// bearer, answering the result's text.
pub(crate) async fn call_mcp(setup: &Setup, line: &str) -> String {
    let mut words = line.splitn(3, ' ').skip(1);
    let tool = words.next().unwrap_or_default().to_string();
    let args: Value = serde_json::from_str(words.next().unwrap_or("{}")).unwrap_or(json!({}));
    let (Some(url), Some(bearer)) = (&setup.url, &setup.bearer) else {
        return "mcp: no server".into();
    };
    let config =
        StreamableHttpClientTransportConfig::with_uri(url.clone()).auth_header(bearer.clone());
    let client = match ().serve(StreamableHttpClientTransport::from_config(config)).await {
        Ok(client) => client,
        Err(e) => return format!("mcp: {e}"),
    };
    let params = CallToolRequestParams::new(tool)
        .with_arguments(args.as_object().cloned().unwrap_or_default());
    let text = match client.call_tool(params).await {
        Ok(r) => r
            .content
            .first()
            .and_then(|c| c.as_text())
            .map(|t| t.text.clone())
            .unwrap_or_default(),
        Err(e) => format!("mcp: {e}"),
    };
    let _ = client.cancel().await;
    text
}
