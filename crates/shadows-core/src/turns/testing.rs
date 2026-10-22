use crate::db::{Actor, Storage, StorageError};
use crate::runtime::RuntimeInstanceId;
use crate::threads::ThreadId;
use crate::turns::model::FailureStage;
use crate::turns::{Operation, OperationId, Turns};
use shadows_agent::events::{TurnObservation, TurnUsage};
use std::sync::Arc;
impl Turns {
    pub async fn create_pending_operation_for_test(
        storage: &Arc<Storage>,
        thread: &ThreadId,
        runtime: &RuntimeInstanceId,
    ) -> Result<OperationId, StorageError> {
        super::store::TurnsStore { storage }
            .create_pending_operation(thread, runtime)
            .await
    }
    pub async fn mark_operation_started_for_test(
        storage: &Arc<Storage>,
        op: &OperationId,
        runtime: &RuntimeInstanceId,
    ) -> Result<(), StorageError> {
        super::store::TurnsStore { storage }
            .mark_operation_started(op, runtime)
            .await
    }
    pub async fn mark_operation_completed_for_test(
        storage: &Arc<Storage>,
        op: &OperationId,
        outcome: serde_json::Value,
        observation: &TurnObservation,
    ) -> Result<(), StorageError> {
        super::store::TurnsStore { storage }
            .mark_operation_completed(op, outcome, observation)
            .await
    }
    pub async fn mark_operation_failed_for_test(
        storage: &Arc<Storage>,
        op: &OperationId,
        stage: FailureStage,
        reason: &str,
    ) -> Result<(), StorageError> {
        super::store::TurnsStore { storage }
            .mark_operation_failed(op, stage, reason)
            .await
    }
    pub async fn mark_operation_failed_with_usage_for_test(
        storage: &Arc<Storage>,
        op: &OperationId,
        stage: FailureStage,
        reason: &str,
        usage: Option<&TurnUsage>,
    ) -> Result<(), StorageError> {
        super::store::TurnsStore { storage }
            .mark_operation_failed_with_usage(op, stage, reason, usage)
            .await
    }
    pub async fn mark_operation_cancelled_for_test(
        storage: &Arc<Storage>,
        op: &OperationId,
    ) -> Result<(), StorageError> {
        super::store::TurnsStore { storage }
            .mark_operation_cancelled(op)
            .await
    }
    pub async fn mark_operation_cancelled_with_usage_for_test(
        storage: &Arc<Storage>,
        op: &OperationId,
        usage: Option<&TurnUsage>,
    ) -> Result<(), StorageError> {
        super::store::TurnsStore { storage }
            .mark_operation_cancelled_with_usage(op, usage)
            .await
    }
    pub async fn request_cancellation_for_test(
        storage: &Arc<Storage>,
        op: &OperationId,
        actor: Actor,
    ) -> Result<(), StorageError> {
        super::store::TurnsStore { storage }
            .request_cancellation(op, actor)
            .await
    }
    pub async fn get_operation_for_test(
        storage: &Arc<Storage>,
        op: &OperationId,
    ) -> Result<Operation, StorageError> {
        super::store::TurnsStore { storage }.get_operation(op).await
    }
    pub async fn list_operations_for_thread_for_test(
        storage: &Arc<Storage>,
        thread: &ThreadId,
    ) -> Result<Vec<Operation>, StorageError> {
        super::store::TurnsStore { storage }
            .list_operations_for_thread(thread)
            .await
    }
}
