//! One job: an MCP grant's rows, from issue to revocation (spec §13.7). A
//! grant is issued with a token stored only as its hash, found by that token,
//! revoked, and judged inside a write's own transaction ("checked twice"), so
//! a write that started before a revocation does not commit after it. A
//! `draft_ref` (§13.5) is a project grant's own row, so it lives here too.
//!
//! Over 300 lines (CLAUDE.md): every function reads or writes `mcp_grant`, or
//! the `draft_intent` rows keyed by it, and a split would leave the check to
//! be read beside the issue and revocation whose rows it judges.

use sqlx::SqliteConnection;

use super::model::{Grant, GrantId, GrantKind, IssuedGrant, Token, hash_token};
use crate::command::{CommandContext, Writer};
use crate::db::{Storage, StorageError, append_event, classify, now, record_command};
use crate::events::{Actor, DurableEvent};
use crate::plans::WorkflowId;
use crate::projects::ProjectId;
use crate::threads::ThreadId;

/// How long an unused `draft_ref` can still start a plan (§13.5).
const DRAFT_REF_LIFETIME: time::Duration = time::Duration::HOUR;

const GRANT_COLUMNS: &str = "id, kind, project_id, thread_id, created_at, revoked_at";

impl Storage {
    /// An external agent's grant on `project` (§13.7's Connect). `token` is
    /// `Some` on the first issue and `None` on a replay: the token is shown once.
    pub async fn issue_project_grant(
        &self,
        ctx: &CommandContext,
        project: &ProjectId,
    ) -> Result<IssuedGrant, StorageError> {
        let (ctx, project, ts) = (ctx.clone(), project.clone(), now());
        self.write_txn(move |conn| {
            Box::pin(async move {
                if let Some(id) = classify(conn, &ctx, "Project", project.as_str()).await? {
                    let grant = load_grant(conn, &GrantId::from_stored(id)).await?;
                    return Ok(IssuedGrant { grant, token: None });
                }
                let known: Option<i64> = sqlx::query_scalar("SELECT 1 FROM project WHERE id = ?")
                    .bind(project.as_str())
                    .fetch_optional(&mut *conn)
                    .await?;
                known.ok_or(StorageError::NotFound("project"))?;
                let actor = Actor::user(&ctx.principal_id);
                let (grant, token) = insert_grant(conn, &project, None, actor, &ts).await?;
                record_command(
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

    /// The Planner's grant on `thread`, in the thread's own project. Internal:
    /// issued each time an adapter opens (§13.7), so no command names it.
    pub async fn issue_thread_grant(
        &self,
        thread: &ThreadId,
    ) -> Result<(Grant, Token), StorageError> {
        let (thread, ts) = (thread.clone(), now());
        self.write_txn(move |conn| {
            Box::pin(async move {
                let project: String =
                    sqlx::query_scalar("SELECT project_id FROM planning_thread WHERE id = ?")
                        .bind(thread.as_str())
                        .fetch_optional(&mut *conn)
                        .await?
                        .ok_or(StorageError::NotFound("thread"))?;
                let project = ProjectId::from_stored(project);
                insert_grant(conn, &project, Some(&thread), Actor::system(), &ts).await
            })
        })
        .await
    }

    /// The person revokes a project grant (§13.7's Revoke): Shadows refuses
    /// it from then on. A thread grant is the Planner's, not the person's, and
    /// is `NotFound` here. A replay answers the grant as it now stands; a
    /// second revoke of a revoked grant changes nothing.
    pub async fn revoke_grant(
        &self,
        ctx: &CommandContext,
        id: &GrantId,
    ) -> Result<Grant, StorageError> {
        let (ctx, id, ts) = (ctx.clone(), id.clone(), now());
        self.write_txn(move |conn| {
            Box::pin(async move {
                if classify(conn, &ctx, "Grant", id.as_str()).await?.is_some() {
                    return load_grant(conn, &id).await;
                }
                let grant = find_grant(conn, &id)
                    .await?
                    .filter(|g| g.kind == GrantKind::Project)
                    .ok_or(StorageError::NotFound("project grant"))?;
                revoke_in(conn, &grant, Actor::user(&ctx.principal_id), &ts).await?;
                let (scope, key) = ("Grant", id.as_str());
                record_command(conn, &ctx, scope, key, "McpGrant", key, &ts).await?;
                load_grant(conn, &id).await
            })
        })
        .await
    }

    /// Revokes the Planner's grant when its adapter closes, or its session
    /// did not open (§13.7). Internal, no command; revoking twice is a no-op.
    pub async fn revoke_thread_grant(&self, id: &GrantId) -> Result<(), StorageError> {
        let (id, ts) = (id.clone(), now());
        self.write_txn(move |conn| {
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

    /// Daemon startup (§13.7, recovery §8): a thread grant still live belongs
    /// to an adapter of a daemon that is gone. Answers how many it revoked.
    /// Project grants are the person's, and survive a restart.
    pub async fn revoke_all_thread_grants(&self) -> Result<u64, StorageError> {
        let ts = now();
        self.write_txn(move |conn| {
            Box::pin(async move {
                let live: Vec<GrantRow> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
                    "SELECT {GRANT_COLUMNS} FROM mcp_grant
                      WHERE kind = 'thread' AND revoked_at IS NULL ORDER BY id"
                )))
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

    /// The live grant `raw` answers to, found by its hash; `None` for an
    /// unknown or revoked token. `/mcp` asks this of every request (§13.6).
    pub async fn grant_for_token(&self, raw: &str) -> Result<Option<Grant>, StorageError> {
        let row: Option<GrantRow> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
            "SELECT {GRANT_COLUMNS} FROM mcp_grant WHERE token_hash = ? AND revoked_at IS NULL"
        )))
        .bind(hash_token(raw))
        .fetch_optional(self.reader())
        .await?;
        row.map(into_grant).transpose()
    }

    /// A project's grants for external agents, revoked ones included, newest
    /// first by the sequence of each one's `McpGrantIssued` event — never by
    /// `created_at`, which does not sort in time order within a second.
    pub async fn list_project_grants(
        &self,
        project: &ProjectId,
    ) -> Result<Vec<Grant>, StorageError> {
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
        .fetch_all(self.reader())
        .await?;
        rows.into_iter().map(into_grant).collect()
    }

    /// `draft_prepare` (§13.5): a new `draft_ref` bound to `grant`, good for a
    /// first use within the hour. Not deduplicated: each call is another ref,
    /// and refs never used simply expire. Only a live project grant asks.
    pub async fn prepare_draft(&self, grant: &GrantId) -> Result<String, StorageError> {
        let grant = grant.clone();
        let issued = time::OffsetDateTime::now_utc();
        let (created, expires) = (rfc3339(issued), rfc3339(issued + DRAFT_REF_LIFETIME));
        self.write_txn(move |conn| {
            Box::pin(async move {
                match find_grant(conn, &grant).await? {
                    Some(g) if g.revoked_at.is_none() && g.kind == GrantKind::Project => {}
                    Some(g) if g.revoked_at.is_none() => return Err(StorageError::GrantScope),
                    _ => return Err(StorageError::GrantInvalid),
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

    /// The plan a draft_ref already started (bound refs never expire), or None
    /// if unused and not expired; Err if unused and expired, unknown, or issued
    /// to another grant. Expiry stops only a first use (§13.5).
    #[cfg(feature = "test-support")]
    pub async fn draft_intent(
        &self,
        grant: &GrantId,
        draft_ref: &str,
    ) -> Result<Option<WorkflowId>, StorageError> {
        // `julianday`, not text order, for the reason `bind_draft_ref` gives.
        let row: Option<(String, Option<String>, bool)> = sqlx::query_as(
            "SELECT grant_id, workflow_id, julianday(expires_at) > julianday(?)
               FROM draft_intent WHERE draft_ref = ?",
        )
        .bind(now())
        .bind(draft_ref)
        .fetch_optional(self.reader())
        .await?;
        match row {
            Some((owner, Some(workflow), _)) if owner == grant.as_str() => {
                Ok(Some(WorkflowId::from_stored(workflow)))
            }
            Some((owner, None, true)) if owner == grant.as_str() => Ok(None),
            _ => Err(StorageError::GrantScope),
        }
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
    let row: Option<GrantRow> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "SELECT {GRANT_COLUMNS} FROM mcp_grant WHERE id = ?"
    )))
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

/// A new grant, its row holding only the token's hash, and its
/// `McpGrantIssued` event, which carries neither.
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
    append_event(conn, &event.with_payload(payload), ts).await?;
    Ok((load_grant(conn, &id).await?, token))
}

/// Marks `grant` revoked, with its `McpGrantRevoked` event; nothing when it
/// already was, so a second revoke records no second event.
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
    append_event(conn, &event.with_payload(payload), ts)
        .await
        .map(|_| ())
}

fn rfc3339(at: time::OffsetDateTime) -> String {
    at.format(&time::format_description::well_known::Rfc3339)
        .expect("RFC3339 formatting cannot fail")
}

/// Ok for a person. For a grant holder: `GrantInvalid` when the grant is
/// unknown or revoked; `GrantScope` when the write's thread and project are
/// outside it — a Planner writes only its own thread, an external agent only
/// its project. `thread` is `None` for a thread not yet created.
pub(crate) async fn check_writer(
    conn: &mut SqliteConnection,
    writer: &Writer,
    project: &ProjectId,
    thread: Option<&ThreadId>,
) -> Result<(), StorageError> {
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
        return Err(StorageError::GrantInvalid);
    };
    let in_scope = match writer {
        Writer::Planner {
            thread: own_thread, ..
        } => {
            kind == "thread"
                && grant_thread.as_deref() == Some(own_thread.as_str())
                && thread == Some(own_thread)
        }
        Writer::External { .. } => kind == "project" && grant_project == project.as_str(),
        Writer::Person => true,
    };
    if in_scope {
        Ok(())
    } else {
        Err(StorageError::GrantScope)
    }
}

/// Binds `draft_ref` to `workflow` (§13.5). Expiry stops only a ref's first
/// use: a ref already bound to this plan stays good. Zero rows — another
/// grant's ref, an expired one, one bound to another plan — or a plan outside
/// the grant's project is `GrantScope`.
pub(crate) async fn bind_draft_ref(
    conn: &mut SqliteConnection,
    writer: &Writer,
    draft_ref: &str,
    workflow: &WorkflowId,
    project: &ProjectId,
    ts: &str,
) -> Result<(), StorageError> {
    let grant = writer.grant().ok_or(StorageError::GrantScope)?;
    // `julianday`, not text order: RFC 3339 with trimmed fractional seconds
    // does not sort in time order within a second.
    let bound = sqlx::query(
        "UPDATE draft_intent SET workflow_id = ?
          WHERE draft_ref = ? AND grant_id = ?
            AND ((workflow_id IS NULL AND julianday(expires_at) > julianday(?))
                 OR workflow_id = ?)",
    )
    .bind(workflow.as_str())
    .bind(draft_ref)
    .bind(grant.as_str())
    .bind(ts)
    .bind(workflow.as_str())
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
        Err(StorageError::GrantScope)
    }
}
