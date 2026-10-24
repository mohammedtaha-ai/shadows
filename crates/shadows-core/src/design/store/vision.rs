//! Atomic workspace edits with immutable command results.

use super::DesignStore;
use crate::design::{DesignAnchor, DesignChange, VisionContent, VisionView, WorkspaceOp};
use crate::events::DurableEvent;
use crate::projects::ProjectId;
use core_db::{Actor, CommandContext, JournalEntry, Storage, StorageError};
use std::collections::BTreeSet;

impl DesignStore {
    pub async fn design_vision(&self, project: &ProjectId) -> Result<VisionView, StorageError> {
        let mut tx = self.storage.reader().begin().await?;
        crate::projects::Projects::require_live_in(&mut tx, project).await?;
        let row: Option<(i64, Option<String>)> = sqlx::query_as(
            "SELECT revision,vision_content FROM design_workspace WHERE project_id=?",
        )
        .bind(project.as_str())
        .fetch_optional(&mut *tx)
        .await?;
        let (revision, content) = row.unwrap_or((0, None));
        Ok(VisionView {
            revision,
            content: content
                .map(|s| serde_json::from_str(&s))
                .transpose()?
                .unwrap_or_default(),
        })
    }
}

impl crate::design::Design {
    pub(crate) fn normalize_ops(ops: &mut [WorkspaceOp]) -> Result<(), crate::CoreError> {
        crate::design::ops::Ops::normalize(ops)
    }
    pub(crate) async fn replay_edit_in(
        conn: &mut sqlx::SqliteConnection,
        ctx: &CommandContext,
        project: &ProjectId,
    ) -> Result<Option<DesignChange>, StorageError> {
        if let Some(id) = CommandContext::classify(conn, ctx, "Project", project.as_str()).await? {
            let result: String = sqlx::query_scalar(
                "SELECT result_json FROM design_command_result
                         WHERE id = ? AND project_id = ?",
            )
            .bind(id)
            .bind(project.as_str())
            .fetch_optional(&mut *conn)
            .await?
            .ok_or(StorageError::NotFound("design command result"))?;
            return Ok(Some(serde_json::from_str(&result)?));
        }
        Ok(None)
    }
    pub(crate) async fn edit_workspace_in(
        conn: &mut sqlx::SqliteConnection,
        ctx: &CommandContext,
        project: &ProjectId,
        expected_revision: i64,
        ops: Vec<WorkspaceOp>,
        facts: &std::collections::BTreeMap<String, Option<String>>,
    ) -> Result<DesignChange, crate::CoreError> {
        let ts = Storage::now();
        let current = super::parts::PartsRows::revision(conn, project).await?;
        if current != expected_revision {
            return Err(StorageError::RevisionConflict {
                current,
                summary: "project design changed".into(),
            }
            .into());
        }
        if ops.is_empty() {
            return Err(StorageError::Constraint("a design edit needs an operation".into()).into());
        }
        let next = current
            .checked_add(1)
            .ok_or_else(|| StorageError::Unavailable("design revision exhausted".into()))?;
        sqlx::query(
            "INSERT INTO design_workspace
                       (project_id, revision, vision_revision, vision_content) VALUES (?,?,?,?)
                     ON CONFLICT(project_id) DO UPDATE SET revision = excluded.revision",
        )
        .bind(project.as_str())
        .bind(next)
        .bind(0_i64)
        .bind(serde_json::to_string(&VisionContent::default())?)
        .execute(&mut *conn)
        .await?;
        let before_parts = super::map_versions::MapVersionsRows::rows(conn, project).await?;
        let mut written = BTreeSet::new();
        let mut reviewed = BTreeSet::new();
        let touches_parts = ops.iter().any(|op| {
            matches!(
                op,
                WorkspaceOp::PartCreate { .. }
                    | WorkspaceOp::PartPut { .. }
                    | WorkspaceOp::PartMove { .. }
                    | WorkspaceOp::PlanLinkPut {
                        anchor: DesignAnchor::Part(_),
                        ..
                    }
                    | WorkspaceOp::PlanLinkRemove {
                        anchor: DesignAnchor::Part(_),
                        ..
                    }
            )
        });
        if touches_parts {
            let map: i64 =
                sqlx::query_scalar("SELECT map_revision FROM design_workspace WHERE project_id=?")
                    .bind(project.as_str())
                    .fetch_one(&mut *conn)
                    .await?;
            if map == 0 {
                return Err(crate::design::DesignError::StageBlocked(
                    "the first map comes from the Planner: ask it to propose one".into(),
                )
                .into());
            }
        }
        super::roadmap_gate::RoadmapGateRows::check_hand_edit(conn, project, &ops).await?;
        for op in &ops {
            match op {
                WorkspaceOp::PartCreate { id, .. } => {
                    written.insert(id.to_string());
                    reviewed.insert(id.to_string());
                }
                WorkspaceOp::PartPut { id, content } => {
                    written.insert(id.to_string());
                    let old = before_parts
                        .iter()
                        .find(|r| r.id == id.as_str())
                        .map(|r| serde_json::from_str::<crate::PartContent>(&r.content_json))
                        .transpose()?;
                    if old.is_none_or(|c| {
                        c.serves != content.serves
                            || c.responsibility != content.responsibility
                            || c.design != content.design
                    }) {
                        reviewed.insert(id.to_string());
                    }
                }
                _ => {}
            }
        }
        let mut changed = BTreeSet::new();
        let mut created = BTreeSet::new();
        let mut changed_outcomes = BTreeSet::new();
        let mut created_outcomes = BTreeSet::new();
        let mut vision = Vec::new();
        for op in ops {
            match op {
                other @ (WorkspaceOp::VisionItemAdd { .. }
                | WorkspaceOp::VisionItemPut { .. }
                | WorkspaceOp::VisionItemRemove { .. }) => vision.push(other),
                other @ (WorkspaceOp::PartCreate { .. }
                | WorkspaceOp::PartPut { .. }
                | WorkspaceOp::PartMove { .. }
                | WorkspaceOp::PlanLinkPut {
                    anchor: DesignAnchor::Part(_),
                    ..
                }
                | WorkspaceOp::PlanLinkRemove {
                    anchor: DesignAnchor::Part(_),
                    ..
                }) => {
                    if let WorkspaceOp::PartCreate { id, .. } = &other {
                        created.insert(id.to_string());
                    }
                    super::part_edit::PartEditRows::apply(
                        conn,
                        project,
                        other,
                        &mut changed,
                        facts,
                    )
                    .await?;
                }
                other @ (WorkspaceOp::OutcomeCreate { .. }
                | WorkspaceOp::OutcomePut { .. }
                | WorkspaceOp::OutcomeMove { .. }
                | WorkspaceOp::OutcomePartPut { .. }
                | WorkspaceOp::OutcomePartRemove { .. }
                | WorkspaceOp::PlanLinkPut {
                    anchor: DesignAnchor::Outcome(_),
                    ..
                }
                | WorkspaceOp::PlanLinkRemove {
                    anchor: DesignAnchor::Outcome(_),
                    ..
                }) => {
                    if let WorkspaceOp::OutcomeCreate { id, .. } = &other {
                        created_outcomes.insert(id.to_string());
                    }
                    super::outcome_edit::OutcomeEditRows::apply(
                        conn,
                        project,
                        other,
                        &mut changed_outcomes,
                        facts,
                    )
                    .await?;
                }
            }
        }
        let vision_changed = !vision.is_empty();
        if vision_changed {
            let (json, counter_json, number): (String, String, i64) = sqlx::query_as(
                "SELECT vision_content,vision_counters,vision_revision
                         FROM design_workspace WHERE project_id=?",
            )
            .bind(project.as_str())
            .fetch_one(&mut *conn)
            .await?;
            let mut content: VisionContent = serde_json::from_str(&json)?;
            let mut counters = serde_json::from_str(&counter_json)?;
            for op in &vision {
                let id = match op {
                    WorkspaceOp::VisionItemAdd { field, .. } => Some(
                        super::super::vision_items::VisionItems::next_id(*field, &mut counters)
                            .map_err(StorageError::Constraint)?,
                    ),
                    _ => None,
                };
                super::super::vision_items::VisionItems::apply(&mut content, op, id, None)
                    .map_err(StorageError::Constraint)?;
            }
            let number = number
                .checked_add(1)
                .ok_or_else(|| StorageError::Unavailable("vision revision exhausted".into()))?;
            sqlx::query(
                "UPDATE design_workspace SET vision_content=?,vision_counters=?, \
                         vision_revision=? WHERE project_id=?",
            )
            .bind(serde_json::to_string(&content)?)
            .bind(serde_json::to_string(&counters)?)
            .bind(number)
            .bind(project.as_str())
            .execute(&mut *conn)
            .await?;
            super::vision_versions::VisionVersionsRows::insert_version(
                conn, project, number, &content, None, &ts,
            )
            .await?;
        }
        for id in changed.iter().filter(|id| !created.contains(*id)) {
            sqlx::query("UPDATE design_part SET revision=revision+1 WHERE project_id=? AND id=?")
                .bind(project.as_str())
                .bind(id)
                .execute(&mut *conn)
                .await?;
        }
        if touches_parts {
            let content =
                super::proposal_read::ProposalReadRows::live_vision(conn, project).await?;
            let rows = super::map_versions::MapVersionsRows::rows(conn, project).await?;
            let contents = super::map_versions::MapVersionsRows::contents(&rows)?;
            let parts = super::map_versions::MapVersionsRows::views(&rows, &contents);
            let standards = super::map_versions::MapVersionsRows::effective(conn, project).await?;
            let waived = super::map_versions::MapVersionsRows::waived(conn, project).await?;
            crate::design::map_rules::MapRules::check_shape(
                &parts,
                &written.iter().map(String::as_str).collect::<Vec<_>>(),
                &content,
                &standards,
            )
            .map_err(StorageError::Constraint)?;
            let old_contents = super::map_versions::MapVersionsRows::contents(&before_parts)?;
            let old_parts =
                super::map_versions::MapVersionsRows::views(&before_parts, &old_contents);
            let old_missing =
                crate::design::map_rules::MapRules::missing_kinds(&old_parts, &waived, &standards);
            let missing =
                crate::design::map_rules::MapRules::missing_kinds(&parts, &waived, &standards);
            let newly: Vec<_> = missing
                .into_iter()
                .filter(|k| !old_missing.contains(k))
                .collect();
            if !newly.is_empty() {
                return Err(crate::design::DesignError::StageBlocked(format!(
                    "missing part: {}",
                    newly.join(", ")
                ))
                .into());
            }
            super::map_versions::MapVersionsRows::review(conn, project, &reviewed, &content)
                .await?;
            super::map_versions::MapVersionsRows::insert_version(conn, project, None, &ts).await?;
        }
        for id in changed_outcomes
            .iter()
            .filter(|id| !created_outcomes.contains(*id))
        {
            sqlx::query(
                "UPDATE design_outcome SET revision=revision+1 \
                         WHERE project_id=? AND id=?",
            )
            .bind(project.as_str())
            .bind(id)
            .execute(&mut *conn)
            .await?;
        }
        let change = DesignChange { revision: next };
        let id = uuid::Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO design_command_result (id, project_id, result_json) \
                     VALUES (?,?,?)",
        )
        .bind(&id)
        .bind(project.as_str())
        .bind(serde_json::to_string(&change)?)
        .execute(&mut *conn)
        .await?;
        let event = DurableEvent::new("ProjectDesignChanged", Actor::user(&ctx.principal_id))
            .with_project(project)
            .with_payload(serde_json::json!({
                "project_id": project, "revision": next,
                "changed_parts": changed,
                "changed_outcomes": changed_outcomes,
                "vision_changed": vision_changed
            }));
        JournalEntry::append_event(conn, event.journal_entry(), &ts).await?;
        CommandContext::record_command(
            conn,
            ctx,
            "Project",
            project.as_str(),
            "DesignChange",
            &id,
            &ts,
        )
        .await?;
        Ok(change)
    }
}
