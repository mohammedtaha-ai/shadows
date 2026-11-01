//! Public extension library metadata contains no local paths.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, utoipa::ToSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum ExtensionKind {
    Plugin,
    Skill,
    Mcp,
}
impl ExtensionKind {
    pub(super) fn as_str(self) -> &'static str {
        match self {
            Self::Plugin => "plugin",
            Self::Skill => "skill",
            Self::Mcp => "mcp",
        }
    }
}
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct Item {
    pub id: String,
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub bundled: bool,
}
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct AgentItem {
    #[serde(flatten)]
    pub item: Item,
    pub enabled: bool,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub transport: Option<McpTransport>,
}
#[derive(Debug, Default, serde::Serialize, utoipa::ToSchema)]
pub struct ExtensionLibrary {
    pub plugins: Vec<Item>,
    pub skills: Vec<Item>,
    pub mcp: Vec<McpItem>,
}
#[derive(Debug, Default, serde::Serialize, utoipa::ToSchema)]
pub struct AgentExtensions {
    pub plugins: Vec<AgentItem>,
    pub skills: Vec<AgentItem>,
    pub mcp: Vec<AgentMcpItem>,
}

#[derive(Clone, serde::Deserialize, utoipa::ToSchema)]
pub struct McpValueInput {
    pub name: String,
    pub value: Option<String>,
}

#[derive(Clone, serde::Deserialize, utoipa::ToSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum McpTransportInput {
    Http {
        url: String,
        headers: Vec<McpValueInput>,
    },
    Stdio {
        command: String,
        args: Vec<String>,
        env: Vec<McpValueInput>,
    },
}

#[derive(Clone, serde::Deserialize, utoipa::ToSchema)]
pub struct McpInput {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    pub transport: McpTransportInput,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct McpValue {
    pub name: String,
    pub set: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum McpTransport {
    Http {
        url: String,
        headers: Vec<McpValue>,
    },
    Stdio {
        command: String,
        args: Vec<String>,
        env: Vec<McpValue>,
    },
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct McpItem {
    pub id: String,
    pub name: String,
    pub description: String,
    pub transport: McpTransport,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct AgentMcpItem {
    #[serde(flatten)]
    pub item: McpItem,
    pub enabled: bool,
}
