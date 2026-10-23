//! Durable eligibility for a conversation fork.
use crate::CoreError;
use crate::Operations;
use crate::threads::{ThreadEntryId, ThreadId, Threads};
use sqlx::SqliteConnection;

impl Operations {
    pub(crate) async fn fork_eligible_in(
        conn: &mut SqliteConnection,
        source: &ThreadId,
        at: &ThreadEntryId,
    ) -> Result<bool, CoreError> {
        Threads::require_live_in(conn, source).await?;
        if Self::has_open_in(conn, source).await? {
            return Err(CoreError::Refused {
                code: crate::ErrorCode::ThreadBusy,
                message: "the thread has a turn running".into(),
            });
        }
        match Threads::fork_operation_in(conn, source, at).await? {
            Some(operation) => Ok(Self::completed_in_snapshot(conn, &operation).await?),
            None => Ok(false),
        }
    }
}
