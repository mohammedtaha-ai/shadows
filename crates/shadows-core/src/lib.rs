mod app;
mod design;
mod error;
mod events;
mod harness;
mod plans;
mod project_lifecycle;
pub use project_lifecycle::ProjectLifecycle;
#[cfg(feature = "test-support")]
pub mod testing;
mod turns;
pub use core_grants::{ApprovalRequest, ApprovedBy, Approver, Writer};

// The public surface (spec §14.4, §14.6): the application, its failure, the
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
pub use core_extensions::{
    AgentExtensions, AgentItem, ExtensionKind, ExtensionLibrary, Extensions, ExtensionsError, Item,
};
pub use core_grants::{Grant, GrantId, GrantKind, Grants, GrantsError, IssuedView};
pub use core_instructions::{Instructions, InstructionsVersion};
pub use core_operations::{
    OperationId, OperationRecord, Operations, OperationsError, ReconcileReport, Transition,
};
pub use core_person::{Person, PersonInput, PersonSettings, Theme};
pub use core_projects::{
    DirectoryEntry, DirectoryError, DirectoryListing, Project, ProjectId, Projects,
};
pub use core_providers::{
    ListedModel, PresetRegion, Provider, ProviderInput, ProviderModel, ProviderModelInput,
    ProviderPreset, ProviderTest, ProviderTestOutcome, Providers, SavedProviderTest, TestRefusal,
};
pub use core_runtime::{RuntimeInstanceId, StopKind};
pub use core_secrets::{EnvEntry, SecretKey, Secrets, SecretsError};
pub use core_threads::{
    Attachment, AttachmentId, AttachmentRef, EntryRef, MAX_IMAGE_BYTES, OperationRef,
    PlanningThread, TaskRef, ThreadEntry, ThreadEntryId, ThreadEntryKind, ThreadId, Threads,
    WorkflowRef,
};
pub use core_usage::{ModelUsage, ProviderUsage, Usage, UsageReport};
pub use design::StructureWritten;
pub use design::{
    AgreementContent, AgreementId, AgreementIssue, AgreementParty, AgreementRole, AgreementState,
    AgreementVersion, AgreementWriter, ContractHead, Design, DesignAnchor, DesignChange,
    DesignRevision, InitialFile, MapAction, MapProposal, MapPropose, Outcome, OutcomeContent,
    OutcomeId, OutcomePage, Part, PartContent, PartId, PartPage, PartWaiver, ProposedPart,
    ProposedWaiver, Stage, StageView, VisionContent, VisionField, VisionItem, VisionView,
    WorkspaceView,
};
pub use design::{ProposalStatus, VisionAction, VisionProposal, VisionPropose};
pub use design::{ProposedOutcome, RoadmapAction, RoadmapProposal, RoadmapPropose};
pub use error::{CoreError, ErrorCode};
pub use events::{Delivery, Events, StoredEvent, Subscription};
pub use harness::{
    AylulConfig, AylulUsageSnapshot, ContextBreakdown, Harness, HarnessInfo, LeaseError, OpenError,
    OpenSession, RememberedSettings,
};
pub use plans::UiSignal;
pub use plans::{
    AcceptanceItem, AgreementBinding, AgreementParticipantImpact, AgreementPartyReview,
    AgreementReview, Approved, BindingParticipant, DesignOp, DraftStart, DraftStarted, EditOutcome,
    Focus, LastEdit, Link, LinkKind, LinkedTask, MapLink, MapPlan, OutcomeView, PartView, Place,
    Plan, PlanContent, PlanEdit, PlanId, PlanListing, PlanMap, PlanOp, PlanShow, PlanShown,
    PlanState, PlanTask, PlanVersions, Plans, Problem, TaskContent, TaskId, TaskParent,
    TaskPreview, VersionLine, WorkflowId, WorkflowState, WrittenBy,
};
pub use turns::{
    InvocationView, Operation, Queued, QueuedMessage, QueuedMessageId, SendTurn, SentNow,
    StartError, Turns,
};
// `Harness`'s session methods answer `shadows-agent`'s `SessionChoices`, which
// `shadows-http` serializes. Re-exported so no adapter depends on
// `shadows-agent` (spec §14.3, §14.6 #1); the schema name is unchanged.
pub use shadows_agent::choices::SessionChoices;
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
