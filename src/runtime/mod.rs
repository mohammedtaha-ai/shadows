use std::fmt;
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
    pub fn new() -> Self {
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

impl Default for RuntimeInstanceId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for RuntimeInstanceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
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
