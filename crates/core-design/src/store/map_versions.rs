//! Transactional map snapshots and per-part review baselines.
#[cfg(feature = "test-support")]
use super::DesignStore;
use crate::{PartContent, VisionContent, map_rules};
use crate::{ProjectId, StorageError};
#[cfg(feature = "test-support")]
use core_db::Storage;
use core_standards::{EffectiveStandards, Standards};
use sqlx::{FromRow, SqliteConnection};
use std::collections::{BTreeMap, BTreeSet};

#[derive(FromRow, serde::Serialize, serde::Deserialize)]
pub(super) struct MapRow {
    pub id: String,
    pub parent_id: Option<String>,
    pub ordinal: i64,
    pub revision: i64,
    pub content_json: String,
    pub served_revisions: String,
}

impl MapVersionsRows {
    pub(super) async fn rows(
        conn: &mut SqliteConnection,
        p: &ProjectId,
    ) -> Result<Vec<MapRow>, StorageError> {
        Ok(sqlx::query_as(
            "SELECT id,parent_id,ordinal,revision,content_json,served_revisions
        FROM design_part WHERE project_id=? ORDER BY ordinal,id",
        )
        .bind(p.as_str())
        .fetch_all(conn)
        .await?)
    }
}

impl MapVersionsRows {
    pub(super) async fn effective(
        conn: &mut SqliteConnection,
        p: &ProjectId,
    ) -> Result<EffectiveStandards, StorageError> {
        Ok(EffectiveStandards {
            base: Standards::base().clone(),
            additions: Standards::current_in(conn, p).await?,
        })
    }
}

impl MapVersionsRows {
    pub(super) async fn waived(
        conn: &mut SqliteConnection,
        p: &ProjectId,
    ) -> Result<Vec<String>, StorageError> {
        Ok(sqlx::query_scalar(
            "SELECT part_name FROM design_part_waiver WHERE project_id=?
        ORDER BY part_name",
        )
        .bind(p.as_str())
        .fetch_all(conn)
        .await?)
    }
}

impl MapVersionsRows {
    pub(super) fn contents(rows: &[MapRow]) -> Result<Vec<PartContent>, StorageError> {
        rows.iter()
            .map(|r| serde_json::from_str(&r.content_json).map_err(Into::into))
            .collect()
    }
}

impl MapVersionsRows {
    pub(super) fn views<'a>(
        rows: &'a [MapRow],
        contents: &'a [PartContent],
    ) -> Vec<map_rules::MapPart<'a>> {
        rows.iter()
            .zip(contents)
            .map(|(r, c)| map_rules::MapPart {
                key: &r.id,
                parent: r.parent_id.as_deref(),
                content: c,
            })
            .collect()
    }
}

impl MapVersionsRows {
    pub(super) async fn insert_version(
        conn: &mut SqliteConnection,
        p: &ProjectId,
        proposal: Option<&str>,
        ts: &str,
    ) -> Result<(), StorageError> {
        let (number, vision): (i64, i64) = sqlx::query_as(
            "SELECT map_revision,vision_revision FROM design_workspace WHERE project_id=?",
        )
        .bind(p.as_str())
        .fetch_one(&mut *conn)
        .await?;
        let number = number
            .checked_add(1)
            .ok_or_else(|| StorageError::Unavailable("map revision exhausted".into()))?;
        let parts = MapVersionsRows::rows(conn, p).await?;
        let standards = MapVersionsRows::effective(conn, p).await?;
        sqlx::query("INSERT INTO design_map_version VALUES (?,?,?,?,?,?,?,?)")
            .bind(p.as_str())
            .bind(number)
            .bind(serde_json::to_string(&parts)?)
            .bind(vision)
            .bind(proposal)
            .bind(standards.base.version)
            .bind(standards.additions.as_ref().map(|a| &a.id))
            .bind(ts)
            .execute(&mut *conn)
            .await?;
        sqlx::query("UPDATE design_workspace SET map_revision=? WHERE project_id=?")
            .bind(number)
            .bind(p.as_str())
            .execute(conn)
            .await?;
        Ok(())
    }
}

impl MapVersionsRows {
    pub(super) async fn review(
        conn: &mut SqliteConnection,
        p: &ProjectId,
        ids: &BTreeSet<String>,
        vision: &VisionContent,
    ) -> Result<(), StorageError> {
        for row in MapVersionsRows::rows(conn, p)
            .await?
            .iter()
            .filter(|r| ids.contains(&r.id))
        {
            let content: PartContent = serde_json::from_str(&row.content_json)?;
            let revisions: BTreeMap<_, _> = content
                .serves
                .iter()
                .filter_map(|id| vision.item(id).map(|i| (id, i.revision)))
                .collect();
            sqlx::query("UPDATE design_part SET served_revisions=? WHERE project_id=? AND id=?")
                .bind(serde_json::to_string(&revisions)?)
                .bind(p.as_str())
                .bind(&row.id)
                .execute(&mut *conn)
                .await?;
        }
        Ok(())
    }
}

#[cfg(feature = "test-support")]
impl DesignStore {
    #[cfg(feature = "test-support")]
    pub async fn seed_map_for_tests(&self, p: &ProjectId) -> Result<(), StorageError> {
        let p = p.clone();
        self.storage
            .write_txn(move |conn| {
                Box::pin(async move {
                    sqlx::query(
                        "INSERT OR IGNORE INTO design_workspace
                (project_id,revision,vision_revision,vision_content) VALUES (?,0,0,?)",
                    )
                    .bind(p.as_str())
                    .bind(serde_json::to_string(&VisionContent::default())?)
                    .execute(&mut *conn)
                    .await?;
                    let n: i64 = sqlx::query_scalar(
                        "SELECT map_revision FROM design_workspace WHERE project_id=?",
                    )
                    .bind(p.as_str())
                    .fetch_one(&mut *conn)
                    .await?;
                    if n == 0 {
                        MapVersionsRows::insert_version(conn, &p, None, &Storage::now()).await?;
                    }
                    Ok(())
                })
            })
            .await
    }
}

pub(crate) struct MapVersionsRows;
