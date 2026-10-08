//! One job: the failures a store can return.

use crate::plans::{PlanId, Problem, WorkflowId};

#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("storage is unavailable: {0}")]
    Unavailable(String),
    #[error("migration failed: {0}")]
    Migration(#[from] sqlx::migrate::MigrateError),
    #[error("constraint violated: {0}")]
    Constraint(String),
    #[error("not found: {0}")]
    NotFound(&'static str),
    #[error("transition conflict: expected {expected}, found {found}")]
    TransitionConflict { expected: String, found: String },
    #[error("command conflict: the same command id was reused with a different request")]
    CommandConflict,
    /// Spec §12.6: the thread already ran a turn on its harness, or is a fork.
    #[error("the thread's harness is fixed: it already ran a turn, or it is a fork")]
    HarnessLocked,
    /// Spec §12.7, §12.9: the thread has a turn that has not ended.
    #[error("the thread has a turn running")]
    ThreadBusy,
    /// Spec §20.5: the waiting message was already sent or removed.
    #[error("the waiting message was already sent or removed")]
    QueuedMessageGone,
    /// Spec §12.9: only the last entry of a completed turn is a fork point.
    #[error("only the thread's last entry, written by a completed turn, can be forked from")]
    ForkPointNotSupported,
    /// Spec §13.5, §18.2: `expected_revision` is not the resource's current one.
    /// `summary` describes what changed since the expected revision.
    #[error("the resource is at revision {current}; changed since: {summary}")]
    RevisionConflict { current: i64, summary: String },
    /// Spec §13.2: a frozen version never changes.
    #[error("the plan version is frozen; start a new version to change it")]
    WorkflowFrozen,
    /// Spec §13.4: the edit's final state, or the approval, breaks a rule.
    #[error("the plan is not valid: {}", problems(.0))]
    PlanInvalid(Vec<Problem>),
    /// §16.7: a new task link names an unavailable or unreachable parent.
    #[error("{0}")]
    PlanLinkInvalid(String),
    /// Spec §13.7: the writer's grant is unknown or revoked.
    #[error("the grant is unknown or revoked")]
    GrantInvalid,
    /// Spec §13.6: the plan, thread or draft ref is outside the writer's grant.
    #[error("outside what the grant allows")]
    GrantScope,
    /// Spec §13.9: a task named by a focus or by `plan_show` is not in that
    /// plan version. The text says which.
    #[error("{0}")]
    TaskNotInPlan(String),
    /// Spec §13.6: a Planner's `draft_start` named a source other than the
    /// version it starts from, its thread's latest, which this carries.
    #[error("the source is not the thread's latest version, {0}")]
    NotLatestVersion(WorkflowId),
    /// Spec §16.3: a version after v1 is started without a non-blank
    /// `change_reason`. A request missing a part, not an invalid plan.
    #[error("a new version needs its reason")]
    ReasonMissing,
    /// §16.2: the plan became read only before this write committed.
    #[error("plan {0} is archived; a person can unarchive it")]
    PlanArchived(PlanId),
    /// Spec §4.2: a project that holds a planning thread is not removed.
    #[error("the project has planning threads; it cannot be removed")]
    ProjectHasThreads,
    #[error("stored JSON is invalid: {0}")]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Database(sqlx::Error),
}

fn problems(list: &[Problem]) -> String {
    list.iter()
        .map(|p| p.message.as_str())
        .collect::<Vec<_>>()
        .join("; ")
}

/// A write the schema refused — a second project with a slug already in use —
/// is `Constraint`, which a client can act on; any other database failure is
/// `Database`. Classified by the driver's kind, never by message text.
impl From<sqlx::Error> for StorageError {
    fn from(error: sqlx::Error) -> Self {
        use sqlx::error::ErrorKind;
        match error
            .as_database_error()
            .map(|db| (db.kind(), db.message()))
        {
            Some((
                ErrorKind::UniqueViolation
                | ErrorKind::ForeignKeyViolation
                | ErrorKind::NotNullViolation
                | ErrorKind::CheckViolation,
                message,
            )) => Self::Constraint(message.to_string()),
            _ => Self::Database(error),
        }
    }
}
