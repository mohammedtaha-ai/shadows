//! Persistence failures exposed by numbered Planner instructions.
#[derive(Debug, thiserror::Error)]
pub enum InstructionsError {
    #[error(transparent)]
    Storage(#[from] core_db::StorageError),
}
