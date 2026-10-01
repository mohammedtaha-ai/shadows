mod app;
mod code;
mod command;
mod db;
mod error;
mod events;
mod grants;
mod harness;
mod id;
mod instructions;
mod plans;
mod projects;
mod runtime;
#[cfg(feature = "test-support")]
pub mod testing;
mod threads;
mod turns;

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
pub use error::{CoreError, ErrorCode};
pub use events::{Actor, Delivery, Events, StoredEvent, Subscription, UiSignal};
pub use grants::{Grant, GrantId, GrantKind, Grants, IssuedView};
pub use harness::{ContextBreakdown, Harness, HarnessInfo, OpenError, RememberedSettings};
pub use instructions::{Instructions, InstructionsVersion};
pub use plans::{
    AcceptanceItem, Approved, DraftStart, DraftStarted, EditOutcome, Focus, LastEdit, Link,
    LinkKind, Place, Plan, PlanContent, PlanEdit, PlanId, PlanListing, PlanOp, PlanShow, PlanShown,
    PlanState, PlanTask, Plans, Problem, TaskContent, TaskId, WorkflowId, WorkflowState,
};
pub use projects::{
    DirectoryEntry, DirectoryError, DirectoryListing, Project, ProjectId, Projects,
};
pub use runtime::{RuntimeInstanceId, StopKind};
pub use threads::{
    EntryRef, PlanningThread, ThreadEntry, ThreadEntryId, ThreadEntryKind, ThreadId, Threads,
};
pub use turns::{InvocationView, Operation, OperationId, SendTurn, StartError, Turns};
// `Harness`'s session methods answer `shadows-agent`'s `SessionChoices`, which
// `shadows-http` serializes. Re-exported so no adapter depends on
// `shadows-agent` (spec §14.3, §14.6 #1); the schema name is unchanged.
pub use shadows_agent::choices::SessionChoices;
