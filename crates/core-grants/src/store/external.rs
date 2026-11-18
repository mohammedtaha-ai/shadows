//! Persistence for external connection credentials and their project scopes.

use core_db::{Actor, CommandContext, JournalEntry, Storage, StorageError, Tx};
use core_projects::{ProjectId, Projects};

use super::{GrantsStore, find_grant, insert_grant, revoke_in};
use crate::{ExternalConnection, ExternalConnectionId, Grant, GrantId, GrantsError, Token};

impl GrantsStore {
    pub(crate) async fn issue_external(
        &self,
        ctx: CommandContext,
        projects: Vec<ProjectId>,
    ) -> Result<(ExternalConnection, Option<Token>), StorageError> {
        let now = Storage::now();
        self.storage.write_txn(move |tx| Box::pin(async move {
            if let Some(id) = CommandContext::classify(tx, &ctx, "Settings", "external-connections").await? {
                return Ok((load(tx, &ExternalConnectionId::from_stored(id)).await?, None));
            }
            let id = ExternalConnectionId::generate();
            let token = Token::generate();
            sqlx::query("INSERT INTO external_connection (id, token_hash, created_at) VALUES (?, ?, ?)")
                .bind(id.as_str()).bind(token.hash()).bind(&now).execute(&mut *tx).await?;
            let actor = Actor::user(&ctx.principal_id);
            for project in &projects {
                Projects::require_live_in(tx, project).await?;
                let (grant, _) = insert_grant(tx, project, None, None, actor.clone(), &now).await?;
                sqlx::query("INSERT INTO external_connection_project (connection_id, project_id, grant_id) VALUES (?, ?, ?)")
                    .bind(id.as_str()).bind(project.as_str()).bind(grant.id.as_str()).execute(&mut *tx).await?;
            }
            let event = JournalEntry::new("ExternalConnectionIssued", actor)
                .with_payload(serde_json::json!({ "connection_id": id, "project_ids": projects }));
            JournalEntry::append_event(tx, event, &now).await?;
            CommandContext::record_command(tx, &ctx, "Settings", "external-connections", "ExternalConnection", id.as_str(), &now).await?;
            Ok((load(tx, &id).await?, Some(token)))
        })).await
    }

    pub(crate) async fn list_external(&self) -> Result<Vec<ExternalConnection>, StorageError> {
        let mut tx = self.storage.reader().begin().await?;
        let ids: Vec<String> = sqlx::query_scalar(
            "SELECT c.id FROM external_connection c
             JOIN durable_event e ON e.kind = 'ExternalConnectionIssued'
                AND json_extract(e.payload_json, '$.connection_id') = c.id
             ORDER BY e.seq DESC",
        )
        .fetch_all(&mut *tx)
        .await?;
        let mut connections = Vec::with_capacity(ids.len());
        for id in ids {
            connections.push(load(&mut tx, &ExternalConnectionId::from_stored(id)).await?);
        }
        Ok(connections)
    }

    pub(crate) async fn revoke_external(
        &self,
        ctx: CommandContext,
        id: ExternalConnectionId,
    ) -> Result<ExternalConnection, StorageError> {
        let now = Storage::now();
        self.storage.write_txn(move |tx| Box::pin(async move {
            if CommandContext::classify(tx, &ctx, "ExternalConnection", id.as_str()).await?.is_some() {
                return load(tx, &id).await;
            }
            let connection = load(tx, &id).await?;
            if connection.revoked_at.is_none() {
                sqlx::query("UPDATE external_connection SET revoked_at = ? WHERE id = ?")
                    .bind(&now).bind(id.as_str()).execute(&mut *tx).await?;
                let scopes: Vec<String> = sqlx::query_scalar(
                    "SELECT grant_id FROM external_connection_project WHERE connection_id = ? ORDER BY project_id",
                ).bind(id.as_str()).fetch_all(&mut *tx).await?;
                let actor = Actor::user(&ctx.principal_id);
                for scope in scopes {
                    let grant = find_grant(tx, &GrantId::from_stored(scope)).await?
                        .ok_or(StorageError::NotFound("connection project grant"))?;
                    revoke_in(tx, &grant, actor.clone(), &now).await?;
                }
                let event = JournalEntry::new("ExternalConnectionRevoked", actor)
                    .with_payload(serde_json::json!({ "connection_id": id }));
                JournalEntry::append_event(tx, event, &now).await?;
            }
            CommandContext::record_command(tx, &ctx, "ExternalConnection", id.as_str(), "ExternalConnection", id.as_str(), &now).await?;
            load(tx, &id).await
        })).await
    }

    pub(crate) async fn authorize_external(
        &self,
        raw: &str,
    ) -> Result<Option<ExternalConnection>, StorageError> {
        let mut tx = self.storage.reader().begin().await?;
        let id: Option<String> = sqlx::query_scalar(
            "SELECT id FROM external_connection WHERE token_hash = ? AND revoked_at IS NULL",
        )
        .bind(Token::hash_token(raw))
        .fetch_optional(&mut *tx)
        .await?;
        match id {
            Some(id) => Ok(Some(
                load(&mut tx, &ExternalConnectionId::from_stored(id)).await?,
            )),
            None => Ok(None),
        }
    }

    pub(crate) async fn external_project(
        &self,
        id: &ExternalConnectionId,
        project: &ProjectId,
    ) -> Result<Grant, GrantsError> {
        let mut tx = self.storage.reader().begin().await?;
        let grant: Option<String> = sqlx::query_scalar(
            "SELECT p.grant_id FROM external_connection_project p
             JOIN external_connection c ON c.id = p.connection_id
             WHERE c.id = ? AND p.project_id = ? AND c.revoked_at IS NULL",
        )
        .bind(id.as_str())
        .bind(project.as_str())
        .fetch_optional(&mut *tx)
        .await?;
        let grant = grant.ok_or(GrantsError::GrantScope)?;
        if !Projects::is_live_in(&mut tx, project).await? {
            return Err(GrantsError::GrantScope);
        }
        find_grant(&mut tx, &GrantId::from_stored(grant))
            .await?
            .filter(|grant| grant.revoked_at.is_none())
            .ok_or(GrantsError::GrantInvalid)
    }
}

impl GrantsStore {
    pub(crate) async fn connection_scope_in(
        tx: &mut Tx,
        grant: &GrantId,
    ) -> Result<bool, StorageError> {
        Ok(sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM external_connection_project WHERE grant_id = ?)",
        )
        .bind(grant.as_str())
        .fetch_one(tx)
        .await?)
    }
}

async fn load(tx: &mut Tx, id: &ExternalConnectionId) -> Result<ExternalConnection, StorageError> {
    let (created_at, revoked_at) =
        sqlx::query_as("SELECT created_at, revoked_at FROM external_connection WHERE id = ?")
            .bind(id.as_str())
            .fetch_optional(&mut *tx)
            .await?
            .ok_or(StorageError::NotFound("external connection"))?;
    let projects: Vec<String> = sqlx::query_scalar(
        "SELECT project_id FROM external_connection_project WHERE connection_id = ? ORDER BY project_id",
    ).bind(id.as_str()).fetch_all(&mut *tx).await?;
    Ok(ExternalConnection {
        id: id.clone(),
        project_ids: projects.into_iter().map(ProjectId::from_recorded).collect(),
        created_at,
        revoked_at,
    })
}
