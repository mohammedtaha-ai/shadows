//! Recoverable projection of the rule strings owned by Shadows.
use super::extension_home::{atomic, local};
use crate::{HarnessError, profile_home::Homes};
use core_extensions::{McpServerConfig, ToolRule};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{collections::BTreeSet, path::Path};

#[derive(Default, Serialize, Deserialize)]
struct RuleManifest {
    #[serde(default)]
    allow: BTreeSet<String>,
    #[serde(default)]
    deny: BTreeSet<String>,
    #[serde(default)]
    created_permissions: bool,
}

fn invalid() -> HarnessError {
    HarnessError::ProfileInvalid("cannot project MCP tool rules".into())
}

impl Homes {
    pub(super) fn write_tool_rules(
        home: &Path,
        servers: &[McpServerConfig],
    ) -> Result<(), HarnessError> {
        let manifest_path = home.join(".aylul-tool-rules.json");
        let settings_path = home.join("settings.json");
        local(&manifest_path)?;
        local(&settings_path)?;
        let prior = match std::fs::read(&manifest_path) {
            Ok(bytes) => serde_json::from_slice::<RuleManifest>(&bytes).map_err(|_| invalid())?,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => RuleManifest::default(),
            Err(_) => return Err(invalid()),
        };
        let mut desired = RuleManifest::default();
        for server in servers {
            let (name, rules) = match server {
                McpServerConfig::Http { name, rules, .. }
                | McpServerConfig::Stdio { name, rules, .. } => (name, rules),
            };
            for entry in rules {
                let target = match entry.rule {
                    ToolRule::Allow => &mut desired.allow,
                    ToolRule::Deny => &mut desired.deny,
                    ToolRule::Ask => continue,
                };
                target.insert(format!("mcp__{name}__{}", entry.tool.replace('.', "_")));
            }
        }
        if desired.allow.is_empty() && desired.deny.is_empty() && !manifest_path.exists() {
            return Ok(());
        }
        let mut settings = Self::check(home)?;
        let original = settings.clone();
        let had_permissions = settings.get("permissions").is_some();
        let permissions = settings
            .as_object_mut()
            .ok_or_else(invalid)?
            .entry("permissions")
            .or_insert_with(|| json!({}))
            .as_object_mut()
            .ok_or_else(invalid)?;
        let mut current = RuleManifest {
            created_permissions: prior.created_permissions || !had_permissions,
            ..RuleManifest::default()
        };
        for (key, old, wanted, owned) in [
            ("allow", &prior.allow, &desired.allow, &mut current.allow),
            ("deny", &prior.deny, &desired.deny, &mut current.deny),
        ] {
            let mut values = match permissions.get(key) {
                Some(Value::Array(values)) if values.iter().all(Value::is_string) => values.clone(),
                Some(_) => return Err(invalid()),
                None => vec![],
            };
            values.retain(|v| !old.contains(v.as_str().unwrap_or_default()));
            for rule in wanted {
                if !values.iter().any(|v| v.as_str() == Some(rule)) {
                    values.push(rule.clone().into());
                    owned.insert(rule.clone());
                }
            }
            values.sort_by(|a, b| a.as_str().cmp(&b.as_str()));
            values.dedup();
            if !values.is_empty() {
                permissions.insert(key.into(), values.into());
            } else if !old.is_empty() {
                permissions.remove(key);
            }
        }
        if permissions.is_empty() && current.created_permissions {
            settings
                .as_object_mut()
                .ok_or_else(invalid)?
                .remove("permissions");
            current.created_permissions = false;
        }
        // The write-ahead ownership union covers both sides of an interrupted update.
        let pending = RuleManifest {
            allow: prior.allow.union(&current.allow).cloned().collect(),
            deny: prior.deny.union(&current.deny).cloned().collect(),
            created_permissions: prior.created_permissions || current.created_permissions,
        };
        atomic(
            &manifest_path,
            &serde_json::to_vec(&pending).map_err(|_| invalid())?,
        )?;
        if settings != original {
            atomic(
                &settings_path,
                &serde_json::to_vec_pretty(&settings).map_err(|_| invalid())?,
            )?;
        }
        atomic(
            &manifest_path,
            &serde_json::to_vec(&current).map_err(|_| invalid())?,
        )?;
        Ok(())
    }
}
