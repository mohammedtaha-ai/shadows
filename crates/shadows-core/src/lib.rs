pub mod app;
pub mod command;
pub mod error;
pub mod events;
pub mod grants;
pub mod harness;
pub mod id;
pub mod plans;
pub mod project;
pub mod runtime;
pub mod storage;
#[cfg(feature = "test-support")]
pub mod testing;
pub mod thread;
mod turns;

// The public surface (spec §14.4, §14.6): the application, its failure, and
// every type an adapter or a test names or serializes.
pub use app::{AppCore, CoreParts, StartConfig};
pub use error::CoreError;
pub use events::UiSignal;
pub use grants::{Grant, GrantId, GrantKind, Grants, IssuedView};
pub use harness::{ContextBreakdown, Harness, HarnessInfo, RememberedSettings};
pub use plans::{
    AcceptanceItem, Approved, DraftStart, DraftStarted, EditOutcome, Focus, LastEdit, Link,
    LinkKind, Place, Plan, PlanContent, PlanEdit, PlanListing, PlanOp, PlanShow, PlanShown,
    PlanTask, Plans, Problem, TaskContent, TaskId, WorkflowId, WorkflowState,
};
pub use storage::StopKind;
// `StartError` is what `CoreError::Start` carries, which each adapter maps.
pub use turns::{InvocationView, Operation, OperationId, SendTurn, StartError, Turns};
