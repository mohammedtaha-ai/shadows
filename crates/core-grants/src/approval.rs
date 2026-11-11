//! Transaction-local approval authority and provenance reads.
use super::{Grants, GrantsError};
use crate::{ApprovalRequest, ApprovedBy, Approver, Writer};
use core_db::StorageError;
use core_projects::ProjectId;
use core_threads::ThreadEntryId;
use core_threads::Threads;
use sqlx::SqliteConnection;

impl Grants {
    pub async fn authorise_in(
        conn: &mut SqliteConnection,
        approver: &Approver,
        project: &ProjectId,
    ) -> Result<Option<ThreadEntryId>, GrantsError> {
        let Approver::Planner {
            writer,
            turn,
            harness,
        } = approver
        else {
            return Ok(None);
        };
        let Writer::Planner { thread, .. } = writer else {
            return Err(GrantsError::GrantScope);
        };
        Self::check_writer(conn, writer, project).await?;
        let actual = Threads::harness_in(conn, thread, project).await?;
        if actual != *harness {
            return Err(GrantsError::GrantScope);
        }
        // Only the person approves: a turn a notice started holds no message
        // from them until they steer one in.
        let request = Threads::request_entry_in(conn, thread, turn).await?;
        Ok(Some(request.ok_or_else(|| {
            GrantsError::GrantScopeMessage(
                "Only the person approves: this turn holds no message from the person.".into(),
            )
        })?))
    }

    pub async fn read_in(
        conn: &mut SqliteConnection,
        principal: Option<String>,
        agent: Option<String>,
        request: Option<String>,
    ) -> Result<Option<ApprovedBy>, StorageError> {
        let (Some(principal), Some(agent)) = (principal, agent) else {
            return Ok(None);
        };
        let request = if let Some(id) = request {
            let (thread_id, ordinal) =
                Threads::entry_in(conn, &ThreadEntryId::from_recorded(id.clone())).await?;
            Some(ApprovalRequest {
                thread_id,
                entry_id: ThreadEntryId::from_recorded(id),
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
}
