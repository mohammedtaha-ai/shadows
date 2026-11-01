//! MCP configuration commands and input rules.
use crate::{
    Extensions, ExtensionsError, McpInput, McpItem, McpTransport, McpTransportInput, McpValue,
};
use core_db::CommandContext;
use core_providers::Providers;
use core_secrets::EnvWrite;
use std::collections::BTreeSet;

pub(super) struct PreparedMcp {
    pub item: McpItem,
    pub writes: Vec<EnvWrite>,
    pub names_without_values: Vec<String>,
    pub fingerprint: serde_json::Value,
}

impl Extensions {
    pub async fn add_mcp(
        &self,
        command_id: String,
        input: McpInput,
    ) -> Result<McpItem, ExtensionsError> {
        let _guard = self.extension_guard().await;
        let id = uuid::Uuid::new_v4().to_string();
        let prepared = self.prepare_mcp(&command_id, id, input)?;
        let command = CommandContext::user(
            command_id,
            "extension.mcp_add",
            prepared.fingerprint.clone(),
        );
        self.store
            .save_mcp(command, prepared, false, self.secrets.clone())
            .await
    }

    pub async fn update_mcp(
        &self,
        command_id: String,
        id: &str,
        input: McpInput,
    ) -> Result<McpItem, ExtensionsError> {
        let _guard = self.extension_guard().await;
        let prepared = self.prepare_mcp(&command_id, id.to_owned(), input)?;
        let command = CommandContext::user(
            command_id,
            "extension.mcp_update",
            serde_json::json!({"id":id,"input":prepared.fingerprint}),
        );
        self.store
            .save_mcp(command, prepared, true, self.secrets.clone())
            .await
    }

    fn prepare_mcp(
        &self,
        command_id: &str,
        id: String,
        input: McpInput,
    ) -> Result<PreparedMcp, ExtensionsError> {
        let name = input.name;
        if name == "shadows"
            || name.is_empty()
            || name.len() > 64
            || !name.as_bytes()[0].is_ascii_lowercase()
            || !name
                .bytes()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'_' || c == b'-')
        {
            return Err(ExtensionsError::Invalid("invalid MCP name".into()));
        }
        let description = input.description.unwrap_or_default().trim().to_owned();
        if description.len() > 4096 || description.contains('\0') {
            return Err(ExtensionsError::Invalid("invalid MCP description".into()));
        }
        let (base, values) = match input.transport {
            McpTransportInput::Http { url, headers } => {
                Providers::validate_url(&url)
                    .map_err(|_| ExtensionsError::Invalid("invalid MCP URL".into()))?;
                (serde_json::json!({"type":"http","url":url}), headers)
            }
            McpTransportInput::Stdio { command, args, env } => {
                if command.trim().is_empty()
                    || command.len() > 4096
                    || command.contains('\0')
                    || args.len() > 128
                    || args.iter().any(|s| s.len() > 4096 || s.contains('\0'))
                {
                    return Err(ExtensionsError::Invalid(
                        "invalid MCP command or arguments".into(),
                    ));
                }
                (
                    serde_json::json!({"type":"stdio","command":command,"args":args}),
                    env,
                )
            }
        };
        let mut values = values;
        values.sort_by(|a, b| a.name.cmp(&b.name));
        let mut seen = BTreeSet::new();
        let mut names_without_values = Vec::new();
        let mut writes = Vec::new();
        let mut keyed = Vec::new();
        let mut names = Vec::new();
        for pair in values {
            if !seen.insert(pair.name.clone()) {
                return Err(ExtensionsError::Invalid("duplicate MCP name".into()));
            }
            if let Some(value) = pair.value {
                let write = self.secrets.mcp_value_write(
                    format!("{command_id}:{}", pair.name),
                    &id,
                    &pair.name,
                    &value,
                )?;
                keyed.push(serde_json::json!({"name":pair.name,"mac":write.value_mac()}));
                writes.push(write);
            } else {
                // The removal constructor validates names even when no value was sent.
                self.secrets.mcp_value_removal(
                    format!("{command_id}:{}", pair.name),
                    &id,
                    &pair.name,
                )?;
                if core_secrets::Secrets::reserved_env_name(&pair.name) {
                    return Err(core_secrets::SecretsError::NameReserved(pair.name).into());
                }
                keyed.push(serde_json::json!({"name":pair.name,"mac":null}));
                names_without_values.push(pair.name.clone());
            }
            names.push(McpValue {
                name: pair.name,
                set: true,
            });
        }
        let transport = if base["type"] == "http" {
            McpTransport::Http {
                url: base["url"].as_str().unwrap().to_owned(),
                headers: names,
            }
        } else {
            McpTransport::Stdio {
                command: base["command"].as_str().unwrap().to_owned(),
                args: serde_json::from_value(base["args"].clone())?,
                env: names,
            }
        };
        let fingerprint = serde_json::json!({
            "name": name, "description": description, "transport": base, "values": keyed
        });
        Ok(PreparedMcp {
            item: McpItem {
                id,
                name,
                description,
                transport,
            },
            writes,
            names_without_values,
            fingerprint,
        })
    }
}

/// A resolved server for a trusted harness opening.
#[derive(Clone)]
pub enum McpServerConfig {
    Http {
        name: String,
        url: String,
        headers: Vec<(String, String)>,
    },
    Stdio {
        name: String,
        command: String,
        args: Vec<String>,
        env: Vec<(String, String)>,
    },
}

impl std::fmt::Debug for McpServerConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Http { name, url, .. } => f
                .debug_struct("Http")
                .field("name", name)
                .field("url", url)
                .finish_non_exhaustive(),
            Self::Stdio { name, command, .. } => f
                .debug_struct("Stdio")
                .field("name", name)
                .field("command", command)
                .finish_non_exhaustive(),
        }
    }
}

impl Extensions {
    pub async fn agent_mcp_servers(
        &self,
        slug: &str,
    ) -> Result<Vec<McpServerConfig>, ExtensionsError> {
        Ok(self.opening_snapshot(slug).await?.mcp)
    }

    pub async fn opening_snapshot(
        &self,
        slug: &str,
    ) -> Result<crate::AgentExtensionSnapshot, ExtensionsError> {
        let guard = self.extension_guard().await;
        let list = self.store.agent_library(slug).await?;
        let mcp = self.resolve_mcp_servers(&list).await?;
        Ok(crate::AgentExtensionSnapshot {
            library: list,
            mcp,
            _guard: guard,
        })
    }

    async fn resolve_mcp_servers(
        &self,
        list: &crate::AgentExtensions,
    ) -> Result<Vec<McpServerConfig>, ExtensionsError> {
        let mut out = Vec::new();
        for agent_item in list.mcp.iter().filter(|item| item.enabled) {
            let item = agent_item.item.clone();
            let values = self
                .secrets
                .resolve_env(&core_secrets::Secrets::mcp_scope(&item.id))
                .await?;
            let select = |names: Vec<McpValue>| -> Result<Vec<(String, String)>, ExtensionsError> {
                names
                    .into_iter()
                    .map(|entry| {
                        let value = values
                            .iter()
                            .find(|(name, _)| *name == entry.name)
                            .ok_or_else(|| ExtensionsError::Invalid("missing MCP value".into()))?;
                        Ok((entry.name, value.1.clone()))
                    })
                    .collect()
            };
            out.push(match item.transport {
                McpTransport::Http { url, headers } => McpServerConfig::Http {
                    name: item.name,
                    url,
                    headers: select(headers)?,
                },
                McpTransport::Stdio { command, args, env } => McpServerConfig::Stdio {
                    name: item.name,
                    command,
                    args,
                    env: select(env)?,
                },
            });
        }
        Ok(out)
    }
}
