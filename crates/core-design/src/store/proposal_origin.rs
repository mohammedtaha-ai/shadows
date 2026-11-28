//! Persistence provenance for a project proposal.
use crate::{DesignError, OperationId, ProjectId, StorageError, ThreadId};
use core_db::{CommandContext, JournalEntry};
use core_grants::{Approver, GrantId, Grants, Writer};
use core_threads::{NewThreadEntry, ThreadEntryId, ThreadEntryKind, Threads};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::Value;
use sqlx::SqliteConnection;

pub(super) struct ProposalOrigin<'a> {
    writer: &'a Writer,
    project: &'a ProjectId,
    turn: Option<&'a OperationId>,
    pub(super) thread: Option<ThreadId>,
    pub(super) grant: Option<GrantId>,
    pub(super) connection: Option<String>,
}

impl<'a> ProposalOrigin<'a> {
    pub(super) fn check_approver(
        grant: Option<&GrantId>,
        approver: &Approver,
    ) -> Result<(), StorageError> {
        if grant.is_some() && matches!(approver, Approver::Planner(_)) {
            return Err(StorageError::Constraint(
                "an external proposal is approved by the person's button".into(),
            ));
        }
        Ok(())
    }

    pub(super) async fn new(
        conn: &mut SqliteConnection,
        writer: &'a Writer,
        project: &'a ProjectId,
        turn: Option<&'a OperationId>,
    ) -> Result<Self, DesignError> {
        Grants::check_writer(conn, writer, project).await?;
        let (thread, grant, connection) = match (writer, turn) {
            (Writer::Planner { thread, .. }, Some(_)) => (Some(thread.clone()), None, None),
            (Writer::External { grant }, None) => {
                let connection = Grants::connections_of_in(conn, std::slice::from_ref(grant))
                    .await?
                    .remove(grant);
                (None, Some(grant.clone()), connection)
            }
            _ => return Err(core_grants::GrantsError::GrantScope.into()),
        };
        Ok(Self {
            writer,
            project,
            turn,
            thread,
            grant,
            connection,
        })
    }

    pub(super) fn scope(&self) -> (&'static str, &str) {
        match &self.thread {
            Some(thread) => ("Thread", thread.as_str()),
            None => ("Project", self.project.as_str()),
        }
    }

    pub(super) async fn require_running(
        &self,
        conn: &mut SqliteConnection,
    ) -> Result<(), DesignError> {
        if let (Some(thread), Some(turn)) = (&self.thread, self.turn) {
            core_operations::Operations::require_running_in(conn, thread, turn.as_str()).await?;
        }
        Ok(())
    }

    pub(super) async fn replay<T: DeserializeOwned>(
        &self,
        conn: &mut SqliteConnection,
        id: &str,
        planner_entry: Option<String>,
    ) -> Result<T, DesignError> {
        let json: String = if let Some(entry) = planner_entry {
            Threads::card_in(conn, &ThreadEntryId::from_recorded(entry)).await?
        } else {
            sqlx::query_scalar(
                "SELECT result_json FROM design_command_result WHERE id=? AND project_id=?",
            )
            .bind(id)
            .bind(self.project.as_str())
            .fetch_one(&mut *conn)
            .await?
        };
        Ok(serde_json::from_str(&json)?)
    }

    pub(super) async fn append_card(
        &self,
        conn: &mut SqliteConnection,
        kind: ThreadEntryKind,
        body: &str,
        card: &Value,
        now: &str,
    ) -> Result<Option<ThreadEntryId>, DesignError> {
        let Some(thread) = &self.thread else {
            return Ok(None);
        };
        let entry = Threads::append_entry_in(
            conn,
            thread,
            NewThreadEntry {
                kind,
                author: self.writer.actor(),
                body,
                refs: &[],
                card: Some(card),
                operation_id: self.turn.map(OperationId::as_str),
            },
            now,
        )
        .await?;
        Ok(Some(entry.id))
    }

    pub(super) fn event(&self, kind: &str, proposal: &str) -> JournalEntry {
        let mut event = JournalEntry::new(kind, self.writer.actor())
            .with_project(self.project.as_str())
            .with_payload(serde_json::json!({"project_id":self.project,"proposal_id":proposal}));
        if let (Some(thread), Some(turn)) = (&self.thread, self.turn) {
            event = event
                .with_thread(thread.as_str())
                .with_operation(turn.as_str());
        }
        event
    }

    pub(super) async fn record(
        &self,
        conn: &mut SqliteConnection,
        ctx: &CommandContext,
        kind: &str,
        id: &str,
        response: &impl Serialize,
        now: &str,
    ) -> Result<(), StorageError> {
        if self.grant.is_some() {
            sqlx::query("INSERT INTO design_command_result VALUES (?,?,?)")
                .bind(id)
                .bind(self.project.as_str())
                .bind(serde_json::to_string(response)?)
                .execute(&mut *conn)
                .await?;
        }
        let (scope, key) = self.scope();
        CommandContext::record_command(conn, ctx, scope, key, kind, id, now).await
    }
}
