//! External connections issue one credential for an explicit set of projects.

use core_db::{CommandContext, newtype_id};
use core_projects::ProjectId;

use crate::{Grant, Grants, GrantsError};

newtype_id! {
    /// Identity of one external MCP connection.
    ExternalConnectionId
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, utoipa::ToSchema)]
pub struct ExternalConnection {
    pub id: ExternalConnectionId,
    pub project_ids: Vec<ProjectId>,
    pub created_at: String,
    pub revoked_at: Option<String>,
}

pub struct IssuedExternalConnection {
    pub connection: ExternalConnection,
    pub token: Option<String>,
    pub mcp_url: String,
}

impl Grants {
    pub async fn issue_external(
        &self,
        command_id: String,
        mut project_ids: Vec<ProjectId>,
    ) -> Result<IssuedExternalConnection, GrantsError> {
        project_ids.sort_by(|left, right| left.as_str().cmp(right.as_str()));
        project_ids.dedup();
        if project_ids.is_empty() {
            return Err(GrantsError::GrantScopeMessage(
                "Select at least one project.".into(),
            ));
        }
        let ctx = CommandContext::user(
            command_id,
            "ExternalConnectionIssue",
            serde_json::json!({ "project_ids": project_ids }),
        );
        let (connection, token) = self.store.issue_external(ctx, project_ids).await?;
        Ok(IssuedExternalConnection {
            connection,
            token: token.map(|token| token.as_str().into()),
            mcp_url: self.mcp_url.clone(),
        })
    }

    pub async fn list_external(&self) -> Result<Vec<ExternalConnection>, GrantsError> {
        Ok(self.store.list_external().await?)
    }

    pub async fn revoke_external(
        &self,
        command_id: String,
        id: &ExternalConnectionId,
    ) -> Result<ExternalConnection, GrantsError> {
        let ctx = CommandContext::user(
            command_id,
            "ExternalConnectionRevoke",
            serde_json::json!({ "connection_id": id }),
        );
        Ok(self.store.revoke_external(ctx, id.clone()).await?)
    }

    pub async fn authorize_external(
        &self,
        token: &str,
    ) -> Result<Option<ExternalConnection>, GrantsError> {
        Ok(self.store.authorize_external(token).await?)
    }

    pub async fn external_project(
        &self,
        connection: &ExternalConnectionId,
        project: &ProjectId,
    ) -> Result<Grant, GrantsError> {
        self.store.external_project(connection, project).await
    }
}
