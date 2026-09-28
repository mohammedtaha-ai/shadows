pub mod app;
pub mod command;
pub mod error;
pub mod events;
pub mod grants;
pub mod id;
pub mod operation;
pub mod planner;
pub mod plans;
pub mod project;
pub mod runtime;
pub mod storage;
#[cfg(feature = "test-support")]
pub mod testing;
pub mod thread;

// The public surface (spec §14.4, §14.6): the application, its failure, and
// every type an adapter or a test names or serializes.
pub use app::{AppCore, CoreParts, StartConfig};
pub use error::CoreError;
pub use events::UiSignal;
pub use grants::{Grant, GrantId, GrantKind, Grants, IssuedView};
pub use plans::{
    AcceptanceItem, Approved, DraftStart, DraftStarted, EditOutcome, Focus, LastEdit, Link,
    LinkKind, Place, Plan, PlanContent, PlanEdit, PlanListing, PlanOp, PlanShow, PlanShown,
    PlanTask, Plans, Problem, TaskContent, TaskId, WorkflowId, WorkflowState,
};
pub use storage::StopKind;
