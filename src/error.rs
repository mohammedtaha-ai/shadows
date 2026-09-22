use std::fmt;

/// Stable codes clients pattern-match on. Never match on human text.
/// Spec §3.4. `Blocked`/`Rejected` are domain outcomes and never appear here.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ErrorCode {
    ProcessSpawnFailed,
    ProcessTerminated,
    StorageUnavailable,
    StorageMigrationFailed,
    StorageConstraintViolation,
    CommandConflict,
    IdempotencyKeyRequired,
    InvalidCommand,
    InvalidCursor,
    AgentAuthFailed,
    AgentUnsupportedProfile,
    PathInvalid,
    PathNotFound,
    PathNotADirectory,
    PathAccessDenied,
    PathAlreadyExists,
    PathUnavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FailureClass {
    Client,
    Infrastructure,
    Agent,
    Storage,
}

/// Classifying a failure as retryable does not authorize an automatic retry.
/// Spec §3.2.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RetryClass {
    Never,
    Immediate,
    Backoff,
    AfterReconfiguration,
    AfterUserAction,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct AppFailure {
    pub code: ErrorCode,
    pub class: FailureClass,
    pub retry: RetryClass,
    pub public_details: String,
}

impl fmt::Display for AppFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}: {}", self.code, self.public_details)
    }
}

impl std::error::Error for AppFailure {}

/// The internal causal chain. Deliberately not `Serialize` — spec §3.2 requires
/// that it cannot accidentally reach a client.
#[derive(Debug)]
pub struct FailureReport {
    pub failure: AppFailure,
    pub source: Option<Box<dyn std::error::Error + Send + Sync>>,
}
