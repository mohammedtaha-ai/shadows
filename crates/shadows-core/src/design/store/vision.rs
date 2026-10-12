//! Atomic workspace edits with immutable command results.

use crate::command::CommandContext;
use crate::db::{Storage, StorageError, append_event, classify, now, record_command};
use crate::design::{DesignAnchor, DesignChange, DesignOp, VisionContent, VisionView};
use crate::events::{Actor, DurableEvent};
use crate::projects::ProjectId;
use std::collections::BTreeSet;

impl Storage {
    pub async fn design_vision(&self, project: &ProjectId) -> Result<VisionView, StorageError> {
        let row: Option<(i64, Option<String>)> = sqlx::query_as(
            "SELECT COALESCE(w.revision, 0), w.vision_content FROM project p
             LEFT JOIN design_workspace w ON w.project_id = p.id
             WHERE p.id = ? AND p.removed_at IS NULL",
        )
        .bind(project.as_str())
        .fetch_optional(self.reader())
        .await?;
        let (revision, content) = row.ok_or(StorageError::NotFound("project"))?;
        Ok(VisionView {
            revision,
            content: content
                .map(|s| serde_json::from_str(&s))
                .transpose()?
                .unwrap_or_default(),
        })
    }

    pub async fn edit_design(
        &self,
        ctx: &CommandContext,
        project: &ProjectId,
        expected_revision: i64,
        ops: Vec<DesignOp>,
    ) -> Result<DesignChange, StorageError> {
        let (ctx, project, ts) = (ctx.clone(), project.clone(), now());
        self.write_txn(move |conn| {
            Box::pin(async move {
                if let Some(id) = classify(conn, &ctx, "Project", project.as_str()).await? {
                    let result: String = sqlx::query_scalar(
                        "SELECT result_json FROM design_command_result
                         WHERE id = ? AND project_id = ?",
                    )
                    .bind(id)
                    .bind(project.as_str())
                    .fetch_optional(&mut *conn)
                    .await?
                    .ok_or(StorageError::NotFound("design command result"))?;
                    return Ok(serde_json::from_str(&result)?);
                }
                let revision: Option<i64> = sqlx::query_scalar(
                    "SELECT COALESCE(w.revision, 0) FROM project p
                     LEFT JOIN design_workspace w ON w.project_id = p.id
                     WHERE p.id = ? AND p.removed_at IS NULL",
                )
                .bind(project.as_str())
                .fetch_optional(&mut *conn)
                .await?;
                let current = revision.ok_or(StorageError::NotFound("project"))?;
                if current != expected_revision {
                    return Err(StorageError::RevisionConflict {
                        current,
                        summary: "project design changed".into(),
                    });
                }
                if ops.is_empty() {
                    return Err(StorageError::Constraint(
                        "a design edit needs an operation".into(),
                    ));
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
                let before_parts = super::map_versions::rows(conn, &project).await?;
                let mut written = BTreeSet::new();
                let mut reviewed = BTreeSet::new();
                let touches_parts = ops.iter().any(|op| {
                    matches!(
                        op,
                        DesignOp::PartCreate { .. }
                            | DesignOp::PartPut { .. }
                            | DesignOp::PartMove { .. }
                            | DesignOp::PlanLinkPut {
                                anchor: DesignAnchor::Part(_),
                                ..
                            }
                            | DesignOp::PlanLinkRemove {
                                anchor: DesignAnchor::Part(_),
                                ..
                            }
                    )
                });
                if touches_parts {
                    let map: i64 = sqlx::query_scalar(
                        "SELECT map_revision FROM design_workspace WHERE project_id=?",
                    )
                    .bind(project.as_str())
                    .fetch_one(&mut *conn)
                    .await?;
                    if map == 0 {
                        return Err(StorageError::StageBlocked(
                            "the first map comes from the Planner: ask it to propose one".into(),
                        ));
                    }
                }
                for op in &ops {
                    match op {
                        DesignOp::PartCreate { id, .. } => {
                            written.insert(id.to_string());
                            reviewed.insert(id.to_string());
                        }
                        DesignOp::PartPut { id, content } => {
                            written.insert(id.to_string());
                            let old = before_parts
                                .iter()
                                .find(|r| r.id == id.as_str())
                                .map(|r| {
                                    serde_json::from_str::<crate::PartContent>(&r.content_json)
                                })
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
                        other @ (DesignOp::VisionItemAdd { .. }
                        | DesignOp::VisionItemPut { .. }
                        | DesignOp::VisionItemRemove { .. }) => vision.push(other),
                        other @ (DesignOp::PartCreate { .. }
                        | DesignOp::PartPut { .. }
                        | DesignOp::PartMove { .. }
                        | DesignOp::PlanLinkPut {
                            anchor: DesignAnchor::Part(_),
                            ..
                        }
                        | DesignOp::PlanLinkRemove {
                            anchor: DesignAnchor::Part(_),
                            ..
                        }) => {
                            if let DesignOp::PartCreate { id, .. } = &other {
                                created.insert(id.to_string());
                            }
                            super::part_edit::apply(conn, &project, other, &mut changed).await?;
                        }
                        other @ (DesignOp::OutcomeCreate { .. }
                        | DesignOp::OutcomePut { .. }
                        | DesignOp::OutcomeMove { .. }
                        | DesignOp::OutcomePartPut { .. }
                        | DesignOp::OutcomePartRemove { .. }
                        | DesignOp::PlanLinkPut {
                            anchor: DesignAnchor::Outcome(_),
                            ..
                        }
                        | DesignOp::PlanLinkRemove {
                            anchor: DesignAnchor::Outcome(_),
                            ..
                        }) => {
                            if let DesignOp::OutcomeCreate { id, .. } = &other {
                                created_outcomes.insert(id.to_string());
                            }
                            super::outcome_edit::apply(
                                conn,
                                &project,
                                other,
                                &mut changed_outcomes,
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
                            DesignOp::VisionItemAdd { field, .. } => Some(
                                super::super::vision_items::next_id(*field, &mut counters)
                                    .map_err(StorageError::Constraint)?,
                            ),
                            _ => None,
                        };
                        super::super::vision_items::apply(&mut content, op, id, None)
                            .map_err(StorageError::Constraint)?;
                    }
                    let number = number.checked_add(1).ok_or_else(|| {
                        StorageError::Unavailable("vision revision exhausted".into())
                    })?;
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
                    super::vision_versions::insert_version(
                        conn, &project, number, &content, None, &ts,
                    )
                    .await?;
                }
                for id in changed.iter().filter(|id| !created.contains(*id)) {
                    sqlx::query(
                        "UPDATE design_part SET revision=revision+1 WHERE project_id=? AND id=?",
                    )
                    .bind(project.as_str())
                    .bind(id)
                    .execute(&mut *conn)
                    .await?;
                }
                if touches_parts {
                    let content = super::proposal_read::live_vision(conn, &project).await?;
                    let rows = super::map_versions::rows(conn, &project).await?;
                    let contents = super::map_versions::contents(&rows)?;
                    let parts = super::map_versions::views(&rows, &contents);
                    let standards = super::map_versions::effective(conn, &project).await?;
                    let waived = super::map_versions::waived(conn, &project).await?;
                    crate::design::map_rules::check_shape(
                        &parts,
                        &written.iter().map(String::as_str).collect::<Vec<_>>(),
                        &content,
                        &standards,
                    )
                    .map_err(StorageError::Constraint)?;
                    let old_contents = super::map_versions::contents(&before_parts)?;
                    let old_parts = super::map_versions::views(&before_parts, &old_contents);
                    let old_missing =
                        crate::design::map_rules::missing_kinds(&old_parts, &waived, &standards);
                    let missing =
                        crate::design::map_rules::missing_kinds(&parts, &waived, &standards);
                    let newly: Vec<_> = missing
                        .into_iter()
                        .filter(|k| !old_missing.contains(k))
                        .collect();
                    if !newly.is_empty() {
                        return Err(StorageError::StageBlocked(format!(
                            "missing part: {}",
                            newly.join(", ")
                        )));
                    }
                    super::map_versions::review(conn, &project, &reviewed, &content).await?;
                    super::map_versions::insert_version(conn, &project, None, &ts).await?;
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
                let event =
                    DurableEvent::new("ProjectDesignChanged", Actor::user(&ctx.principal_id))
                        .with_project(&project)
                        .with_payload(serde_json::json!({
                            "project_id": project, "revision": next,
                            "changed_parts": changed,
                            "changed_outcomes": changed_outcomes,
                            "vision_changed": vision_changed
                        }));
                append_event(conn, &event, &ts).await?;
                record_command(
                    conn,
                    &ctx,
                    "Project",
                    project.as_str(),
                    "DesignChange",
                    &id,
                    &ts,
                )
                .await?;
                Ok(change)
            })
        })
        .await
    }
}
