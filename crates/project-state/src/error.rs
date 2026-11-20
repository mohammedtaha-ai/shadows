//! One job: the refusals of the project's state.

#[derive(Debug, thiserror::Error)]
pub enum ProjectStateError {
    /// The asked part is not in the project's approved map.
    #[error("{0}")]
    UnknownPart(String),
    #[error(transparent)]
    Design(#[from] core_design::DesignError),
    #[error(transparent)]
    Plans(#[from] core_plans::PlansError),
    #[error(transparent)]
    Runs(#[from] core_runs::RunsError),
    #[error(transparent)]
    CodeRecord(#[from] code_record::CodeRecordError),
}
