mod app;
pub use core_runtime::RuntimeError;
mod error;
pub use core_project_lifecycle::ProjectLifecycle;
#[cfg(feature = "test-support")]
pub mod testing;
pub use core_grants::{ApprovalRequest, ApprovedBy, Approver, Writer};

// The public surface: the application, its failure, the
// services, and every type an adapter names or serializes. Storage, the
// sessions, the live-turn registry and the runtime are private to the crate;
// tests reach what they need through `testing`, under `test-support`.
#[cfg(feature = "test-support")]
pub use app::CoreParts;
pub use app::{AppCore, StartConfig};
pub use core_code::{
    Answer, Asker, Code, CodeConfig, CodeSettings, Hit, IndexState, ProjectLink, ProjectStatus,
    Skipped,
};
// `StorageError`, `StartError` and `DirectoryError` are what `CoreError`
// carries, which each adapter maps.
pub use core_agents::{Agent, AgentInput, AgentWorkspace, Agents, AgentsError};
pub use core_db::Actor;
pub use core_db::StorageError;
pub use core_design::StructureWritten;
pub use core_design::{
    AgreementContent, AgreementId, AgreementIssue, AgreementParty, AgreementRole, AgreementState,
    AgreementVersion, AgreementWriter, ContractHead, Design, DesignAnchor, DesignChange,
    DesignRevision, InitialFile, MapAction, MapProposal, MapPropose, Outcome, OutcomeContent,
    OutcomeId, OutcomePage, Part, PartContent, PartId, PartPage, PartWaiver, ProposedPart,
    ProposedWaiver, Stage, StageView, VisionContent, VisionField, VisionItem, VisionView,
    WorkspaceView,
};
pub use core_design::{ProposalStatus, VisionAction, VisionProposal, VisionPropose};
pub use core_design::{ProposedOutcome, RoadmapAction, RoadmapProposal, RoadmapPropose};
pub use core_events::{Delivery, Events, StoredEvent, Subscription};
pub use core_extensions::{
    AgentExtensions, AgentItem, ExtensionKind, ExtensionLibrary, Extensions, ExtensionsError, Item,
    McpInput, McpItem,
};
pub use core_grants::{Grant, GrantId, GrantKind, Grants, GrantsError, IssuedView};
pub use core_harness::{
    AylulConfig, AylulUsageSnapshot, Harness, HarnessError, HarnessInfo, LeaseError, OpenError,
    OpenSession, RememberedSettings,
};
pub use core_instructions::{Instructions, InstructionsVersion};
pub use core_operations::{
    OperationId, OperationRecord, Operations, OperationsError, ReconcileReport, Transition,
};
pub use core_person::{Person, PersonPatch, PersonSettings, Theme};
pub use core_plans::UiSignal;
pub use core_plans::{
    AcceptanceItem, AgreementBinding, AgreementParticipantImpact, AgreementPartyReview,
    AgreementReview, Approved, BindingParticipant, DesignOp, DraftStart, DraftStarted, EditOutcome,
    Focus, LastEdit, Link, LinkKind, LinkedTask, MapLink, MapPlan, OutcomeView, PartView, Place,
    Plan, PlanContent, PlanEdit, PlanId, PlanListing, PlanMap, PlanOp, PlanShow, PlanShown,
    PlanState, PlanTask, PlanVersions, Plans, Problem, TaskContent, TaskId, TaskParent,
    TaskPreview, VersionLine, WorkflowId, WorkflowState, WrittenBy,
};
pub use core_projects::{
    DirectoryEntry, DirectoryError, DirectoryListing, Project, ProjectId, Projects,
};
pub use core_providers::{
    ListedModel, PresetRegion, Provider, ProviderInput, ProviderModel, ProviderModelInput,
    ProviderPreset, ProviderTest, ProviderTestOutcome, Providers, SavedProviderTest, TestRefusal,
};
pub use core_runtime::{RuntimeInstanceId, StopKind};
pub use core_secrets::{EnvEntry, KeySource, SecretKey, Secrets, SecretsError};
pub use core_threads::{
    Attachment, AttachmentId, AttachmentRef, EntryRef, MAX_IMAGE_BYTES, OperationRef,
    PlanningThread, TaskRef, ThreadEntry, ThreadEntryId, ThreadEntryKind, ThreadId, Threads,
    WorkflowRef,
};
pub use core_turns::{
    InvocationView, Operation, Queued, QueuedMessage, QueuedMessageId, SendTurn, SentNow,
    StartError, Turns,
};
pub use core_usage::{ModelUsage, ProviderUsage, Usage, UsageReport};
pub use error::{CoreError, ErrorCode};
// `Harness`'s session methods answer `shadows-agent`'s `SessionChoices`, which
// `shadows-http` serializes. Re-exported so no adapter depends on
// `shadows-agent`; adapters serialize the same schema name.
pub use shadows_agent::choices::SessionChoices;
pub use shadows_agent::events::{CallKind, CallStatus, DiffHunk, DiffLine, DiffOp, ToolCallCard};
pub use shadows_agent::events::{TodoItem, TodoListCard, TodoStatus, ToolLive};
pub use shadows_agent::profile::ClaudeProfile;

pub use core_standards::{
    AdditionalPart, BaseStandards, ContractTemplate, EffectiveStandards, MandatoryPart,
    StandardRule, Standards, StandardsAdditions, StandardsAdditionsVersion,
};

/// Application error translation for the existing validation entry point.
pub fn validate_env_name(name: &str) -> Result<(), CoreError> {
    Secrets::validate_env_name(name).map_err(Into::into)
}

pub use core_providers::ProvidersError;

pub use core_projects::ProjectsError;

pub use core_usage::UsageError;

pub use core_person::PersonError;

pub use core_instructions::InstructionsError;

pub use core_standards::StandardsError;

pub use core_code::CodeError;

pub use core_threads::ThreadsError;

pub use core_design::DesignError;

pub use core_plans::PlansError;

pub use core_turns::TurnsError;

pub use core_project_lifecycle::ProjectLifecycleError;

pub use core_events::EventsError;
