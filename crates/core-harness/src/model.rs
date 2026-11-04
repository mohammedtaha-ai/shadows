//! One job: the harness shapes a caller meets.
//!
//! Harness builds these values; adapters serialize them.

use shadows_agent::events::AccountLimits;
use shadows_agent::policy;

/// The model and effort last chosen for this harness.
#[derive(serde::Serialize, utoipa::ToSchema)]
#[schema(description = include_str!("schema_descriptions/preferences.txt").replace("\r\n", "\n"))]
pub struct RememberedSettings {
    pub(super) model: String,
    #[schema(required)]
    pub(super) effort: Option<String>,
}

/// Why a thread's session lost what it heard.
#[derive(Debug, Clone)]
pub enum Forget {
    /// A compaction completed in the turn `operation`; `trigger` is `manual`
    /// or `automatic` when the adapter said.
    Compaction {
        operation: core_operations::OperationId,
        trigger: Option<String>,
    },
    /// The person asked for the context again.
    Request { command_id: String },
}

/// A mode Shadows allows on a harness, with the words a client shows for it.
#[derive(serde::Serialize, utoipa::ToSchema)]
pub struct HarnessMode {
    pub(super) id: String,
    pub(super) label: String,
    pub(super) description: String,
}

/// A CLI a conversation can run on. `kind` is `claude-code` or
/// `codex`.
#[derive(serde::Serialize, utoipa::ToSchema)]
#[schema(description = include_str!("schema_descriptions/harness-info.txt").replace("\r\n", "\n"))]
pub struct HarnessInfo {
    pub(super) kind: String,
    pub(super) label: String,
    pub(super) available: bool,
    /// Why it cannot run, when `available` is false.
    #[schema(required)]
    pub(super) reason: Option<String>,
    #[schema(required)]
    pub(super) remembered: Option<RememberedSettings>,
    #[schema(required)]
    pub(super) limits: Option<AccountLimits>,
    /// The modes Shadows allows, in listed order; empty when none are decided.
    pub(super) modes: Vec<HarnessMode>,
    /// The mode a new conversation starts in.
    #[schema(required)]
    pub(super) initial_mode: Option<String>,
    /// The allowed mode that asks nothing.
    #[schema(required)]
    pub(super) unattended_mode: Option<String>,
}

impl super::Harness {
    pub(super) fn label(kind: &str) -> &'static str {
        match kind {
            policy::CLAUDE_CODE => "Claude Code",
            policy::CODEX => "Codex",
            policy::AYLUL => "AYLUL",
            _ => "Unknown",
        }
    }
}

/// Credential-free routing metadata pinned to the actual open adapter.
#[derive(Debug, Clone)]
pub struct AylulUsageSnapshot {
    pub agent_slug: String,
    pub provider_slug: String,
    pub provider_name: String,
    pub label: Option<String>,
    pub preset: Option<String>,
}

/// What a turn asked for and what the harness reported.
/// Anything not reported is `None`, never estimated.
#[derive(Debug, Clone, PartialEq, serde::Serialize, utoipa::ToSchema)]
#[schema(description = include_str!("schema_descriptions/invocation.txt").replace("\r\n", "\n"))]
pub struct InvocationView {
    pub harness_kind: String,
    pub harness_version: String,
    pub agent_version: String,
    pub requested_model: String,
    pub requested_mode: String,
    #[schema(required)]
    pub requested_effort: Option<String>,
    #[schema(required)]
    pub observed_model: Option<String>,
    #[schema(required)]
    pub context_used: Option<i64>,
    #[schema(required)]
    pub context_window: Option<i64>,
}

/// Versions delivered by the thread's most recent started turn.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvocationVersions {
    pub prompt: Option<String>,
    pub instructions: Option<String>,
    pub standards: Option<i64>,
    pub additions: Option<String>,
}

/// Nonsecret input pinned by the transaction that starts a turn.
pub struct NewInvocation<'a> {
    pub identity: &'a [String; 6],
    pub settings: &'a shadows_agent::TurnSettings,
    pub versions: &'a InvocationVersions,
    pub snapshot: Option<&'a AylulUsageSnapshot>,
}

pub struct InvocationUsage {
    pub agent_slug: String,
    pub provider_slug: String,
    pub provider_name: String,
    pub label: Option<String>,
    pub preset: Option<String>,
    pub model: String,
}
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct PermissionAnswer {
    pub thread_id: core_threads::ThreadId,
    pub request_id: String,
    pub option_id: String,
    pub kind: String,
    pub answered_at: String,
}

#[derive(Debug, Clone, serde::Serialize, utoipa::ToSchema)]
pub struct WaitingPermission {
    pub thread_id: core_threads::ThreadId,
    pub agent: String,
    pub request_id: String,
    pub title: String,
    pub options: Vec<shadows_agent::events::PermissionOption>,
    pub asked_at: String,
}
/// Planning material prepared for one invocation.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct CarriedContext {
    pub digest: String,
    pub text: String,
    pub sources: Vec<ContextSource>,
    pub index_tokens: i64,
    pub detail_tokens: i64,
    pub over_tokens: i64,
    pub cuts: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct ContextSource {
    pub kind: String,
    pub id: String,
    pub version: i64,
}
