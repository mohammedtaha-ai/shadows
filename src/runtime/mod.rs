use std::sync::Arc;

use crate::id::newtype_id;

use crate::storage::{ReconcileReport, StopKind, Storage, StorageError};

newtype_id! {
    /// Spec §4.1. Recovery is by ownership (§8.6), so this id is what a
    /// transition's CAS pins; confusing it with an operation id would assert the
    /// wrong owner and compile.
    RuntimeInstanceId
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
