//! Transaction-local approval authority and provenance reads.
use super::{Grants, GrantsError};
use crate::grants::{ApprovalRequest, ApprovedBy, Approver, Writer};
use crate::{ProjectId, StorageError, ThreadEntryId};
use core_threads::Threads;
use sqlx::SqliteConnection;

impl Grants {
    pub(crate) async fn authorise_in(
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
        Ok(Some(
            Threads::request_entry_in(conn, thread, turn)
                .await?
                .ok_or(StorageError::NotFound("request message"))?,
        ))
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
