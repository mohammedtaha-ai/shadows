//! Durable eligibility for a conversation fork.
use crate::{Operations, OperationsError};
use core_threads::{ThreadEntryId, ThreadId, Threads};
use sqlx::SqliteConnection;

impl Operations {
    pub async fn fork_eligible_in(
        conn: &mut SqliteConnection,
        source: &ThreadId,
        at: &ThreadEntryId,
    ) -> Result<bool, OperationsError> {
        Threads::require_live_in(conn, source).await?;
        if Self::has_open_in(conn, source).await? {
            return Err(OperationsError::ThreadBusy);
        }
        match Threads::fork_operation_in(conn, source, at).await? {
            Some(operation) => Ok(Self::completed_in_snapshot(conn, &operation).await?),
            None => Ok(false),
        }
    }
}
