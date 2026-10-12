//! Snapshot reads and dependency-based staleness of roadmap cards.
use crate::db::Storage;
use crate::design::{ProposalStatus, RoadmapAction, RoadmapProposal};
use crate::{ProjectId, StorageError};
use sqlx::{FromRow, SqliteConnection};
use std::collections::BTreeMap;

#[derive(FromRow)]
struct Row {
    payload_json: String,
    status: String,
    expected_outcome_revision: Option<i64>,
    expected_parts_json: String,
    approved_principal: Option<String>,
    approved_agent: Option<String>,
    approved_request_entry: Option<String>,
}

pub(super) async fn load(
    conn: &mut SqliteConnection,
    p: &ProjectId,
    id: &str,
) -> Result<RoadmapProposal, StorageError> {
    let row: Row = sqlx::query_as(
        "SELECT payload_json,status,expected_outcome_revision,expected_parts_json,
         approved_principal,approved_agent,approved_request_entry
         FROM design_roadmap_proposal WHERE project_id=? AND id=?",
    )
    .bind(p.as_str())
    .bind(id)
    .fetch_optional(&mut *conn)
    .await?
    .ok_or(StorageError::NotFound("roadmap proposal"))?;
    view(conn, p, row).await
}

async fn view(
    conn: &mut SqliteConnection,
    p: &ProjectId,
    row: Row,
) -> Result<RoadmapProposal, StorageError> {
    let mut proposal: RoadmapProposal = serde_json::from_str(&row.payload_json)?;
    proposal.approved_by = crate::db::approval::read_in(
        conn,
        row.approved_principal,
        row.approved_agent,
        row.approved_request_entry,
    )
    .await?;
    proposal.status = serde_json::from_value(serde_json::json!(row.status))?;
    let revision: i64 =
        sqlx::query_scalar("SELECT roadmap_revision FROM design_workspace WHERE project_id=?")
            .bind(p.as_str())
            .fetch_one(&mut *conn)
            .await?;
    let expected: BTreeMap<String, i64> = serde_json::from_str(&row.expected_parts_json)?;
    let mut outdated = proposal.action == RoadmapAction::First && revision > 0;
    for (part, expected) in &expected {
        let current: Option<i64> =
            sqlx::query_scalar("SELECT revision FROM design_part WHERE project_id=? AND id=?")
                .bind(p.as_str())
                .bind(part)
                .fetch_optional(&mut *conn)
                .await?;
        outdated |= current != Some(*expected);
    }
    if let Some(id) = &proposal.outcome_id {
        let current: Option<i64> =
            sqlx::query_scalar("SELECT revision FROM design_outcome WHERE project_id=? AND id=?")
                .bind(p.as_str())
                .bind(id.as_str())
                .fetch_optional(&mut *conn)
                .await?;
        outdated |= current.is_none() || current != row.expected_outcome_revision;
    }
    proposal.outdated = proposal.status == ProposalStatus::Pending && outdated;
    Ok(proposal)
}

impl Storage {
    pub async fn list_roadmap_proposals(
        &self,
        p: &ProjectId,
    ) -> Result<Vec<RoadmapProposal>, StorageError> {
        let mut tx = self.reader().begin().await?;
        super::proposal_read::live_vision(&mut tx, p).await?;
        let rows: Vec<Row> = sqlx::query_as(
            "SELECT payload_json,status,expected_outcome_revision,expected_parts_json,
             approved_principal,approved_agent,approved_request_entry
             FROM design_roadmap_proposal WHERE project_id=? ORDER BY seq,id",
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
