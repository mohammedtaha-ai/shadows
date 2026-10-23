mod images;
mod operation;
mod operation_read;
mod queue;
mod remove;
mod standards;
mod steer;
mod turn;
mod usage;

pub use queue::{NewQueued, QueueAnswer};
#[cfg(feature = "test-support")]
pub use turn::InvocationVersions;
pub use turn::{Dequeue, NewTurn, StartedTurn};

#[derive(Clone)]
pub(super) struct TurnsStore {
    pub(super) storage: std::sync::Arc<crate::db::Storage>,
}

impl TurnsStore {
    pub(super) fn with_storage(storage: std::sync::Arc<crate::db::Storage>) -> Self {
        Self { storage }
    }
}
