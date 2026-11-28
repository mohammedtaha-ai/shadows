//! Persistence for external connection credentials and their project scopes.

use core_db::{Actor, CommandContext, JournalEntry, Storage, StorageError, Tx};
use core_projects::{ProjectId, Projects};
use std::collections::{HashMap, HashSet};

use super::{GrantsStore, find_grant, insert_grant, revoke_in};
use crate::{ExternalConnection, ExternalConnectionId, Grant, GrantId, GrantsError, Token};

impl GrantsStore {
    pub(crate) async fn connections_of_in(
        conn: &mut sqlx::SqliteConnection,
        grants: &[GrantId],
    ) -> Result<HashMap<GrantId, String>, StorageError> {
        let mut connections = HashMap::new();
        for grant in grants.iter().collect::<HashSet<_>>() {
            let connection = sqlx::query_scalar(
                "SELECT connection_id FROM external_connection_project WHERE grant_id=?",
            )
            .bind(grant.as_str())
            .fetch_optional(&mut *conn)
            .await?;
            if let Some(connection) = connection {
                connections.insert(grant.clone(), connection);
            }
        }
        Ok(connections)
    }

    pub(crate) async fn issue_external(
        &self,
        ctx: CommandContext,
        name: Option<String>,
        projects: Vec<ProjectId>,
    ) -> Result<(ExternalConnection, Option<Token>), StorageError> {
        let now = Storage::now();
        self.storage
            .write_txn(move |tx| {
                Box::pin(async move { issue_in(tx, &ctx, name.as_deref(), &projects, &now).await })
            })
            .await
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
        self.storage
            .write_txn(move |tx| Box::pin(async move { revoke_in_tx(tx, &ctx, &id, &now).await }))
            .await
    }

    pub(crate) async fn update_external_projects(
        &self,
        ctx: CommandContext,
        id: ExternalConnectionId,
        expected_revision: i64,
        projects: Vec<ProjectId>,
    ) -> Result<ExternalConnection, StorageError> {
        let now = Storage::now();
        self.storage
            .write_txn(move |tx| {
                Box::pin(async move {
                    update_projects_in(tx, &ctx, &id, expected_revision, &projects, &now).await
                })
            })
            .await
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

const ISSUE_SCOPE: (&str, &str) = ("Settings", "external-connections");
const CONNECTION: &str = "ExternalConnection";

async fn issue_in(
    tx: &mut Tx,
    ctx: &CommandContext,
    name: Option<&str>,
    projects: &[ProjectId],
    now: &str,
) -> Result<(ExternalConnection, Option<Token>), StorageError> {
    let (kind, scope) = ISSUE_SCOPE;
    if let Some(id) = CommandContext::classify(tx, ctx, kind, scope).await? {
        return Ok((
            load(tx, &ExternalConnectionId::from_stored(id)).await?,
            None,
        ));
    }
    let id = ExternalConnectionId::generate();
    let token = Token::generate();
    sqlx::query(
        "INSERT INTO external_connection (id, token_hash, created_at, name) VALUES (?, ?, ?, ?)",
    )
    .bind(id.as_str())
    .bind(token.hash())
    .bind(now)
    .bind(name)
    .execute(&mut *tx)
    .await?;
    let actor = Actor::user(&ctx.principal_id);
    for project in projects {
        add_project(tx, &id, project, actor.clone(), now).await?;
    }
    let payload = serde_json::json!({ "connection_id": id, "project_ids": projects });
    let event = JournalEntry::new("ExternalConnectionIssued", actor).with_payload(payload);
    JournalEntry::append_event(tx, event, now).await?;
    CommandContext::record_command(tx, ctx, kind, scope, CONNECTION, id.as_str(), now).await?;
    Ok((load(tx, &id).await?, Some(token)))
}

async fn add_project(
    tx: &mut Tx,
    id: &ExternalConnectionId,
    project: &ProjectId,
    actor: Actor,
    now: &str,
) -> Result<(), StorageError> {
    Projects::require_live_in(tx, project).await?;
    let (grant, _) = insert_grant(tx, project, None, None, actor, now).await?;
    sqlx::query(
        "INSERT INTO external_connection_project (connection_id, project_id, grant_id)
         VALUES (?, ?, ?)",
    )
    .bind(id.as_str())
    .bind(project.as_str())
    .bind(grant.id.as_str())
    .execute(&mut *tx)
    .await?;
    Ok(())
}

async fn update_projects_in(
    tx: &mut Tx,
    ctx: &CommandContext,
    id: &ExternalConnectionId,
    expected_revision: i64,
    projects: &[ProjectId],
    now: &str,
) -> Result<ExternalConnection, StorageError> {
    let raw = id.as_str();
    if CommandContext::classify(tx, ctx, CONNECTION, raw)
        .await?
        .is_some()
    {
        return load(tx, id).await;
    }
    let connection = load(tx, id).await?;
    if connection.revoked_at.is_some() {
        return Err(StorageError::TransitionConflict {
            expected: "active connection".into(),
            found: "revoked connection".into(),
        });
    }
    if connection.project_revision != expected_revision {
        return Err(StorageError::RevisionConflict {
            current: connection.project_revision,
            summary: "allowed projects".into(),
        });
    }
    if connection.project_ids != projects {
        let actor = Actor::user(&ctx.principal_id);
        for project in &connection.project_ids {
            if !projects.contains(project) {
                remove_project(tx, id, project, actor.clone(), now).await?;
            }
        }
        for project in projects {
            if !connection.project_ids.contains(project) {
                add_project(tx, id, project, actor.clone(), now).await?;
            }
        }
        sqlx::query(
            "UPDATE external_connection SET project_revision = project_revision + 1 WHERE id = ?",
        )
        .bind(raw)
        .execute(&mut *tx)
        .await?;
        let event = JournalEntry::new("ExternalConnectionProjectsUpdated", actor)
            .with_payload(serde_json::json!({ "connection_id": id, "project_ids": projects }));
        JournalEntry::append_event(tx, event, now).await?;
    }
    CommandContext::record_command(tx, ctx, CONNECTION, raw, CONNECTION, raw, now).await?;
    load(tx, id).await
}

async fn remove_project(
    tx: &mut Tx,
    id: &ExternalConnectionId,
    project: &ProjectId,
    actor: Actor,
    now: &str,
) -> Result<(), StorageError> {
    let scope: String = sqlx::query_scalar(
        "DELETE FROM external_connection_project WHERE connection_id = ? AND project_id = ?
         RETURNING grant_id",
    )
    .bind(id.as_str())
    .bind(project.as_str())
    .fetch_one(&mut *tx)
    .await?;
    revoke_scope(tx, scope, actor, now).await
}

async fn revoke_scope(
    tx: &mut Tx,
    scope: String,
    actor: Actor,
    now: &str,
) -> Result<(), StorageError> {
    let grant = find_grant(tx, &GrantId::from_stored(scope))
        .await?
        .ok_or(StorageError::NotFound("connection project grant"))?;
    revoke_in(tx, &grant, actor, now).await
}

async fn revoke_in_tx(
    tx: &mut Tx,
    ctx: &CommandContext,
    id: &ExternalConnectionId,
    now: &str,
) -> Result<ExternalConnection, StorageError> {
    if CommandContext::classify(tx, ctx, CONNECTION, id.as_str())
        .await?
        .is_some()
    {
        return load(tx, id).await;
    }
    let connection = load(tx, id).await?;
    if connection.revoked_at.is_none() {
        sqlx::query("UPDATE external_connection SET revoked_at = ? WHERE id = ?")
            .bind(now)
            .bind(id.as_str())
            .execute(&mut *tx)
            .await?;
        let scopes: Vec<String> = sqlx::query_scalar(
            "SELECT grant_id FROM external_connection_project
             WHERE connection_id = ? ORDER BY project_id",
        )
        .bind(id.as_str())
        .fetch_all(&mut *tx)
        .await?;
        let actor = Actor::user(&ctx.principal_id);
        for scope in scopes {
            revoke_scope(tx, scope, actor.clone(), now).await?;
        }
        let event = JournalEntry::new("ExternalConnectionRevoked", actor)
            .with_payload(serde_json::json!({ "connection_id": id }));
        JournalEntry::append_event(tx, event, now).await?;
    }
    let raw = id.as_str();
    CommandContext::record_command(tx, ctx, CONNECTION, raw, CONNECTION, raw, now).await?;
    load(tx, id).await
}

async fn load(tx: &mut Tx, id: &ExternalConnectionId) -> Result<ExternalConnection, StorageError> {
    let (created_at, revoked_at, name, project_revision) = sqlx::query_as(
        "SELECT created_at, revoked_at, name, project_revision
         FROM external_connection WHERE id = ?",
    )
    .bind(id.as_str())
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(StorageError::NotFound("external connection"))?;
    let projects: Vec<String> = sqlx::query_scalar(
        "SELECT project_id FROM external_connection_project
         WHERE connection_id = ? ORDER BY project_id",
    )
    .bind(id.as_str())
    .fetch_all(&mut *tx)
    .await?;
    Ok(ExternalConnection {
        id: id.clone(),
        name,
        project_ids: projects.into_iter().map(ProjectId::from_recorded).collect(),
        project_revision,
        created_at,
        revoked_at,
        wait_url: String::new(),
    })
}
