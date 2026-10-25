use core_db::newtype_id;
use core_plans::{Focus, PlanId};
use core_runtime::RuntimeInstanceId;
use core_threads::{ThreadEntryId, ThreadId};

pub use core_operations::OperationId;

newtype_id! {
    /// A message written while a turn ran, waiting to be sent.
    QueuedMessageId
}

/// A waiting message with the settings it will be sent under.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct QueuedMessage {
    pub id: QueuedMessageId,
    pub thread_id: ThreadId,
    /// Send order within the thread; never reused while the row lives.
    pub position: i64,
    pub prompt: String,
    #[serde(default)]
    pub attachments: Vec<core_threads::AttachmentId>,
    pub model: String,
    pub mode: String,
    pub effort: Option<String>,
    pub focus: Option<Focus>,
    pub plan: Option<PlanId>,
    /// Why the last attempt to send it failed, if one did.
    pub last_error: Option<String>,
    pub created_at: String,
}

/// What queueing answers.
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

/// What Send now answers.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum SentNow {
    /// The running turn took it as steering.
    Steered { entry_id: ThreadEntryId },
    /// The thread had gone idle: a turn started.
    Started { operation_id: OperationId },
}

/// Prepare failure ("we could not get ready") and Spawn failure
/// ("process creation failed") are kept distinct in the durable record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailureStage {
    /// Resolving the harness, building the environment, readying the
    /// workspace. The process never existed.
    Prepare,
    /// Process creation failed in the OS.
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

/// The durable operation record; its thread reference is stored as plain text.
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
    /// What the turn asked of the harness and what it reported.
    /// `None` when no invocation is recorded.
    #[schema(required)]
    pub invocation: Option<InvocationView>,
}

pub use core_harness::InvocationView;
