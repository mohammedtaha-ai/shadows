//! One job: the project's state as Shadows knows it — the `ProjectState` service.
//!
//! It composes Design's elements and layout, Plans' live versions, Runs' task
//! progress and CodeRecord's element states into one read, and holds no table:
//! task progress is Runs', and Runs depends on CodeRecord, so neither of them
//! can answer it.

mod error;
mod model;
mod state;

pub use error::ProjectStateError;
pub use model::{
    ElementGap, ElementView, PartDetail, PartState, PlanProgress, PlanTasks, ProjectStateView,
    StateCounts, TaskCounts, TaskLine,
};

use code_record::CodeRecord;
use core_design::Design;
use core_plans::Plans;
use core_runs::Runs;

/// The project's state. Cheap to clone: each service it reads is.
#[derive(Clone)]
pub struct ProjectState {
    design: Design,
    plans: Plans,
    runs: Runs,
    code_record: CodeRecord,
}

impl ProjectState {
    /// Each argument is one service ProjectState reads, built once in AppCore.
    pub fn new(design: Design, plans: Plans, runs: Runs, code_record: CodeRecord) -> Self {
        Self {
            design,
            plans,
            runs,
            code_record,
        }
    }
}
