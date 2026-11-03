//! Per-agent MCP tool rule commands.
use crate::{Extensions, ExtensionsError, ToolRuleEntry};
use core_db::CommandContext;

impl Extensions {
    pub async fn tool_rules(
        &self,
        slug: &str,
        id: &str,
    ) -> Result<Vec<ToolRuleEntry>, ExtensionsError> {
        self.store.tool_rules(slug, id).await
    }

    pub async fn set_tool_rules(
        &self,
        command_id: String,
        slug: &str,
        id: &str,
        mut rules: Vec<ToolRuleEntry>,
    ) -> Result<Vec<ToolRuleEntry>, ExtensionsError> {
        rules.sort_by(|a, b| a.tool.cmp(&b.tool));
        if rules.iter().any(|entry| !valid_tool(&entry.tool))
            || rules.windows(2).any(|pair| pair[0].tool == pair[1].tool)
        {
            return Err(ExtensionsError::Invalid(
                "invalid or duplicate tool name".into(),
            ));
        }
        let c = CommandContext::user(
            command_id,
            "extension.tool_rules",
            serde_json::json!({"slug":slug,"id":id,"rules":rules}),
        );
        let _guard = self.extension_guard().await;
        self.store.set_tool_rules(c, slug, id, rules).await
    }
}

fn valid_tool(tool: &str) -> bool {
    !tool.is_empty()
        && tool.len() <= 128
        && tool
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-' || b == b'.')
}
