use crate::turns::model::FailureStage;
use crate::turns::{Operation, OperationId, Turns};
use core_db::{Actor, CommandContext, Storage, StorageError};
use core_runtime::RuntimeInstanceId;
use core_threads::ThreadId;
use shadows_agent::events::{TurnObservation, TurnUsage};
use std::sync::Arc;
impl Turns {
    pub async fn start_turn_for_test(
        storage: &std::sync::Arc<Storage>,
        command: &core_db::CommandContext,
        turn: super::store::NewTurn<'_>,
    ) -> Result<super::store::StartedTurn, crate::CoreError> {
        super::store::TurnsStore::with_storage(storage.clone())
            .start_turn(command, turn)
            .await
    }
    pub async fn queue_message_for_test(
        storage: &std::sync::Arc<Storage>,
        command: &core_db::CommandContext,
        thread: &ThreadId,
        new: super::store::NewQueued<'_>,
    ) -> Result<super::store::QueueAnswer, StorageError> {
        super::store::TurnsStore::with_storage(storage.clone())
            .queue_message(command, thread, new)
            .await
    }
    pub async fn queued_messages_for_test(
        storage: &std::sync::Arc<Storage>,
        thread: &ThreadId,
    ) -> Result<Vec<super::QueuedMessage>, StorageError> {
        super::store::TurnsStore::with_storage(storage.clone())
            .queued_messages(thread)
            .await
    }
    pub async fn unqueue_message_for_test(
        storage: &std::sync::Arc<Storage>,
        command: &core_db::CommandContext,
        thread: &ThreadId,
        row: &super::QueuedMessageId,
    ) -> Result<(), crate::CoreError> {
        super::store::TurnsStore::with_storage(storage.clone())
            .unqueue_message(command, thread, row)
            .await
    }
    pub async fn latest_invocation_versions_for_test(
        storage: &std::sync::Arc<Storage>,
        thread: &ThreadId,
    ) -> Result<Option<crate::harness::InvocationVersions>, StorageError> {
        super::store::TurnsStore::with_storage(storage.clone())
            .latest_invocation_versions(thread)
            .await
    }

    pub async fn remove_thread_for_test(
        storage: &Arc<Storage>,
        ctx: &CommandContext,
        thread: &ThreadId,
    ) -> Result<core_threads::PlanningThread, StorageError> {
        super::store::TurnsStore::with_storage(storage.clone())
            .remove_thread(ctx, thread)
            .await
    }

    pub async fn create_pending_operation_for_test(
        storage: &Arc<Storage>,
        thread: &ThreadId,
        runtime: &RuntimeInstanceId,
    ) -> Result<OperationId, StorageError> {
        super::store::TurnsStore {
            storage: storage.clone(),
        }
        .create_pending_operation(thread, runtime)
        .await
    }
    pub async fn mark_operation_started_for_test(
        storage: &Arc<Storage>,
        op: &OperationId,
        runtime: &RuntimeInstanceId,
    ) -> Result<(), StorageError> {
        super::store::TurnsStore {
            storage: storage.clone(),
        }
        .mark_operation_started(op, runtime)
        .await
    }
    pub async fn mark_operation_completed_for_test(
        storage: &Arc<Storage>,
        op: &OperationId,
        outcome: serde_json::Value,
        observation: &TurnObservation,
    ) -> Result<(), StorageError> {
        super::store::TurnsStore {
            storage: storage.clone(),
        }
        .mark_operation_completed(op, outcome, observation)
        .await
    }
    pub async fn mark_operation_failed_for_test(
        storage: &Arc<Storage>,
        op: &OperationId,
        stage: FailureStage,
        reason: &str,
    ) -> Result<(), StorageError> {
        super::store::TurnsStore {
            storage: storage.clone(),
        }
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
        super::store::TurnsStore {
            storage: storage.clone(),
        }
        .mark_operation_failed_with_usage(op, stage, reason, usage)
        .await
    }
    pub async fn mark_operation_cancelled_for_test(
        storage: &Arc<Storage>,
        op: &OperationId,
    ) -> Result<(), StorageError> {
        super::store::TurnsStore {
            storage: storage.clone(),
        }
        .mark_operation_cancelled(op)
        .await
    }
    pub async fn mark_operation_cancelled_with_usage_for_test(
        storage: &Arc<Storage>,
        op: &OperationId,
        usage: Option<&TurnUsage>,
    ) -> Result<(), StorageError> {
        super::store::TurnsStore {
            storage: storage.clone(),
        }
        .mark_operation_cancelled_with_usage(op, usage)
        .await
    }
    pub async fn request_cancellation_for_test(
        storage: &Arc<Storage>,
        op: &OperationId,
        actor: Actor,
    ) -> Result<(), StorageError> {
        super::store::TurnsStore {
            storage: storage.clone(),
        }
        .request_cancellation(op, actor)
        .await
    }
    pub async fn get_operation_for_test(
        storage: &Arc<Storage>,
        op: &OperationId,
    ) -> Result<Operation, StorageError> {
        super::store::TurnsStore {
            storage: storage.clone(),
        }
        .get_operation(op)
        .await
    }
    pub async fn list_operations_for_thread_for_test(
        storage: &Arc<Storage>,
        thread: &ThreadId,
    ) -> Result<Vec<Operation>, StorageError> {
        super::store::TurnsStore {
            storage: storage.clone(),
        }
        .list_operations_for_thread(thread)
        .await
    }
}
