//! Grant persistence over the shared serialized database handle.
use sqlx::SqliteConnection;

use super::model::{Grant, GrantId, GrantKind, IssuedGrant, Token};
use super::{Grants, GrantsError, Writer};
use crate::events::DurableEvent;
use crate::projects::ProjectId;
use crate::threads::ThreadId;
use core_db::{Actor, CommandContext, JournalEntry, Storage, StorageError};
use std::sync::Arc;

const DRAFT_REF_LIFETIME: time::Duration = time::Duration::HOUR;

#[derive(Clone)]
pub(super) struct GrantsStore {
    storage: Arc<Storage>,
}

impl GrantsStore {
    pub(super) fn with_storage(storage: Arc<Storage>) -> Self {
        Self { storage }
    }
    pub(super) async fn issue_project_grant(
        &self,
        ctx: &CommandContext,
        project: &ProjectId,
    ) -> Result<IssuedGrant, StorageError> {
        let (ctx, project, ts) = (ctx.clone(), project.clone(), Storage::now());
        self.storage
            .write_txn(move |conn| {
                Box::pin(async move {
                    if let Some(id) =
                        CommandContext::classify(conn, &ctx, "Project", project.as_str()).await?
                    {
                        let grant = load_grant(conn, &GrantId::from_stored(id)).await?;
                        return Ok(IssuedGrant { grant, token: None });
                    }
                    crate::projects::Projects::require_live_in(conn, &project).await?;
                    let actor = Actor::user(&ctx.principal_id);
                    let (grant, token) = insert_grant(conn, &project, None, actor, &ts).await?;
                    CommandContext::record_command(
                        conn,
                        &ctx,
                        "Project",
                        project.as_str(),
                        "McpGrant",
                        grant.id.as_str(),
                        &ts,
                    )
                    .await?;
                    Ok(IssuedGrant {
                        grant,
                        token: Some(token),
                    })
                })
            })
            .await
    }
    pub(super) async fn issue_thread_grant_in(
        conn: &mut SqliteConnection,
        project: &ProjectId,
        thread: &ThreadId,
    ) -> Result<(Grant, Token), StorageError> {
        insert_grant(
            conn,
            project,
            Some(thread),
            Actor::system(),
            &Storage::now(),
        )
        .await
    }
    pub(super) async fn revoke_grant(
        &self,
        ctx: &CommandContext,
        id: &GrantId,
    ) -> Result<Grant, StorageError> {
        let (ctx, id, ts) = (ctx.clone(), id.clone(), Storage::now());
        self.storage
            .write_txn(move |conn| {
                Box::pin(async move {
                    if CommandContext::classify(conn, &ctx, "Grant", id.as_str())
                        .await?
                        .is_some()
                    {
                        return load_grant(conn, &id).await;
                    }
                    let grant = find_grant(conn, &id)
                        .await?
                        .filter(|g| g.kind == GrantKind::Project)
                        .ok_or(StorageError::NotFound("project grant"))?;
                    revoke_in(conn, &grant, Actor::user(&ctx.principal_id), &ts).await?;
                    let (scope, key) = ("Grant", id.as_str());
                    CommandContext::record_command(conn, &ctx, scope, key, "McpGrant", key, &ts)
                        .await?;
                    load_grant(conn, &id).await
                })
            })
            .await
    }
    pub(super) async fn revoke_thread_grant(&self, id: &GrantId) -> Result<(), StorageError> {
        let (id, ts) = (id.clone(), Storage::now());
        self.storage
            .write_txn(move |conn| {
                Box::pin(async move {
                    let grant = find_grant(conn, &id)
                        .await?
                        .filter(|g| g.kind == GrantKind::Thread)
                        .ok_or(StorageError::NotFound("thread grant"))?;
                    revoke_in(conn, &grant, Actor::system(), &ts).await
                })
            })
            .await
    }
    pub(super) async fn revoke_all_thread_grants(&self) -> Result<u64, StorageError> {
        let ts = Storage::now();
        self.storage
            .write_txn(move |conn| {
                Box::pin(async move {
                    let live: Vec<GrantRow> = sqlx::query_as(
                        "SELECT id, kind, project_id, thread_id, created_at, revoked_at \
                        FROM mcp_grant\n                      \
                        WHERE kind = 'thread' AND revoked_at IS NULL ORDER BY id",
                    )
                    .fetch_all(&mut *conn)
                    .await?;
                    let mut revoked = 0;
                    for row in live {
                        revoke_in(conn, &into_grant(row)?, Actor::system(), &ts).await?;
                        revoked += 1;
                    }
                    Ok(revoked)
                })
            })
            .await
    }
    pub(super) async fn grant_for_token(&self, raw: &str) -> Result<Option<Grant>, StorageError> {
        let row: Option<GrantRow> = sqlx::query_as(
            "SELECT id, kind, project_id, thread_id, created_at, revoked_at FROM mcp_grant \
            WHERE token_hash = ? AND revoked_at IS NULL",
        )
        .bind(Token::hash_token(raw))
        .fetch_optional(self.storage.reader())
        .await?;
        row.map(into_grant).transpose()
    }
    pub(super) async fn list_project_grants(
        &self,
        project: &ProjectId,
    ) -> Result<Vec<Grant>, StorageError> {
        let mut snapshot = self.storage.reader().begin().await?;
        match crate::projects::Projects::require_live_in(&mut snapshot, project).await {
            Ok(()) => {}
            Err(StorageError::NotFound(_)) => return Ok(Vec::new()),
            Err(error) => return Err(error),
        }
        let rows: Vec<GrantRow> = sqlx::query_as(
            "SELECT g.id, g.kind, g.project_id, g.thread_id, g.created_at, g.revoked_at
               FROM mcp_grant g
               JOIN durable_event e
                 ON e.kind = 'McpGrantIssued' AND e.project_id = g.project_id
                AND json_extract(e.payload_json, '$.grant_id') = g.id
              WHERE g.project_id = ? AND g.kind = 'project'
              ORDER BY e.seq DESC",
        )
        .bind(project.as_str())
        .fetch_all(&mut *snapshot)
        .await?;
        rows.into_iter().map(into_grant).collect()
    }
    pub(super) async fn prepare_draft(&self, grant: &GrantId) -> Result<String, GrantsError> {
        let grant = grant.clone();
        let issued = time::OffsetDateTime::now_utc();
        let (created, expires) = (rfc3339(issued), rfc3339(issued + DRAFT_REF_LIFETIME));
        self.storage
            .write_txn_as(move |conn| {
                Box::pin(async move {
                    match find_grant(conn, &grant).await? {
                        Some(g) if g.revoked_at.is_none() && g.kind == GrantKind::Project => {}
                        Some(g) if g.revoked_at.is_none() => return Err(GrantsError::GrantScope),
                        _ => return Err(GrantsError::GrantInvalid),
                    }
                    let draft_ref = format!("dr-{}", uuid::Uuid::new_v4());
                    sqlx::query(
                        "INSERT INTO draft_intent (draft_ref, grant_id, created_at, expires_at)
                     VALUES (?,?,?,?)",
                    )
                    .bind(&draft_ref)
                    .bind(grant.as_str())
                    .bind(&created)
                    .bind(&expires)
                    .execute(&mut *conn)
                    .await?;
                    Ok(draft_ref)
                })
            })
            .await
    }
}

type GrantRow = (
    String,
    String,
    String,
    Option<String>,
    String,
    Option<String>,
);

fn into_grant(
    (id, kind, project, thread, created_at, revoked_at): GrantRow,
) -> Result<Grant, StorageError> {
    let kind = match kind.as_str() {
        "thread" => GrantKind::Thread,
        "project" => GrantKind::Project,
        _ => return Err(StorageError::Constraint(format!("mcp_grant.kind {kind}"))),
    };
    Ok(Grant {
        id: GrantId::from_stored(id),
        kind,
        project_id: ProjectId::from_stored(project),
        thread_id: thread.map(ThreadId::from_stored),
        created_at,
        revoked_at,
    })
}

async fn find_grant(
    conn: &mut SqliteConnection,
    id: &GrantId,
) -> Result<Option<Grant>, StorageError> {
    let row: Option<GrantRow> = sqlx::query_as(
        "SELECT id, kind, project_id, thread_id, created_at, revoked_at FROM mcp_grant WHERE \
        id = ?",
    )
    .bind(id.as_str())
    .fetch_optional(&mut *conn)
    .await?;
    row.map(into_grant).transpose()
}

async fn load_grant(conn: &mut SqliteConnection, id: &GrantId) -> Result<Grant, StorageError> {
    find_grant(conn, id)
        .await?
        .ok_or(StorageError::NotFound("grant"))
}
async fn insert_grant(
    conn: &mut SqliteConnection,
    project: &ProjectId,
    thread: Option<&ThreadId>,
    actor: Actor,
    ts: &str,
) -> Result<(Grant, Token), StorageError> {
    let kind = match thread {
        Some(_) => GrantKind::Thread,
        None => GrantKind::Project,
    };
    let (id, token) = (GrantId::generate(), Token::generate());
    sqlx::query(
        "INSERT INTO mcp_grant (id, kind, thread_id, project_id, token_hash, created_at)
         VALUES (?,?,?,?,?,?)",
    )
    .bind(id.as_str())
    .bind(kind.as_str())
    .bind(thread.map(ThreadId::as_str))
    .bind(project.as_str())
    .bind(token.hash())
    .bind(ts)
    .execute(&mut *conn)
    .await?;
    let payload = serde_json::json!({ "grant_id": id, "kind": kind.as_str() });
    let event = DurableEvent::new("McpGrantIssued", actor).with_project(project);
    let event = match thread {
        Some(thread) => event.with_thread(thread),
        None => event,
    };
    JournalEntry::append_event(conn, event.with_payload(payload).journal_entry(), ts).await?;
    Ok((load_grant(conn, &id).await?, token))
}
async fn revoke_in(
    conn: &mut SqliteConnection,
    grant: &Grant,
    actor: Actor,
    ts: &str,
) -> Result<(), StorageError> {
    let revoked =
        sqlx::query("UPDATE mcp_grant SET revoked_at = ? WHERE id = ? AND revoked_at IS NULL")
            .bind(ts)
            .bind(grant.id.as_str())
            .execute(&mut *conn)
            .await?;
    if revoked.rows_affected() == 0 {
        return Ok(());
    }
    let payload = serde_json::json!({ "grant_id": grant.id });
    let event = DurableEvent::new("McpGrantRevoked", actor).with_project(&grant.project_id);
    let event = match &grant.thread_id {
        Some(thread) => event.with_thread(thread),
        None => event,
    };
    JournalEntry::append_event(conn, event.with_payload(payload).journal_entry(), ts)
        .await
        .map(|_| ())
}
impl Grants {
    pub(crate) async fn revoke_project_grants_in(
        conn: &mut SqliteConnection,
        project: &ProjectId,
        actor: Actor,
        ts: &str,
    ) -> Result<(), StorageError> {
        let live: Vec<GrantRow> = sqlx::query_as(
            "SELECT id, kind, project_id, thread_id, created_at, revoked_at FROM mcp_grant
          WHERE kind = 'project' AND project_id = ? AND revoked_at IS NULL ORDER BY id",
        )
        .bind(project.as_str())
        .fetch_all(&mut *conn)
        .await?;
        for row in live {
            revoke_in(conn, &into_grant(row)?, actor.clone(), ts).await?;
        }
        Ok(())
    }
    pub(crate) async fn revoke_thread_grants_in(
        conn: &mut SqliteConnection,
        thread: &ThreadId,
        actor: Actor,
        ts: &str,
    ) -> Result<(), StorageError> {
        let live: Vec<GrantRow> = sqlx::query_as(
            "SELECT id, kind, project_id, thread_id, created_at, revoked_at FROM mcp_grant
          WHERE kind = 'thread' AND thread_id = ? AND revoked_at IS NULL ORDER BY id",
        )
        .bind(thread.as_str())
        .fetch_all(&mut *conn)
        .await?;
        for row in live {
            revoke_in(conn, &into_grant(row)?, actor.clone(), ts).await?;
        }
        Ok(())
    }
}

fn rfc3339(at: time::OffsetDateTime) -> String {
    at.format(&time::format_description::well_known::Rfc3339)
        .expect("RFC3339 formatting cannot fail")
}
impl Grants {
    pub(crate) async fn check_writer(
        conn: &mut SqliteConnection,
        writer: &Writer,
        project: &ProjectId,
    ) -> Result<(), GrantsError> {
        let Some(grant) = writer.grant() else {
            return Ok(());
        };
        let row: Option<(String, Option<String>, String, Option<String>)> = sqlx::query_as(
            "SELECT kind, thread_id, project_id, revoked_at FROM mcp_grant WHERE id = ?",
        )
        .bind(grant.as_str())
        .fetch_optional(&mut *conn)
        .await?;
        let Some((kind, grant_thread, grant_project, None)) = row else {
            return Err(GrantsError::GrantInvalid);
        };
        let in_scope = match writer {
            Writer::Planner {
                thread: own_thread, ..
            } => {
                kind == "thread"
                    && grant_thread.as_deref() == Some(own_thread.as_str())
                    && grant_project == project.as_str()
            }
            Writer::External { .. } => kind == "project" && grant_project == project.as_str(),
            Writer::Person => true,
        };
        if in_scope {
            Ok(())
        } else {
            Err(GrantsError::GrantScope)
        }
    }
    pub(crate) async fn bind_draft_ref(
        conn: &mut SqliteConnection,
        writer: &Writer,
        draft_ref: &str,
        workflow: &str,
        project: &ProjectId,
        ts: &str,
    ) -> Result<(), GrantsError> {
        let grant = writer.grant().ok_or(GrantsError::GrantScope)?;
        // `julianday`, not text order: RFC 3339 with trimmed fractional seconds
        // does not sort in time order within a second.
        let bound = sqlx::query(
            "UPDATE draft_intent SET workflow_id = ?
          WHERE draft_ref = ? AND grant_id = ?
            AND ((workflow_id IS NULL AND julianday(expires_at) > julianday(?))
                 OR workflow_id = ?)",
        )
        .bind(workflow)
        .bind(draft_ref)
        .bind(grant.as_str())
        .bind(ts)
        .bind(workflow)
        .execute(&mut *conn)
        .await?;
        let grant_project: Option<String> =
            sqlx::query_scalar("SELECT project_id FROM mcp_grant WHERE id = ?")
                .bind(grant.as_str())
                .fetch_optional(&mut *conn)
                .await?;
        if bound.rows_affected() == 1 && grant_project.as_deref() == Some(project.as_str()) {
            Ok(())
        } else {
            Err(GrantsError::GrantScope)
        }
    }
}
