//! Prepared planning context and the session that heard it.
use crate::{CarriedContext, Harness, StorageError, ThreadId};
use core_operations::OperationId;

impl Harness {
    pub async fn prepare_context(
        &self,
        op: &OperationId,
        thread: &ThreadId,
        session: &str,
        carried: &CarriedContext,
    ) -> Result<(), StorageError> {
        self.store
            .prepare_context(op, thread, session, carried)
            .await
    }

    pub async fn mark_context_heard(&self, op: &OperationId) -> Result<(), StorageError> {
        self.store.mark_context_heard(op).await
    }

    pub async fn heard_context_digest(
        &self,
        session: &str,
    ) -> Result<Option<String>, StorageError> {
        self.store.heard_context_digest(session).await
    }

    pub async fn context_record(
        &self,
        op: &OperationId,
    ) -> Result<Option<(CarriedContext, bool)>, StorageError> {
        self.store.context_record(op).await
    }
}
