//! Reading every stage input in one consistent project snapshot.

use crate::db::{Storage, StorageError};
use crate::design::{PartContent, StageView, VisionContent, stage};
use crate::projects::ProjectId;
use crate::standards::{EffectiveStandards, Standards};

impl Storage {
    pub async fn project_stage(&self, project: &ProjectId) -> Result<StageView, StorageError> {
        let mut tx = self.reader().begin().await?;
        let row: Option<(Option<String>,)> = sqlx::query_as(
            "SELECT w.vision_content FROM project p
             LEFT JOIN design_workspace w ON w.project_id=p.id
             WHERE p.id=? AND p.removed_at IS NULL",
        )
        .bind(project.as_str())
        .fetch_optional(&mut *tx)
        .await?;
        let (content,) = row.ok_or(StorageError::NotFound("project"))?;
        let vision: VisionContent = content
            .map(|s| serde_json::from_str(&s))
            .transpose()?
            .unwrap_or_default();
        let parts: Vec<String> = sqlx::query_scalar(
            "SELECT content_json FROM design_part WHERE project_id=? AND parent_id IS NULL
             ORDER BY ordinal,id",
        )
        .bind(project.as_str())
        .fetch_all(&mut *tx)
        .await?;
        let kinds: Vec<String> = parts
            .into_iter()
            .map(|s| serde_json::from_str::<PartContent>(&s))
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .filter_map(|p| p.kind)
            .collect();
        let effective = EffectiveStandards {
            base: Standards::base().clone(),
            additions: Standards::current_in(&mut tx, project).await?,
        };
        let waived = super::map_versions::waived(&mut tx, project).await?;
        let required: Vec<_> = effective
            .mandatory_parts()
            .into_iter()
            .filter(|k| !waived.iter().any(|w| w == k))
            .collect();
        let mut stage = stage::compute(&vision, &kinds, &required);
        let map: Option<i64> =
            sqlx::query_scalar("SELECT map_revision FROM design_workspace WHERE project_id=?")
                .bind(project.as_str())
                .fetch_optional(&mut *tx)
                .await?;
        if matches!(stage.stage, crate::Stage::Map | crate::Stage::Structure) {
            stage.stage = if map.unwrap_or(0) == 0 {
                crate::Stage::Map
            } else {
                crate::Stage::Structure
            };
        }
        if map.unwrap_or(0) > 0 {
            let rows = super::map_versions::rows(&mut tx, project).await?;
            let content = super::map_versions::contents(&rows)?;
            let revisions: Vec<std::collections::BTreeMap<String, i64>> = rows
                .iter()
                .map(|r| serde_json::from_str(&r.served_revisions))
                .collect::<Result<_, _>>()?;
            let parts = super::map_versions::views(&rows, &content);
            let parts = parts.into_iter().zip(&revisions).collect::<Vec<_>>();
            stage.drift = crate::design::map_rules::drift(&parts, &vision);
        }
        let written: Option<String> = sqlx::query_scalar(
            "SELECT completed_at FROM design_structure_write WHERE project_id=?",
        )
        .bind(project.as_str())
        .fetch_optional(&mut *tx)
        .await?
        .flatten();
        if written.is_some() {
            let roadmap: Option<i64> = sqlx::query_scalar(
                "SELECT roadmap_revision FROM design_workspace WHERE project_id=?",
            )
            .bind(project.as_str())
            .fetch_optional(&mut *tx)
            .await?;
            stage.stage = if roadmap.unwrap_or(0) > 0 {
                crate::Stage::Plans
            } else {
                crate::Stage::Roadmap
            };
        }
        tx.commit().await?;
        Ok(stage)
    }
}
