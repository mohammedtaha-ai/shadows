mod agents;
mod app;
mod code;
mod db;
mod design;
mod error;
mod events;
mod extensions;
mod grants;
mod harness;
mod instructions;
mod operations;
mod person;
mod plans;
mod projects;
mod providers;
mod runtime;
mod secrets;
mod standards;
#[cfg(feature = "test-support")]
pub mod testing;
mod threads;
mod turns;
mod usage;
pub use grants::{ApprovalRequest, ApprovedBy, Approver, Writer};

// The public surface (spec §14.4, §14.6): the application, its failure, the
// services, and every type an adapter names or serializes. Storage, the
// sessions, the live-turn registry and the runtime are private to the crate;
// tests reach what they need through `testing`, under `test-support`.
#[cfg(feature = "test-support")]
pub use app::CoreParts;
pub use app::{AppCore, StartConfig};
pub use code::{
    Answer, Asker, Code, CodeConfig, CodeSettings, Hit, IndexState, ProjectLink, ProjectStatus,
    Skipped,
};
// `StorageError`, `StartError` and `DirectoryError` are what `CoreError`
// carries, which each adapter maps.
pub use agents::{Agent, AgentInput, AgentWorkspace, Agents};
pub use db::Actor;
pub use db::StorageError;
pub use design::StructureWritten;
pub use design::{
    AgreementContent, AgreementId, AgreementIssue, AgreementParticipantImpact, AgreementParty,
    AgreementPartyReview, AgreementReview, AgreementRole, AgreementState, AgreementVersion,
    AgreementWriter, ContractHead, Design, DesignAnchor, DesignChange, DesignOp, DesignRevision,
    InitialFile, MapAction, MapProposal, MapPropose, Outcome, OutcomeContent, OutcomeId,
    OutcomePage, OutcomeView, Part, PartContent, PartId, PartPage, PartView, PartWaiver,
    ProposedPart, ProposedWaiver, Stage, StageView, VisionContent, VisionField, VisionItem,
    VisionView, WorkspaceView,
};
pub use design::{ProposalStatus, VisionAction, VisionProposal, VisionPropose};
pub use design::{ProposedOutcome, RoadmapAction, RoadmapProposal, RoadmapPropose};
pub use error::{CoreError, ErrorCode};
pub use events::{Delivery, Events, StoredEvent, Subscription, UiSignal};
pub use extensions::{
    AgentExtensions, AgentItem, ExtensionKind, ExtensionLibrary, Extensions, Item,
};
pub use grants::{Grant, GrantId, GrantKind, Grants, GrantsError, IssuedView};
pub use harness::{
    AylulConfig, AylulUsageSnapshot, ContextBreakdown, Harness, HarnessInfo, OpenError,
    RememberedSettings,
};
pub use instructions::{Instructions, InstructionsVersion};
pub use operations::Operations;
pub use person::{Person, PersonInput, PersonSettings, Theme};
pub use plans::{
    AcceptanceItem, AgreementBinding, Approved, BindingParticipant, DraftStart, DraftStarted,
    EditOutcome, Focus, LastEdit, Link, LinkKind, LinkedTask, MapLink, MapPlan, Place, Plan,
    PlanContent, PlanEdit, PlanId, PlanListing, PlanMap, PlanOp, PlanShow, PlanShown, PlanState,
    PlanTask, PlanVersions, Plans, Problem, TaskContent, TaskId, TaskParent, TaskPreview,
    VersionLine, WorkflowId, WorkflowState, WrittenBy,
};
pub use projects::{
    DirectoryEntry, DirectoryError, DirectoryListing, Project, ProjectId, Projects,
};
pub use providers::{
    ListedModel, PresetRegion, Provider, ProviderInput, ProviderModel, ProviderModelInput,
    ProviderPreset, ProviderTest, ProviderTestOutcome, Providers, SavedProviderTest, TestRefusal,
};
pub use runtime::{RuntimeInstanceId, StopKind};
pub use secrets::{EnvEntry, SecretKey, Secrets, validate_env_name};
pub use threads::{
    EntryRef, PlanningThread, ThreadEntry, ThreadEntryId, ThreadEntryKind, ThreadId, Threads,
};
pub use turns::{
    InvocationView, Operation, OperationId, Queued, QueuedMessage, QueuedMessageId, SendTurn,
    SentNow, StartError, Turns,
};
pub use usage::{ModelUsage, ProviderUsage, Usage, UsageReport};
// `Harness`'s session methods answer `shadows-agent`'s `SessionChoices`, which
// `shadows-http` serializes. Re-exported so no adapter depends on
// `shadows-agent` (spec §14.3, §14.6 #1); the schema name is unchanged.
pub use shadows_agent::choices::SessionChoices;
pub use shadows_agent::profile::ClaudeProfile;

pub use standards::{
    AdditionalPart, BaseStandards, ContractTemplate, EffectiveStandards, MandatoryPart,
    StandardRule, Standards, StandardsAdditions, StandardsAdditionsVersion,
};
