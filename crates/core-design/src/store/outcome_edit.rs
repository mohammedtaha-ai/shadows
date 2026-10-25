//! Transactional mutations of roadmap outcomes and their associations.
use super::{
    hierarchy::{self, Tree},
    outcomes::OutcomesRows,
};
use crate::{DesignAnchor, OutcomeId, ProjectId, WorkspaceOp};
use core_db::StorageError;
use sqlx::SqliteConnection;
use std::collections::BTreeSet;
async fn require(
    conn: &mut SqliteConnection,
    project: &ProjectId,
    id: &OutcomeId,
) -> Result<(), StorageError> {
    OutcomesRows::load(conn, project, id)
        .await
        .map(|_| ())
        .map_err(|e| match e {
            StorageError::NotFound(_) => StorageError::Constraint(format!(
                "outcome {id} does not belong to project {project}"
            )),
            other => other,
        })
}
impl OutcomeEditRows {
    pub(super) async fn apply(
        conn: &mut SqliteConnection,
        project: &ProjectId,
        op: WorkspaceOp,
        changed: &mut BTreeSet<String>,
        facts: &std::collections::BTreeMap<String, Option<String>>,
    ) -> Result<(), StorageError> {
        let remove = matches!(
            &op,
            WorkspaceOp::PlanLinkRemove { .. } | WorkspaceOp::OutcomePartRemove { .. }
        );
        match op {
            WorkspaceOp::OutcomeCreate {
                id,
                parent,
                before,
                content,
            } => {
                hierarchy::HierarchyRows::check_parent(
                    conn,
                    Tree::Outcomes,
                    project,
                    id.as_str(),
                    parent.as_ref().map(OutcomeId::as_str),
                )
                .await?;
                sqlx::query(
                "INSERT INTO design_outcome(id,project_id,parent_id,revision,ordinal,content_json) \
                 VALUES (?,?,?,1,0,?)",
            )
            .bind(id.as_str())
            .bind(project.as_str())
            .bind(parent.as_ref().map(OutcomeId::as_str))
            .bind(serde_json::to_string(&content)?)
            .execute(&mut *conn)
            .await?;
                hierarchy::HierarchyRows::place(
                    conn,
                    Tree::Outcomes,
                    project,
                    id.as_str(),
                    parent.as_ref().map(OutcomeId::as_str),
                    before.as_ref().map(OutcomeId::as_str),
                    changed,
                )
                .await?;
                changed.insert(id.to_string());
            }
            WorkspaceOp::OutcomePut { id, content } => {
                require(conn, project, &id).await?;
                sqlx::query("UPDATE design_outcome SET content_json=? WHERE project_id=? AND id=?")
                    .bind(serde_json::to_string(&content)?)
                    .bind(project.as_str())
                    .bind(id.as_str())
                    .execute(&mut *conn)
                    .await?;
                changed.insert(id.to_string());
            }
            WorkspaceOp::OutcomeMove { id, parent, before } => {
                require(conn, project, &id).await?;
                hierarchy::HierarchyRows::check_parent(
                    conn,
                    Tree::Outcomes,
                    project,
                    id.as_str(),
                    parent.as_ref().map(OutcomeId::as_str),
                )
                .await?;
                sqlx::query("UPDATE design_outcome SET parent_id=? WHERE project_id=? AND id=?")
                    .bind(parent.as_ref().map(OutcomeId::as_str))
                    .bind(project.as_str())
                    .bind(id.as_str())
                    .execute(&mut *conn)
                    .await?;
                hierarchy::HierarchyRows::place(
                    conn,
                    Tree::Outcomes,
                    project,
                    id.as_str(),
                    parent.as_ref().map(OutcomeId::as_str),
                    before.as_ref().map(OutcomeId::as_str),
                    changed,
                )
                .await?;
                changed.insert(id.to_string());
            }
            WorkspaceOp::PlanLinkPut {
                anchor: DesignAnchor::Outcome(id),
                plan,
            }
            | WorkspaceOp::PlanLinkRemove {
                anchor: DesignAnchor::Outcome(id),
                plan,
            } => {
                require(conn, project, &id).await?;
                if let Some(message) = &facts[&plan] {
                    return Err(StorageError::Constraint(message.clone()));
                }
                changed.insert(id.to_string());
            }
            WorkspaceOp::OutcomePartPut { outcome, part }
            | WorkspaceOp::OutcomePartRemove { outcome, part } => {
                require(conn, project, &outcome).await?;
                super::parts::PartsRows::load(conn, project, &part)
                    .await
                    .map_err(|e| match e {
                        StorageError::NotFound(_) => StorageError::Constraint(format!(
                            "part {part} does not belong to project {project}"
                        )),
                        other => other,
                    })?;
                let query = if remove {
                    sqlx::query(
                        "DELETE FROM design_outcome_part \
                         WHERE project_id=? AND outcome_id=? AND part_id=?",
                    )
                } else {
                    sqlx::query(
                        "INSERT OR IGNORE INTO design_outcome_part \
                             (project_id,outcome_id,part_id) VALUES (?,?,?)",
                    )
                };
                query
                    .bind(project.as_str())
                    .bind(outcome.as_str())
                    .bind(part.as_str())
                    .execute(&mut *conn)
                    .await?;
                changed.insert(outcome.to_string());
            }
            _ => unreachable!("only outcome operations are dispatched here"),
        }
        Ok(())
    }
}

pub(crate) struct OutcomeEditRows;
