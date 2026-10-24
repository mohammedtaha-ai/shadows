use crate::harness::Harness;
use crate::operations::Operations;
#[cfg(feature = "test-support")]
use crate::threads::ThreadId;
use crate::turns::model::{FailureStage, OperationId};
use core_db::{Actor, Storage, StorageError};
use core_runtime::RuntimeInstanceId;
use shadows_agent::events::TurnObservation;
impl super::TurnsStore {
    #[cfg(feature = "test-support")]
    pub async fn create_pending_operation(
        &self,
        thread_id: &ThreadId,
        runtime_instance_id: &RuntimeInstanceId,
    ) -> Result<OperationId, StorageError> {
        let id = OperationId::generate();
        let (op_id, thread_id, runtime_id, ts) = (
            id.clone(),
            thread_id.clone(),
            runtime_instance_id.clone(),
            Storage::now(),
        );
        self.storage
            .write_txn(move |conn| {
                Box::pin(async move {
                    Operations::insert_pending_in(conn, &op_id, &thread_id, &runtime_id, &ts).await
                })
            })
            .await?
            .log();
        Ok(id)
    }

    pub async fn mark_operation_started(
        &self,
        op_id: &OperationId,
        expected_runtime: &RuntimeInstanceId,
    ) -> Result<(), StorageError> {
        let (op_id, expected_runtime, ts) =
            (op_id.clone(), expected_runtime.clone(), Storage::now());
        self.storage
            .write_txn(move |conn| {
                Box::pin(async move {
                    Operations::started_in(conn, &op_id, &expected_runtime, &ts).await
                })
            })
            .await?
            .log();
        Ok(())
    }

    pub async fn mark_operation_completed(
        &self,
        op_id: &OperationId,
        outcome: serde_json::Value,
        observation: &TurnObservation,
    ) -> Result<(), StorageError> {
        let (op_id, outcome, seen, ts) = (
            op_id.clone(),
            outcome.to_string(),
            observation.clone(),
            Storage::now(),
        );
        self.storage
            .write_txn(move |conn| {
                Box::pin(async move {
                    Operations::require_status_in(conn, &op_id, "Running").await?;
                    Harness::observe_invocation_in(conn, op_id.as_str(), &seen).await?;
                    Self::record_usage_in(conn, &op_id, seen.token_usage.as_ref(), &ts).await?;
                    let invocation = Harness::invocation_in(conn, op_id.as_str())
                        .await?
                        .map(serde_json::to_value)
                        .transpose()?;
                    Operations::completed_in(conn, &op_id, &outcome, invocation, &ts).await
                })
            })
            .await?
            .log();
        Ok(())
    }

    pub async fn mark_operation_failed(
        &self,
        op_id: &OperationId,
        stage: FailureStage,
        reason: &str,
    ) -> Result<(), StorageError> {
        self.mark_operation_failed_with_usage(op_id, stage, reason, None)
            .await
    }

    pub async fn mark_operation_failed_with_usage(
        &self,
        op_id: &OperationId,
        stage: FailureStage,
        reason: &str,
        usage: Option<&shadows_agent::events::TurnUsage>,
    ) -> Result<(), StorageError> {
        let usage = usage.cloned();
        let (op_id, reason, ts) = (op_id.clone(), reason.to_string(), Storage::now());
        self.storage
            .write_txn(move |conn| {
                Box::pin(async move {
                    Operations::require_unfinished_in(conn, &op_id).await?;
                    Self::record_usage_in(conn, &op_id, usage.as_ref(), &ts).await?;
                    Operations::failed_in(conn, &op_id, stage.as_str(), &reason, &ts).await
                })
            })
            .await?
            .log();
        Ok(())
    }

    pub async fn request_cancellation(
        &self,
        op_id: &OperationId,
        requester: Actor,
    ) -> Result<(), StorageError> {
        let (op_id, requester, ts) = (op_id.clone(), requester, Storage::now());
        let transition = self
            .storage
            .write_txn(move |conn| {
                Box::pin(async move {
                    Operations::request_cancellation_in(conn, &op_id, requester, &ts).await
                })
            })
            .await?;
        if let Some(transition) = transition {
            transition.log();
        }
        Ok(())
    }

    pub async fn mark_operation_cancelled(&self, op_id: &OperationId) -> Result<(), StorageError> {
        self.mark_operation_cancelled_with_usage(op_id, None).await
    }

    pub async fn mark_operation_cancelled_with_usage(
        &self,
        op_id: &OperationId,
        usage: Option<&shadows_agent::events::TurnUsage>,
    ) -> Result<(), StorageError> {
        let usage = usage.cloned();
        let (op_id, ts) = (op_id.clone(), Storage::now());
        self.storage
            .write_txn(move |conn| {
                Box::pin(async move {
                    Operations::require_unfinished_in(conn, &op_id).await?;
                    Self::record_usage_in(conn, &op_id, usage.as_ref(), &ts).await?;
                    Operations::cancelled_in(conn, &op_id, &ts).await
                })
            })
            .await?
            .log();
        Ok(())
    }
}
