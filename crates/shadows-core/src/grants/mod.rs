//! Grant authorization, approval provenance and draft references.

mod approver;
mod error;
mod model;
mod writer;
pub use approver::{ApprovalRequest, ApprovedBy, Approver};
pub use error::GrantsError;
pub use writer::Writer;
mod approval;
mod store;
#[cfg(feature = "test-support")]
mod testing;
mod tools;

use std::sync::Arc;

pub use model::{Grant, GrantId, GrantKind};
#[cfg(feature = "test-support")]
pub use model::{IssuedGrant, Token};

use crate::error::CoreError;
use crate::projects::ProjectId;
use core_db::{CommandContext, Storage};

/// Grants: what storage holds, and the `/mcp` address a `claude mcp add`
/// line names.
#[derive(Clone)]
pub struct Grants {
    store: store::GrantsStore,
    /// `http://<bound address>/mcp`.
    mcp_url: String,
}

/// What issuing answers. `token` and `command` are `Some` on the first issue
/// only: a replay of the same command answers the grant without them,
/// because a token is shown once.
pub struct IssuedView {
    pub grant: Grant,
    pub token: Option<String>,
    pub command: Option<String>,
}

impl Grants {
    pub(crate) fn new(storage: Arc<Storage>, mcp_url: String) -> Self {
        Self {
            store: store::GrantsStore::with_storage(storage),
            mcp_url,
        }
    }

    /// A project's grants for external agents, revoked ones included, newest
    /// first. Never a token.
    pub async fn list(&self, project: &ProjectId) -> Result<Vec<Grant>, CoreError> {
        Ok(self.store.list_project_grants(project).await?)
    }
    /// Issues the grant with the `claude mcp add` line for this daemon's `/mcp`, present only when
    /// the token is.
    pub async fn issue(
        &self,
        command_id: String,
        project: &ProjectId,
    ) -> Result<IssuedView, CoreError> {
        let c = CommandContext::user(
            command_id,
            "McpGrantIssue",
            serde_json::json!({ "project": project }),
        );
        let issued = self.store.issue_project_grant(&c, project).await?;
        let command = issued.token.as_ref().map(|token| {
            format!(
                "claude mcp add --transport http shadows {} --header \"Authorization: Bearer {}\"",
                self.mcp_url,
                token.as_str()
            )
        });
        Ok(IssuedView {
            grant: issued.grant,
            token: issued.token.map(|t| t.as_str().to_string()),
            command,
        })
    }
    /// A thread grant is the Planner's, not the person's, and is not found here.
    pub async fn revoke(&self, command_id: String, grant: &GrantId) -> Result<Grant, CoreError> {
        let c = CommandContext::user(
            command_id,
            "McpGrantRevoke",
            serde_json::json!({ "grant": grant }),
        );
        Ok(self.store.revoke_grant(&c, grant).await?)
    }
    /// Resolves a bearer by its hash; unknown or revoked tokens return none.
    pub async fn authorize(&self, token: &str) -> Result<Option<Grant>, CoreError> {
        Ok(self.store.grant_for_token(token).await?)
    }
    /// Revokes every live thread grant during startup; project grants survive.
    pub async fn revoke_thread_grants(&self) -> Result<u64, CoreError> {
        Ok(self.store.revoke_all_thread_grants().await?)
    }
}

impl Grants {
    pub(crate) async fn issue_thread_grant_in(
        conn: &mut sqlx::SqliteConnection,
        project: &ProjectId,
        thread: &crate::threads::ThreadId,
    ) -> Result<(Grant, model::Token), core_db::StorageError> {
        store::GrantsStore::issue_thread_grant_in(conn, project, thread).await
    }
    pub(crate) async fn revoke_thread_grant(
        &self,
        id: &GrantId,
    ) -> Result<(), core_db::StorageError> {
        self.store.revoke_thread_grant(id).await
    }
    pub async fn prepare_draft(&self, grant: &GrantId) -> Result<String, CoreError> {
        Ok(self.store.prepare_draft(grant).await?)
    }
}
