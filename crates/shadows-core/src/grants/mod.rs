//! One job: MCP grants, from issue to revocation (spec §13.7) — the `Grants`
//! service.
//!
//! A person issues and revokes an external agent's project grant over HTTP;
//! `/mcp` asks `authorize` of every request; startup revokes every thread
//! grant an earlier daemon left live. A token is shown once, in the answer
//! that issues it, and stored only as its hash.
//!
//! `model` holds the types, `store` the queries. Both are private: a caller
//! reaches a grant through `Grants` only. The two checks a plan write runs
//! inside its own transaction, `check_writer` and `bind_draft_ref`, are the
//! crate's, declared in `contract.yaml` under `shared_in_transaction`.

mod model;
mod store;

use std::sync::Arc;

pub use model::{Grant, GrantId, GrantKind};
#[cfg(feature = "test-support")]
pub use model::{IssuedGrant, Token, hash_token};
pub(crate) use store::{bind_draft_ref, check_writer};

use crate::app::user_command;
use crate::db::Storage;
use crate::error::CoreError;
use crate::projects::ProjectId;

/// Grants: what storage holds, and the `/mcp` address a `claude mcp add`
/// line names.
pub struct Grants {
    storage: Arc<Storage>,
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
        Self { storage, mcp_url }
    }

    /// A project's grants for external agents, revoked ones included, newest
    /// first. Never a token.
    pub async fn list(&self, project: &ProjectId) -> Result<Vec<Grant>, CoreError> {
        Ok(self.storage.list_project_grants(project).await?)
    }

    /// Connect (§13.7): "McpGrantIssue", params { "project" }. `command` is
    /// the `claude mcp add` line for this daemon's `/mcp`, present only when
    /// the token is.
    pub async fn issue(
        &self,
        command_id: String,
        project: &ProjectId,
    ) -> Result<IssuedView, CoreError> {
        let c = user_command(
            command_id,
            "McpGrantIssue",
            serde_json::json!({ "project": project }),
        );
        let issued = self.storage.issue_project_grant(&c, project).await?;
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

    /// Revoke (§13.7): "McpGrantRevoke", params { "grant" }. A thread grant
    /// is the Planner's, not the person's, and is not found here.
    pub async fn revoke(&self, command_id: String, grant: &GrantId) -> Result<Grant, CoreError> {
        let c = user_command(
            command_id,
            "McpGrantRevoke",
            serde_json::json!({ "grant": grant }),
        );
        Ok(self.storage.revoke_grant(&c, grant).await?)
    }

    /// The bearer check (§13.6): the live grant `token` answers to, found by
    /// its hash; `Ok(None)` for an unknown or revoked token.
    pub async fn authorize(&self, token: &str) -> Result<Option<Grant>, CoreError> {
        Ok(self.storage.grant_for_token(token).await?)
    }

    /// Startup (§13.7): every thread grant still live belongs to an adapter
    /// of an earlier daemon. Answers how many it revoked; project grants are
    /// the person's, and survive.
    pub async fn revoke_thread_grants(&self) -> Result<u64, CoreError> {
        Ok(self.storage.revoke_all_thread_grants().await?)
    }
}
