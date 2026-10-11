//! Snapshot reads of vision proposal state (§23.5).
use crate::db::Storage;
use crate::design::{ProposalStatus, VisionAction, VisionProposal};
use crate::{ProjectId, StorageError, VisionContent};
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
}
impl Row {
    fn view(self, content: &VisionContent) -> Result<VisionProposal, StorageError> {
        let mut p = VisionProposal {
            id: self.id,
            thread_id: crate::ThreadId::from_stored(self.thread_id),
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

pub(super) async fn live_vision(
    conn: &mut SqliteConnection,
    project: &ProjectId,
) -> Result<VisionContent, StorageError> {
    let json: Option<Option<String>> = sqlx::query_scalar(
        "SELECT w.vision_content FROM project p
         LEFT JOIN design_workspace w ON w.project_id=p.id
         WHERE p.id=? AND p.removed_at IS NULL",
    )
    .bind(project.as_str())
    .fetch_optional(conn)
    .await?;
    json.ok_or(StorageError::NotFound("project"))?
        .map(|s| serde_json::from_str(&s))
        .transpose()
        .map(|v| v.unwrap_or_default())
        .map_err(Into::into)
}

pub(super) async fn load(
    conn: &mut SqliteConnection,
    project: &ProjectId,
    id: &str,
    content: &VisionContent,
) -> Result<VisionProposal, StorageError> {
    sqlx::query_as::<_, Row>(
        "SELECT id,thread_id,action,field,item_id,text,old_text,
        expected_item_revision,status,created_at FROM design_vision_proposal
        WHERE project_id=? AND id=?",
    )
    .bind(project.as_str())
    .bind(id)
    .fetch_optional(conn)
    .await?
    .ok_or(StorageError::NotFound("vision proposal"))?
    .view(content)
}

impl Storage {
    pub async fn list_vision_proposals(
        &self,
        project: &ProjectId,
    ) -> Result<Vec<VisionProposal>, StorageError> {
        let mut txn = self.reader().begin().await?;
        let content = live_vision(&mut txn, project).await?;
        let rows = sqlx::query_as::<_, Row>(
            "SELECT id,thread_id,action,field,item_id,text,old_text,
            expected_item_revision,status,created_at FROM design_vision_proposal
            WHERE project_id=? ORDER BY seq",
        )
        .bind(project.as_str())
        .fetch_all(&mut *txn)
        .await?;
        rows.into_iter().map(|r| r.view(&content)).collect()
    }
}
