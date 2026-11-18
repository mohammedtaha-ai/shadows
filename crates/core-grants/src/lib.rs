//! Grant authorization, approval provenance and draft references.

mod approver;
mod error;
mod model;
mod writer;
pub use approver::{ApprovalRequest, ApprovedBy, Approver};
pub use error::GrantsError;
pub use writer::Writer;
mod approval;
mod external;
mod store;
#[cfg(feature = "test-support")]
mod testing;
mod tools;

use std::sync::Arc;

pub use external::{ExternalConnection, ExternalConnectionId, IssuedExternalConnection};
#[cfg(feature = "test-support")]
pub use model::IssuedGrant;
pub use model::{Grant, GrantId, GrantKind, TaskScope, Token};

use core_db::Storage;
use core_projects::ProjectId;

/// Grants: what storage holds, and the `/mcp` address a `claude mcp add`
/// line names.
#[derive(Clone)]
pub struct Grants {
    store: store::GrantsStore,
    /// `http://<bound address>/mcp`.
    mcp_url: String,
}

impl Grants {
    pub fn new(storage: Arc<Storage>, mcp_url: String) -> Self {
        Self {
            store: store::GrantsStore::with_storage(storage),
            mcp_url,
        }
    }

    /// Resolves a bearer by its hash; unknown or revoked tokens return none.
    pub async fn authorize(&self, token: &str) -> Result<Option<Grant>, GrantsError> {
        Ok(self.store.grant_for_token(token).await?)
    }
    /// Whether `grant`, as a request authenticated it, is still the live grant
    /// `tx` reads: a caller rechecks it after awaited work, inside its own read
    /// or write.
    pub async fn check_live_in(tx: &mut core_db::Tx, grant: &Grant) -> Result<(), GrantsError> {
        store::GrantsStore::check_live_in(tx, grant).await
    }
    /// Revokes every live thread and executor grant during startup; project grants survive.
    pub async fn revoke_thread_grants(&self) -> Result<u64, GrantsError> {
        Ok(self.store.revoke_all_thread_grants().await?)
    }
}

impl Grants {
    pub async fn issue_thread_grant_in(
        conn: &mut sqlx::SqliteConnection,
        project: &ProjectId,
        thread: &core_threads::ThreadId,
    ) -> Result<(Grant, model::Token), core_db::StorageError> {
        store::GrantsStore::issue_thread_grant_in(conn, project, thread).await
    }
    /// The grant of one executor session, bound to one task.
    pub async fn issue_executor_grant_in(
        conn: &mut sqlx::SqliteConnection,
        project: &ProjectId,
        thread: &core_threads::ThreadId,
        scope: TaskScope,
    ) -> Result<(Grant, model::Token), core_db::StorageError> {
        store::GrantsStore::issue_executor_grant_in(conn, project, thread, scope).await
    }
    pub async fn revoke_thread_grant(&self, id: &GrantId) -> Result<(), core_db::StorageError> {
        self.store.revoke_thread_grant(id).await
    }
    pub async fn prepare_draft(&self, grant: &GrantId) -> Result<String, GrantsError> {
        self.store.prepare_draft(grant).await
    }
}
