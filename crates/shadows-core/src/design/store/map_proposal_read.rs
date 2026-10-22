//! Snapshot reads and dependency-based staleness of map cards.
use crate::db::Storage;
use crate::design::{MapAction, MapProposal, ProposalStatus};
use crate::{ProjectId, StorageError};
use sqlx::{FromRow, SqliteConnection};
use std::collections::BTreeMap;

#[derive(FromRow)]
pub(super) struct Row {
    pub payload_json: String,
    pub status: String,
    pub vision_revision: i64,
    pub expected_part_revision: Option<i64>,
    pub expected_items_json: String,
    pub standards_version: i64,
    pub standards_additions_version_id: Option<String>,
    pub approved_principal: Option<String>,
    pub approved_agent: Option<String>,
    pub approved_request_entry: Option<String>,
}
pub(super) async fn load(
    conn: &mut SqliteConnection,
    p: &ProjectId,
    id: &str,
) -> Result<MapProposal, StorageError> {
    let row: Row = sqlx::query_as(
        "SELECT payload_json,status,vision_revision,
        expected_part_revision,expected_items_json,standards_version,standards_additions_version_id,
        approved_principal,approved_agent,approved_request_entry
        FROM design_map_proposal WHERE project_id=? AND id=?",
    )
    .bind(p.as_str())
    .bind(id)
    .fetch_optional(&mut *conn)
    .await?
    .ok_or(StorageError::NotFound("map proposal"))?;
    view(conn, p, row).await
}
async fn view(
    conn: &mut SqliteConnection,
    p: &ProjectId,
    row: Row,
) -> Result<MapProposal, StorageError> {
    let vision = super::proposal_read::live_vision(conn, p).await?;
    let mut proposal: MapProposal = serde_json::from_str(&row.payload_json)?;
    proposal.approved_by = crate::grants::Grants::read_in(
        conn,
        row.approved_principal,
        row.approved_agent,
        row.approved_request_entry,
    )
    .await?;
    proposal.status = serde_json::from_value(serde_json::json!(row.status))?;
    let standards = super::map_versions::effective(conn, p).await?;
    let (v, m): (i64, i64) = sqlx::query_as(
        "SELECT vision_revision,map_revision FROM design_workspace WHERE project_id=?",
    )
    .bind(p.as_str())
    .fetch_one(&mut *conn)
    .await?;
    let expected: BTreeMap<String, i64> = serde_json::from_str(&row.expected_items_json)?;
    let mut outdated = standards.base.version != row.standards_version
        || standards.additions.as_ref().map(|a| &a.id)
            != row.standards_additions_version_id.as_ref();
    if proposal.action == MapAction::First {
        outdated |= v != row.vision_revision || m != 0;
    } else {
        outdated |= expected
            .iter()
            .any(|(id, rev)| vision.item(id).is_none_or(|i| i.revision != *rev));
        if let Some(id) = &proposal.part_id {
            let rev: Option<i64> =
                sqlx::query_scalar("SELECT revision FROM design_part WHERE project_id=? AND id=?")
                    .bind(p.as_str())
                    .bind(id.as_str())
                    .fetch_optional(&mut *conn)
                    .await?;
            outdated |= rev != row.expected_part_revision;
        }
    }
    proposal.outdated = proposal.status == ProposalStatus::Pending && outdated;
    Ok(proposal)
}
impl Storage {
    pub async fn list_map_proposals(
        &self,
        p: &ProjectId,
    ) -> Result<Vec<MapProposal>, StorageError> {
        let mut tx = self.reader().begin().await?;
        super::proposal_read::live_vision(&mut tx, p).await?;
        let rows: Vec<Row> = sqlx::query_as(
            "SELECT payload_json,status,vision_revision,
            expected_part_revision,expected_items_json,standards_version,
            standards_additions_version_id,approved_principal,approved_agent,approved_request_entry
            FROM design_map_proposal WHERE project_id=? ORDER BY seq,id",
        )
        .bind(p.as_str())
        .fetch_all(&mut *tx)
        .await?;
        let mut result = Vec::new();
        for row in rows {
            result.push(view(&mut tx, p, row).await?);
        }
        tx.commit().await?;
        Ok(result)
    }
}
