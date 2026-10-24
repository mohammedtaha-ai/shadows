use core_db::newtype_id;

newtype_id! {
    /// Pins an operation transition to the process that owns it.
    RuntimeInstanceId
}

impl RuntimeInstanceId {
    /// Reconstructs an id already persisted by a service that refers to this runtime.
    pub fn from_recorded(id: String) -> Self {
        Self::from_stored(id)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StopKind {
    Graceful,
    Escalated,
}
impl StopKind {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Graceful => "Graceful",
            Self::Escalated => "Escalated",
        }
    }
}
