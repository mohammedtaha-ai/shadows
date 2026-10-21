use crate::db::Actor;
use crate::grants::GrantId;
use crate::threads::ThreadId;

pub mod approver;
pub use approver::{ApprovalRequest, ApprovedBy, Approver};

/// Who writes a plan (spec §13.5), and so the principal its commands are
/// recorded under.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Writer {
    /// A person adopting an agreement through Plans' HTTP entry point (§18.8).
    Person,
    /// The internal Planner of `thread`, holding `grant`. Its principal is the
    /// thread, not the grant: the grant is replaced whenever an adapter opens,
    /// and a retry after a resume must still find the first result.
    Planner { thread: ThreadId, grant: GrantId },
    /// An external agent holding a project grant.
    External { grant: GrantId },
}

impl Writer {
    /// `(principal_kind, principal_id)` of the command record.
    pub fn principal(&self) -> (&'static str, String) {
        match self {
            Writer::Person => ("User", "local".to_string()),
            Writer::Planner { thread, .. } => ("Thread", thread.to_string()),
            Writer::External { grant } => ("Grant", grant.to_string()),
        }
    }

    /// The actor of the events the writer's commands append: its principal.
    pub fn actor(&self) -> Actor {
        let (kind, id) = self.principal();
        Actor {
            kind: kind.to_string(),
            id,
        }
    }

    /// The grant a write must find valid and in scope (§13.7); none for a person.
    pub fn grant(&self) -> Option<&GrantId> {
        match self {
            Writer::Person => None,
            Writer::Planner { grant, .. } | Writer::External { grant } => Some(grant),
        }
    }
}
