/// SQLite assigns sequence numbers while holding the write lock, so journal
/// sequence order equals commit order. Other backends must preserve this guarantee.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
pub struct EventCursor(pub i64);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EndedTurn {
    pub seq: i64,
    pub thread_id: core_threads::ThreadId,
    pub operation_id: core_operations::OperationId,
    pub outcome: EndedOutcome,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EndedOutcome {
    Completed,
    Failed,
    Cancelled,
    Interrupted,
}
