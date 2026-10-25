//! Snapshot reads of vision proposal state.

use super::DesignStore;
use crate::{ProjectId, StorageError, VisionContent};
use crate::{ProposalStatus, VisionAction, VisionProposal};
use sqlx::{FromRow, SqliteConnection};

#[derive(FromRow)]
struct Row {
    id: String,
    thread_id: String,
    action: String,
    field: String,
    item_id: Option<String>,
    text: Option<String>,
    old_text: Option<String>,
    expected_item_revision: Option<i64>,
    status: String,
    created_at: String,
    approved_principal: Option<String>,
    approved_agent: Option<String>,
    approved_request_entry: Option<String>,
}
impl Row {
    async fn view(
        self,
        conn: &mut SqliteConnection,
        content: &VisionContent,
    ) -> Result<VisionProposal, StorageError> {
        let mut p = VisionProposal {
            approved_by: core_grants::Grants::read_in(
                conn,
                self.approved_principal,
                self.approved_agent,
                self.approved_request_entry,
            )
            .await?,
            id: self.id,
            thread_id: crate::ThreadId::from_recorded(self.thread_id),
            action: serde_json::from_value(serde_json::json!(self.action))?,
            field: serde_json::from_value(serde_json::json!(self.field))?,
            item_id: self.item_id,
            text: self.text,
            old_text: self.old_text,
            expected_item_revision: self.expected_item_revision,
            status: serde_json::from_value(serde_json::json!(self.status))?,
            outdated: false,
            created_at: self.created_at,
        };
        p.outdated = p.status == ProposalStatus::Pending
            && p.action != VisionAction::Add
            && content
                .item(p.item_id.as_deref().unwrap_or_default())
                .is_none_or(|i| Some(i.revision) != p.expected_item_revision);
        Ok(p)
    }
}

impl ProposalReadRows {
    pub(super) async fn live_vision(
        conn: &mut SqliteConnection,
        project: &ProjectId,
    ) -> Result<VisionContent, StorageError> {
        core_projects::Projects::require_live_in(conn, project).await?;
        let json: Option<Option<String>> =
            sqlx::query_scalar("SELECT vision_content FROM design_workspace WHERE project_id=?")
                .bind(project.as_str())
                .fetch_optional(conn)
                .await?;
        json.flatten()
            .map(|s| serde_json::from_str(&s))
            .transpose()
            .map(|v| v.unwrap_or_default())
            .map_err(Into::into)
    }
}

impl ProposalReadRows {
    pub(super) async fn load(
        conn: &mut SqliteConnection,
        project: &ProjectId,
        id: &str,
        content: &VisionContent,
    ) -> Result<VisionProposal, StorageError> {
        sqlx::query_as::<_, Row>(
            "SELECT id,thread_id,action,field,item_id,text,old_text,
        expected_item_revision,status,created_at,approved_principal,approved_agent,
        approved_request_entry FROM design_vision_proposal
        WHERE project_id=? AND id=?",
        )
        .bind(project.as_str())
        .bind(id)
        .fetch_optional(&mut *conn)
        .await?
        .ok_or(StorageError::NotFound("vision proposal"))?
        .view(conn, content)
        .await
    }
}

impl DesignStore {
    pub async fn list_vision_proposals(
        &self,
        project: &ProjectId,
    ) -> Result<Vec<VisionProposal>, StorageError> {
        let mut txn = self.storage.reader().begin().await?;
        let content = ProposalReadRows::live_vision(&mut txn, project).await?;
        let rows = sqlx::query_as::<_, Row>(
            "SELECT id,thread_id,action,field,item_id,text,old_text,
            expected_item_revision,status,created_at,approved_principal,approved_agent,
            approved_request_entry FROM design_vision_proposal
            WHERE project_id=? ORDER BY seq",
        )
        .bind(project.as_str())
        .fetch_all(&mut *txn)
        .await?;
        let mut result = Vec::new();
        for row in rows {
            result.push(row.view(&mut txn, &content).await?);
        }
        Ok(result)
    }
}

pub(crate) struct ProposalReadRows;
