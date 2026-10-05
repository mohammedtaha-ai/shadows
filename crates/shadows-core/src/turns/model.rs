use crate::id::newtype_id;
use crate::plans::{Focus, PlanId};
use crate::runtime::RuntimeInstanceId;
use crate::threads::{ThreadEntryId, ThreadId};

newtype_id! {
    /// Spec §4.1. This and `RuntimeInstanceId` are the pair that first sat
    /// adjacent in one call — `mark_operation_started(op_id, expected_runtime)` —
    /// where two `String`s compiled cleanly when swapped.
    OperationId
}

newtype_id! {
    /// Spec §20.2: a message written while a turn ran, waiting to be sent.
    QueuedMessageId
}

/// Spec §20.2: a waiting message with the settings it will be sent under.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct QueuedMessage {
    pub id: QueuedMessageId,
    pub thread_id: ThreadId,
    /// Send order within the thread; never reused while the row lives.
    pub position: i64,
    pub prompt: String,
    pub model: String,
    pub mode: String,
    pub effort: Option<String>,
    pub focus: Option<Focus>,
    pub plan: Option<PlanId>,
    /// Why the last attempt to send it failed (§20.3), if one did.
    pub last_error: Option<String>,
    pub created_at: String,
}

/// Spec §20.2: what queueing answers.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
#[serde(tag = "status", rename_all = "snake_case")]
#[expect(
    clippy::large_enum_variant,
    reason = "one answer per request; boxing buys nothing"
)]
pub enum Queued {
    /// The thread was busy: the message waits.
    Waiting { message: QueuedMessage },
    /// The thread was idle: a turn started.
    Started { operation_id: OperationId },
}

/// Spec §20.3: what Send now answers.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum SentNow {
    /// The running turn took it as steering.
    Steered { entry_id: ThreadEntryId },
    /// The thread had gone idle: a turn started.
    Started { operation_id: OperationId },
}

/// Spec §8.3: Prepare failure ("we could not get ready") and Spawn failure
/// ("the OS refused to start it") are kept distinct in the durable record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailureStage {
    /// Resolving the harness, building the environment, readying the
    /// workspace. The process never existed. Spec §8.3.
    Prepare,
    /// The OS refused to start the process. Spec §2.7.
    #[cfg_attr(
        not(feature = "test-support"),
        expect(
            dead_code,
            reason = "no production path records a `Spawn` failure today; only tests do"
        )
    )]
    Spawn,
    /// The process ran and failed.
    Run,
}

impl FailureStage {
    pub fn as_str(self) -> &'static str {
        match self {
            FailureStage::Prepare => "Prepare",
            FailureStage::Spawn => "Spawn",
            FailureStage::Run => "Run",
        }
    }
}

/// Spec §2.7, §6.14. `thread_id` stays a plain `String` here on purpose: the
/// `ProjectId`/`ThreadId`/`ThreadEntryId` sweep is a separate change, staged
/// for reviewability, that follows this task (spec §4.1 OPEN block).
#[derive(Debug, Clone, serde::Serialize, utoipa::ToSchema)]
pub struct Operation {
    pub id: OperationId,
    pub kind: String,
    pub status_kind: String,
    pub thread_id: Option<String>,
    pub runtime_instance_id: RuntimeInstanceId,
    pub outcome_json: Option<String>,
    pub failure_stage: Option<String>,
    pub failure_reason: Option<String>,
    pub interrupt_reason: Option<String>,
    pub cancel_requested_at: Option<String>,
    pub created_at: String,
    pub started_at: Option<String>,
    pub finished_at: Option<String>,
    /// What the turn asked of the harness and what it reported (§12.7).
    /// `None` for an operation from before invocations were recorded.
    #[schema(required)]
    pub invocation: Option<InvocationView>,
}

/// What a turn asked for and what the harness reported (spec §8.2, §12.8).
/// Anything not reported is `None`, never estimated.
#[derive(Debug, Clone, PartialEq, serde::Serialize, utoipa::ToSchema)]
pub struct InvocationView {
    pub harness_kind: String,
    pub harness_version: String,
    pub agent_version: String,
    pub requested_model: String,
    pub requested_mode: String,
    #[schema(required)]
    pub requested_effort: Option<String>,
    #[schema(required)]
    pub observed_model: Option<String>,
    #[schema(required)]
    pub context_used: Option<i64>,
    #[schema(required)]
    pub context_window: Option<i64>,
}
