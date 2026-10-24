use crate::grants::GrantId;
use core_db::Actor;
use core_threads::ThreadId;
/// The authenticated principal recorded for each write.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Writer {
    Person,
    /// The internal Planner of `thread`, holding `grant`. Its principal is the
    /// thread, not the grant: the grant is replaced whenever an adapter opens,
    /// and a retry after a resume must still find the first result.
    Planner {
        thread: ThreadId,
        grant: GrantId,
    },
    /// An external agent holding a project grant.
    External {
        grant: GrantId,
    },
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
    pub fn grant(&self) -> Option<&GrantId> {
        match self {
            Writer::Person => None,
            Writer::Planner { grant, .. } | Writer::External { grant } => Some(grant),
        }
    }
}

impl Writer {
    pub(crate) fn from_grant(
        grant: &super::Grant,
        missing_thread_message: Option<&str>,
    ) -> Result<Self, crate::CoreError> {
        Ok(match grant.kind {
            super::GrantKind::Project => Self::External {
                grant: grant.id.clone(),
            },
            super::GrantKind::Thread => Self::Planner {
                grant: grant.id.clone(),
                thread: grant
                    .thread_id
                    .clone()
                    .ok_or_else(|| match missing_thread_message {
                        Some(message) => crate::CoreError::Refused {
                            code: crate::ErrorCode::GrantScope,
                            message: message.into(),
                        },
                        None => super::GrantsError::GrantScope.into(),
                    })?,
            },
        })
    }
}
