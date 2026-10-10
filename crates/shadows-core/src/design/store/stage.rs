//! Reading every stage input in one consistent project snapshot.

use crate::db::{Storage, StorageError};
use crate::design::{EffectiveStandards, PartContent, StageView, VisionContent, stage, standards};
use crate::projects::ProjectId;

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
            base: standards::base().clone(),
            additions: super::standards::current_in(&mut tx, project).await?,
        };
        let stage = stage::compute(&vision, &kinds, &effective.mandatory_parts());
        tx.commit().await?;
        Ok(stage)
    }
}
