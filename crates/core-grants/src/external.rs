//! External connections issue one credential for an explicit set of projects.

use core_db::{CommandContext, newtype_id};
use core_projects::ProjectId;
use std::collections::HashMap;

use crate::{Grant, GrantId, Grants, GrantsError};

newtype_id! {
    /// Identity of one external MCP connection.
    ExternalConnectionId
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, utoipa::ToSchema)]
pub struct ExternalConnection {
    pub id: ExternalConnectionId,
    #[schema(required = true)]
    pub name: Option<String>,
    pub project_ids: Vec<ProjectId>,
    pub project_revision: i64,
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
        name: Option<String>,
        project_ids: Vec<ProjectId>,
    ) -> Result<IssuedExternalConnection, GrantsError> {
        let project_ids = normalized_projects(project_ids);
        let name = name.map(|value| value.trim().to_owned());
        if name.as_deref() == Some("") {
            return Err(GrantsError::GrantScopeMessage(
                "Enter a connection name.".into(),
            ));
        }
        if project_ids.is_empty() {
            return Err(GrantsError::GrantScopeMessage(
                "Select at least one project.".into(),
            ));
        }
        let mut params = serde_json::json!({ "project_ids": project_ids });
        // Keep command fingerprints valid for nameless issues made by older clients.
        if let Some(name) = &name {
            params["name"] = serde_json::json!(name);
        }
        let ctx = CommandContext::user(command_id, "ExternalConnectionIssue", params);
        let (connection, token) = self.store.issue_external(ctx, name, project_ids).await?;
        Ok(IssuedExternalConnection {
            connection,
            token: token.map(|token| token.as_str().into()),
            mcp_url: self.mcp_url.clone(),
        })
    }

    pub async fn list_external(&self) -> Result<Vec<ExternalConnection>, GrantsError> {
        Ok(self.store.list_external().await?)
    }

    pub async fn update_external_projects(
        &self,
        command_id: String,
        id: &ExternalConnectionId,
        expected_revision: i64,
        project_ids: Vec<ProjectId>,
    ) -> Result<ExternalConnection, GrantsError> {
        let project_ids = normalized_projects(project_ids);
        let ctx = CommandContext::user(
            command_id,
            "ExternalConnectionProjectsUpdate",
            serde_json::json!({
                "connection_id": id, "expected_revision": expected_revision,
                "project_ids": project_ids
            }),
        );
        Ok(self
            .store
            .update_external_projects(ctx, id.clone(), expected_revision, project_ids)
            .await?)
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

    /// Identifies the connections of the recorded grants in the caller's snapshot.
    pub async fn connections_of_in(
        conn: &mut sqlx::SqliteConnection,
        grants: &[GrantId],
    ) -> Result<HashMap<GrantId, String>, GrantsError> {
        Ok(super::store::GrantsStore::connections_of_in(conn, grants).await?)
    }
}

fn normalized_projects(mut projects: Vec<ProjectId>) -> Vec<ProjectId> {
    projects.sort_by(|left, right| left.as_str().cmp(right.as_str()));
    projects.dedup();
    projects
}
