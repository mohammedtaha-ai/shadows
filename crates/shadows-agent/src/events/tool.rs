//! Tool activity reports from the harness.
use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::ToSchema;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ToolFacts {
    pub kind: Option<CallKind>,
    pub locations: Option<Vec<ToolLocation>>,
    pub input: Option<Value>,
    pub diffs: Option<Vec<ToolDiff>>,
    pub texts: Option<Vec<String>>,
    pub output: Option<Value>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ToolLocation {
    pub path: String,
    pub line: Option<u32>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ToolDiff {
    pub path: String,
    pub old_text: Option<String>,
    pub new_text: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum CallKind {
    Read,
    Edit,
    Delete,
    Move,
    Search,
    Execute,
    Think,
    Fetch,
    SwitchMode,
    Other,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum CallStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
    Stopped,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum DiffOp {
    Context,
    Add,
    Remove,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct DiffLine {
    pub op: DiffOp,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct DiffHunk {
    pub path: String,
    pub old_start: Option<u32>,
    pub new_start: Option<u32>,
    pub lines: Vec<DiffLine>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct ToolCallCard {
    pub id: String,
    pub tool: Option<String>,
    pub kind: CallKind,
    pub status: CallStatus,
    pub target: Option<String>,
    pub lines: Option<[u32; 2]>,
    pub added: Option<u32>,
    pub removed: Option<u32>,
    pub diff: Option<Vec<DiffHunk>>,
    pub error: Option<String>,
    pub started_at: String,
    pub finished_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct ToolLive {
    pub id: String,
    pub tool: Option<String>,
    pub kind: CallKind,
    pub title: String,
    pub target: Option<String>,
    pub lines: Option<[u32; 2]>,
    pub status: CallStatus,
    pub started_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum TodoStatus {
    Pending,
    InProgress,
    Completed,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct TodoItem {
    pub content: String,
    pub status: TodoStatus,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct TodoListCard {
    pub entries: Vec<TodoItem>,
}
