//! Bounded MCP discovery without exposing transport errors or resolved values.
use crate::{AgentTools, Extensions, ExtensionsError, McpServerConfig, McpTool};
use rmcp::{
    ServiceExt,
    transport::{
        StreamableHttpClientTransport, streamable_http_client::StreamableHttpClientTransportConfig,
    },
};
use std::{collections::HashMap, time::Duration};

const REFUSED: &str = "the server refused the connection";
const PROTOCOL: &str = "the server's answer was not MCP";

struct ListingChild(shadows_process::ProcessHandle);
impl Drop for ListingChild {
    fn drop(&mut self) {
        let _ = self.0.terminate_tree();
    }
}

impl Extensions {
    pub async fn agent_tools(&self, slug: &str, id: &str) -> Result<AgentTools, ExtensionsError> {
        let guard = self.extension_guard().await;
        let mut list = self.store.agent_library(slug).await?;
        if !list.mcp.iter().any(|i| i.item.id == id) {
            return Err(ExtensionsError::NotFound);
        }
        for item in &mut list.mcp {
            item.enabled = item.item.id == id;
        }
        let rules = self.store.tool_rules(slug, id).await?;
        let servers = self.resolve_mcp_servers(slug, &list).await?;
        drop(guard);
        let server = servers
            .into_iter()
            .next()
            .ok_or(ExtensionsError::NotFound)?;
        let entries = match &server {
            McpServerConfig::Http { headers, .. } => headers,
            McpServerConfig::Stdio { env, .. } => env,
        };
        let mut values: Vec<_> = entries
            .iter()
            .map(|(_, value)| value.clone())
            .filter(|value| !value.is_empty())
            .collect();
        values.sort_by_key(|value| std::cmp::Reverse(value.len()));
        values.dedup();
        let result = tokio::time::timeout(Duration::from_secs(10), list_tools(server)).await;
        let (tools, list_error) = match result {
            Ok(Ok(mut tools)) => {
                if tools
                    .iter()
                    .any(|tool| values.iter().any(|value| tool.name.contains(value)))
                {
                    (vec![], Some(PROTOCOL.into()))
                } else {
                    for tool in &mut tools {
                        for value in &values {
                            tool.description = tool.description.replace(value, "[redacted]");
                        }
                    }
                    (tools, None)
                }
            }
            Ok(Err(message)) => (vec![], Some(message.into())),
            Err(_) => (
                vec![],
                Some("the server did not answer in 10 seconds".into()),
            ),
        };
        Ok(AgentTools {
            tools,
            list_error,
            rules,
        })
    }
}

async fn list_tools(server: McpServerConfig) -> Result<Vec<McpTool>, &'static str> {
    let mut child = None;
    let running = match server {
        McpServerConfig::Http { url, headers, .. } => {
            let mut values = HashMap::new();
            for (name, value) in headers {
                let name = reqwest::header::HeaderName::from_bytes(name.as_bytes())
                    .map_err(|_| REFUSED)?;
                let mut value =
                    reqwest::header::HeaderValue::from_str(&value).map_err(|_| REFUSED)?;
                value.set_sensitive(true);
                values.insert(name, value);
            }
            let client = reqwest::Client::builder()
                .redirect(reqwest::redirect::Policy::none())
                .build()
                .map_err(|_| REFUSED)?;
            let config = StreamableHttpClientTransportConfig::with_uri(url).custom_headers(values);
            ().serve(StreamableHttpClientTransport::with_client(client, config))
                .await
                .map_err(|_| REFUSED)?
        }
        McpServerConfig::Stdio {
            command, args, env, ..
        } => {
            let mut environment: HashMap<_, _> = std::env::vars().collect();
            environment.extend(env);
            let handle = shadows_process::spawn(shadows_process::ProcessSpec {
                executable: command.into(),
                args,
                cwd: std::env::current_dir().map_err(|_| REFUSED)?,
                env: environment.into_iter().collect(),
                capture_stdout: true,
                pipe_stdin: true,
            })
            .map_err(|_| REFUSED)?;
            child = Some(ListingChild(handle));
            let handle = &mut child.as_mut().ok_or(REFUSED)?.0;
            let (stdin, stdout, mut stderr) = handle.take_stdio().ok_or(REFUSED)?;
            tokio::spawn(async move {
                let _ = tokio::io::copy(&mut stderr, &mut tokio::io::sink()).await;
            });
            ().serve((stdout, stdin)).await.map_err(|_| PROTOCOL)?
        }
    };
    let result = running.list_all_tools().await.map_err(|_| PROTOCOL);
    let _ = running.cancel().await;
    if let Some(mut child) = child {
        child.0.terminate_tree().map_err(|_| REFUSED)?;
        child.0.wait().await.map_err(|_| REFUSED)?;
    }
    let mut tools: Vec<_> = result?
        .into_iter()
        .map(|tool| McpTool {
            name: tool.name.into_owned(),
            description: tool.description.map(|v| v.into_owned()).unwrap_or_default(),
        })
        .collect();
    tools.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(tools)
}
