//! One-snapshot reads of accepted map parts and effective standards.
use super::map_versions::MapVersionsRows;
use crate::map_rules::MapRules;
use crate::{Design, DesignError, LayoutLayer, LayoutPart, PartId, ProjectLayout, RuleText};
use core_projects::ProjectId;
use sqlx::SqliteConnection;

impl Design {
    pub async fn layout(&self, project: &ProjectId) -> Result<ProjectLayout, DesignError> {
        let mut tx = self.store.storage.reader().begin().await?;
        let layout = Self::layout_in(&mut tx, project).await?;
        tx.commit().await?;
        Ok(layout)
    }

    pub async fn layout_in(
        conn: &mut SqliteConnection,
        project: &ProjectId,
    ) -> Result<ProjectLayout, DesignError> {
        core_projects::Projects::require_live_in(conn, project).await?;
        let map_revision =
            sqlx::query_scalar("SELECT map_revision FROM design_workspace WHERE project_id=?")
                .bind(project.as_str())
                .fetch_optional(&mut *conn)
                .await?
                .unwrap_or(0);
        let rows = MapVersionsRows::rows(conn, project).await?;
        let contents = MapVersionsRows::contents(&rows)?;
        let views = MapVersionsRows::views(&rows, &contents);
        let effective = MapVersionsRows::effective(conn, project).await?;
        let mut parts = Vec::new();
        for (row, view) in rows.iter().zip(&views) {
            let content = view.content;
            let kind = content.kind.as_deref().unwrap_or_default().trim();
            let catalog = effective.layers(kind);
            let mut layers = Vec::new();
            for layer in &content.layers {
                let spec = catalog
                    .iter()
                    .find(|s| s.id == layer.layer)
                    .ok_or_else(|| {
                        DesignError::InvalidCommand(format!(
                            "part {} has unknown layer {}",
                            content.title, layer.layer
                        ))
                    })?;
                layers.push(LayoutLayer {
                    id: spec.id.clone(),
                    role: spec.role.clone(),
                    place: (!layer.place.is_empty()).then(|| layer.place.clone()),
                    not_used: layer.not_used.clone(),
                    may_use: spec.may_use.clone(),
                });
            }
            let rules = effective
                .base
                .rules
                .iter()
                .chain(effective.additions.iter().flat_map(|a| &a.content.rules))
                .filter(|r| r.parts.is_empty() || r.parts.iter().any(|p| p == kind))
                .map(|r| RuleText {
                    id: r.id.clone(),
                    text: r.text.clone(),
                })
                .collect();
            parts.push(LayoutPart {
                id: PartId::from_recorded(row.id.clone()),
                parent: row.parent_id.clone().map(PartId::from_recorded),
                title: content.title.clone(),
                kind: content.kind.clone(),
                folder: content.folder.clone(),
                grouping: MapRules::is_grouping(view, &views),
                layers,
                rules,
            });
        }
        Ok(ProjectLayout {
            map_revision,
            standards_base: effective.base.version,
            standards_additions: effective.additions.map(|a| a.number),
            parts,
        })
    }
}
