//! Prepared planning context and the session that heard it.
use crate::{CarriedContext, Forget, Harness, HarnessError, StorageError, ThreadId};
use core_operations::OperationId;
use shadows_agent::policy;

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
    pub async fn forget_context(
        &self,
        thread: &ThreadId,
        forget: Forget,
    ) -> Result<(), HarnessError> {
        let context =
            core_threads::Threads::turn_context_in(&mut *self.store.reader_conn().await?, thread)
                .await?;
        if context.harness == policy::AYLUL {
            return Err(HarnessError::InvalidCommand(
                "an AYLUL thread has no planning context".into(),
            ));
        }
        if self.store.forget_context(thread, &forget).await? {
            self.sessions
                .setups
                .reset(thread, context.fork_session_id.is_some());
        }
        Ok(())
    }

    pub async fn context_record(
        &self,
        op: &OperationId,
    ) -> Result<Option<(CarriedContext, bool)>, StorageError> {
        self.store.context_record(op).await
    }
}
