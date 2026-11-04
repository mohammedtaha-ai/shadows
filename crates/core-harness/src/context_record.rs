//! Prepared planning context and the session that heard it.
use crate::store::Forgot;
use crate::{CarriedContext, Forget, Harness, HarnessError, StorageError, ThreadId};
use core_operations::OperationId;

impl Harness {
    pub async fn mark_context_heard_in(
        conn: &mut core_db::Tx,
        op: &OperationId,
    ) -> Result<(), StorageError> {
        super::store::HarnessStore::mark_context_heard_in(conn, op).await
    }
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

    /// The one rule for "the session lost what it heard": the next turn owes
    /// the planning context and every instructions and standards change since
    /// the session opened. A replayed request answers without writing.
    /// An AYLUL session runs on the same adapter, so it compacts too; it was
    /// given no planning context, so its compaction forgets nothing, and only
    /// a request is refused.
    pub async fn forget_context(
        &self,
        thread: &ThreadId,
        forget: Forget,
    ) -> Result<(), HarnessError> {
        match self.store.forget_context(thread, &forget).await? {
            Forgot::Aylul => match forget {
                Forget::Compaction { .. } => Ok(()),
                Forget::Request { .. } => Err(HarnessError::InvalidCommand(
                    "an AYLUL thread has no planning context".into(),
                )),
            },
            Forgot::Replayed => Ok(()),
            Forgot::Written { fork } => {
                self.sessions.setups.reset(thread, fork);
                Ok(())
            }
        }
    }

    pub async fn context_record(
        &self,
        op: &OperationId,
    ) -> Result<Option<(CarriedContext, bool)>, StorageError> {
        self.store.context_record(op).await
    }
}
