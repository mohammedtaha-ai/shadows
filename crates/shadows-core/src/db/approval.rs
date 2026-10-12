//! Transaction-local approval authority and provenance reads.
use crate::command::{ApprovalRequest, ApprovedBy, Approver, Writer};
use crate::{ProjectId, StorageError, ThreadEntryId, ThreadId};
use sqlx::SqliteConnection;

pub(crate) async fn authorise_in(
    conn: &mut SqliteConnection,
    approver: &Approver,
    project: &ProjectId,
) -> Result<Option<ThreadEntryId>, StorageError> {
    let Approver::Planner {
        writer,
        turn,
        harness,
    } = approver
    else {
        return Ok(None);
    };
    let Writer::Planner { thread, .. } = writer else {
        return Err(StorageError::GrantScope);
    };
    crate::grants::check_writer(conn, writer, project).await?;
    crate::turns::check_running_turn_in(conn, thread, turn).await?;
    let actual: String =
        sqlx::query_scalar("SELECT harness_kind FROM planning_thread WHERE id=? AND project_id=?")
            .bind(thread.as_str())
            .bind(project.as_str())
            .fetch_one(&mut *conn)
            .await?;
    if actual != *harness {
        return Err(StorageError::GrantScope);
    }
    let entry: Option<String> = sqlx::query_scalar(
        "SELECT id FROM thread_entry WHERE thread_id=? AND operation_id=?
         AND kind='UserMessage' ORDER BY ordinal DESC LIMIT 1",
    )
    .bind(thread.as_str())
    .bind(turn.as_str())
    .fetch_optional(conn)
    .await?;
    Ok(Some(ThreadEntryId::from_stored(
        entry.ok_or(StorageError::NotFound("request message"))?,
    )))
}

pub(crate) async fn read_in(
    conn: &mut SqliteConnection,
    principal: Option<String>,
    agent: Option<String>,
    request: Option<String>,
) -> Result<Option<ApprovedBy>, StorageError> {
    let (Some(principal), Some(agent)) = (principal, agent) else {
        return Ok(None);
    };
    let request = if let Some(id) = request {
        let (thread, ordinal): (String, i64) =
            sqlx::query_as("SELECT thread_id,ordinal FROM thread_entry WHERE id=?")
                .bind(&id)
                .fetch_one(conn)
                .await?;
        Some(ApprovalRequest {
            thread_id: ThreadId::from_stored(thread),
            entry_id: ThreadEntryId::from_stored(id),
            ordinal,
        })
    } else {
        None
    };
    Ok(Some(ApprovedBy {
        principal,
        agent,
        request,
    }))
}
