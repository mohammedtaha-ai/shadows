//! One job: what a writer's MCP grant permits, judged inside the write's own
//! transaction (spec §13.7: "checked twice"), so a write that started before
//! a revocation does not commit after it.

use sqlx::SqliteConnection;

use super::StorageError;
use crate::command::Writer;
use crate::project::ProjectId;
use crate::thread::ThreadId;
use crate::workflow::WorkflowId;

/// Ok for a person. For a grant holder: `GrantInvalid` when the grant is
/// unknown or revoked; `GrantScope` when the write's thread and project are
/// outside it — a Planner writes only its own thread, an external agent only
/// its project. `thread` is `None` for a thread not yet created.
pub(super) async fn check_writer(
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
pub(super) async fn bind_draft_ref(
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
