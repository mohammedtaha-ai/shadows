mod app;
mod code;
mod command;
mod db;
mod design;
mod error;
mod events;
mod grants;
mod harness;
mod id;
mod instructions;
mod plans;
mod profiles;
mod projects;
mod runtime;
mod secrets;
#[cfg(feature = "test-support")]
pub mod testing;
mod threads;
mod turns;
pub use command::{ApprovalRequest, ApprovedBy, Approver, Writer};

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
pub use db::StorageError;
pub use design::StructureWritten;
pub use design::{
    AdditionalPart, AgreementContent, AgreementId, AgreementIssue, AgreementParticipantImpact,
    AgreementParty, AgreementPartyReview, AgreementReview, AgreementRole, AgreementState,
    AgreementVersion, AgreementWriter, BaseStandards, ContractHead, ContractTemplate, Design,
    DesignAnchor, DesignChange, DesignOp, DesignRevision, EffectiveStandards, InitialFile,
    MandatoryPart, MapAction, MapProposal, MapPropose, Outcome, OutcomeContent, OutcomeId,
    OutcomePage, OutcomeView, Part, PartContent, PartId, PartPage, PartView, PartWaiver,
    ProposedPart, ProposedWaiver, Stage, StageView, StandardRule, StandardsAdditions,
    StandardsAdditionsVersion, VisionContent, VisionField, VisionItem, VisionView, WorkspaceView,
    base_standards,
};
pub use design::{ProposalStatus, VisionAction, VisionProposal, VisionPropose};
pub use design::{ProposedOutcome, RoadmapAction, RoadmapProposal, RoadmapPropose};
pub use error::{CoreError, ErrorCode};
pub use events::{Actor, Delivery, Events, StoredEvent, Subscription, UiSignal};
pub use grants::{Grant, GrantId, GrantKind, Grants, IssuedView};
pub use harness::{
    AylulConfig, ContextBreakdown, Harness, HarnessInfo, OpenError, RememberedSettings,
};
pub use instructions::{Instructions, InstructionsVersion};
pub use plans::{
    AcceptanceItem, AgreementBinding, Approved, BindingParticipant, DraftStart, DraftStarted,
    EditOutcome, Focus, LastEdit, Link, LinkKind, LinkedTask, MapLink, MapPlan, Place, Plan,
    PlanContent, PlanEdit, PlanId, PlanListing, PlanMap, PlanOp, PlanShow, PlanShown, PlanState,
    PlanTask, PlanVersions, Plans, Problem, TaskContent, TaskId, TaskParent, TaskPreview,
    VersionLine, WorkflowId, WorkflowState, WrittenBy,
};
pub use profiles::{
    Agent, AgentInput, AgentWorkspace, ListedModel, Person, PersonInput, PresetRegion, Profiles,
    Provider, ProviderInput, ProviderModel, ProviderModelInput, ProviderPreset, ProviderTest,
    ProviderTestOutcome, SavedProviderTest, TestRefusal, Theme,
};
pub use projects::{
    DirectoryEntry, DirectoryError, DirectoryListing, Project, ProjectId, Projects,
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
// `Harness`'s session methods answer `shadows-agent`'s `SessionChoices`, which
// `shadows-http` serializes. Re-exported so no adapter depends on
// `shadows-agent` (spec §14.3, §14.6 #1); the schema name is unchanged.
pub use shadows_agent::choices::SessionChoices;
pub use shadows_agent::profile::ClaudeProfile;
