use std::sync::Arc;

use crate::storage::{ReconcileReport, StopKind, Storage, StorageError};

/// UUID-v4 newtype over the runtime instance identity. Spec §4.1: Task 9 is
/// where an operation id and a runtime instance id first sit adjacent in one
/// call (`mark_operation_started(op_id, expected_runtime)`), and two `String`s
/// there compile cleanly when swapped. This type is what the compiler tells
/// apart instead.
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct RuntimeInstanceId(String);

impl RuntimeInstanceId {
    /// Named `generate` rather than `new` deliberately. A constructor called
    /// `new` that takes nothing and mints a random UUID reads like a cheap
    /// empty value, and `clippy::new_without_default` then demands a `Default`
    /// impl — which would mean `RuntimeInstanceId::default()` silently produces a
    /// *different* id every call. `generate` says what it does, and leaves the
    /// type with no way to be created by accident.
    pub fn generate() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Reconstructs an id already known to be valid — a value read back from
    /// storage. Storage is the only caller; this is not a general parser.
    pub(crate) fn from_stored(id: String) -> Self {
        Self(id)
    }
}

/// Owns this process's runtime identity for its whole lifetime. Spec §8.1:
/// startup performs migration, ownership, containment setup, and recovery
/// before any work is accepted.
pub struct Runtime {
    pub instance_id: RuntimeInstanceId,
    pub storage: Arc<Storage>,
}

impl Runtime {
    pub async fn start(storage: Arc<Storage>) -> Result<(Self, ReconcileReport), StorageError> {
        let version = env!("CARGO_PKG_VERSION");
        let instance_id = storage.register_runtime_instance(version).await?;
        let report = storage.reconcile_orphans(&instance_id).await?;

        for op in &report.interrupted {
            tracing::info!(operation_id = %op, "recovery.reconcile");
        }
        for op in &report.anomalies {
            tracing::error!(
                operation_id = %op,
                "recovery.anomaly: a Graceful runtime owned a non-terminal operation"
            );
        }
        Ok((
            Self {
                instance_id,
                storage,
            },
            report,
        ))
    }

    pub async fn stop(&self, kind: StopKind) -> Result<(), StorageError> {
        self.storage
            .stop_runtime_instance(&self.instance_id, kind)
            .await
    }
}
