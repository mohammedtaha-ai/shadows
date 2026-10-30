//! Reading every stage input in one consistent project snapshot.

use super::DesignStore;
use crate::{DesignError, PartContent, Stage, StageView, VisionContent, stage, structure};
use core_db::StorageError;
use core_projects::ProjectId;
use core_standards::{EffectiveStandards, Standards};

impl DesignStore {
    pub async fn project_stage(&self, project: &ProjectId) -> Result<StageView, StorageError> {
        let mut tx = self.storage.reader().begin().await?;
        let view = crate::Design::stage_view_in(&mut tx, project).await?;
        tx.commit().await?;
        Ok(view)
    }
}
impl crate::Design {
    pub async fn stage_view_in(
        tx: &mut sqlx::SqliteConnection,
        project: &ProjectId,
    ) -> Result<StageView, StorageError> {
        core_projects::Projects::require_live_in(tx, project).await?;
        let content: Option<Option<String>> =
            sqlx::query_scalar("SELECT vision_content FROM design_workspace WHERE project_id=?")
                .bind(project.as_str())
                .fetch_optional(&mut *tx)
                .await?;
        let content = content.flatten();
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
            additions: Standards::current_in(tx, project).await?,
        };
        let waived = super::map_versions::MapVersionsRows::waived(tx, project).await?;
        let required: Vec<_> = effective
            .mandatory_parts()
            .into_iter()
            .filter(|k| !waived.iter().any(|w| w == k))
            .collect();
        let mut stage = stage::StageRules::compute(&vision, &kinds, &required);
        let map: Option<i64> =
            sqlx::query_scalar("SELECT map_revision FROM design_workspace WHERE project_id=?")
                .bind(project.as_str())
                .fetch_optional(&mut *tx)
                .await?;
        if map.unwrap_or(0) > 0 {
            let rows = super::map_versions::MapVersionsRows::rows(tx, project).await?;
            let content = super::map_versions::MapVersionsRows::contents(&rows)?;
            let revisions: Vec<std::collections::BTreeMap<String, i64>> = rows
                .iter()
                .map(|r| serde_json::from_str(&r.served_revisions))
                .collect::<Result<_, _>>()?;
            let parts = super::map_versions::MapVersionsRows::views(&rows, &content);
            let parts = parts.into_iter().zip(&revisions).collect::<Vec<_>>();
            stage.drift = crate::map_rules::MapRules::drift(&parts, &vision);
        }
        let written: Option<String> = sqlx::query_scalar(
            "SELECT completed_at FROM design_structure_write WHERE project_id=?",
        )
        .bind(project.as_str())
        .fetch_optional(&mut *tx)
        .await?
        .flatten();
        let roadmap: Option<i64> = if written.is_some() {
            sqlx::query_scalar("SELECT roadmap_revision FROM design_workspace WHERE project_id=?")
                .bind(project.as_str())
                .fetch_optional(&mut *tx)
                .await?
        } else {
            None
        };
        let mut stage = stage::StageRules::with_workspace(
            stage,
            map.unwrap_or(0),
            written.is_some(),
            roadmap.unwrap_or(0),
        );
        stage.map_revision = map.unwrap_or(0);
        if stage.stage == Stage::Structure {
            stage.structure_file_count = Self::structure_file_count_in(tx, project).await?;
        }
        Ok(stage)
    }

    /// The files `create_structure` would write now, planned the way it plans them;
    /// none when it would refuse.
    async fn structure_file_count_in(
        tx: &mut sqlx::SqliteConnection,
        project: &ProjectId,
    ) -> Result<Option<u32>, StorageError> {
        let snapshot = match DesignStore::structure_snapshot_in(tx, project, None).await {
            Ok(snapshot) => snapshot,
            Err(DesignError::Storage(error)) => return Err(error),
            Err(_) => return Ok(None),
        };
        Ok(structure::Structure::plan(&snapshot)
            .ok()
            .and_then(|plan| u32::try_from(plan.file_count()).ok()))
    }
}
