/// SQLite assigns sequence numbers while holding the write lock, so journal
/// sequence order equals commit order. Other backends must preserve this guarantee.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
pub struct EventCursor(pub i64);
