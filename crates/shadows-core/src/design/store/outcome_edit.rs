//! Transactional mutations of roadmap outcomes and their associations.
use super::{
    hierarchy::{self, Tree},
    outcomes::load,
};
use crate::{
    ProjectId,
    db::StorageError,
    design::{DesignAnchor, DesignOp, OutcomeId},
};
use sqlx::SqliteConnection;
use std::collections::BTreeSet;
async fn require(
    conn: &mut SqliteConnection,
    project: &ProjectId,
    id: &OutcomeId,
) -> Result<(), StorageError> {
    load(conn, project, id)
        .await
        .map(|_| ())
        .map_err(|e| match e {
            StorageError::NotFound(_) => StorageError::Constraint(format!(
                "outcome {id} does not belong to project {project}"
            )),
            other => other,
        })
}
pub(super) async fn apply(
    conn: &mut SqliteConnection,
    project: &ProjectId,
    op: DesignOp,
    changed: &mut BTreeSet<String>,
) -> Result<(), StorageError> {
    let remove = matches!(
        &op,
        DesignOp::PlanLinkRemove { .. } | DesignOp::OutcomePartRemove { .. }
    );
    match op {
        DesignOp::OutcomeCreate {
            id,
            parent,
            before,
            content,
        } => {
            hierarchy::check_parent(
                conn,
                Tree::Outcomes,
                project,
                id.as_str(),
                parent.as_ref().map(OutcomeId::as_str),
            )
            .await?;
            sqlx::query("INSERT INTO design_outcome(id,project_id,parent_id,revision,ordinal,content_json) VALUES (?,?,?,1,0,?)").bind(id.as_str()).bind(project.as_str()).bind(parent.as_ref().map(OutcomeId::as_str)).bind(serde_json::to_string(&content)?).execute(&mut *conn).await?;
            hierarchy::place(
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
        DesignOp::OutcomePut { id, content } => {
            require(conn, project, &id).await?;
            sqlx::query("UPDATE design_outcome SET content_json=? WHERE project_id=? AND id=?")
                .bind(serde_json::to_string(&content)?)
                .bind(project.as_str())
                .bind(id.as_str())
                .execute(&mut *conn)
                .await?;
            changed.insert(id.to_string());
        }
        DesignOp::OutcomeMove { id, parent, before } => {
            require(conn, project, &id).await?;
            hierarchy::check_parent(
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
            hierarchy::place(
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
        DesignOp::PlanLinkPut {
            anchor: DesignAnchor::Outcome(id),
            plan,
        }
        | DesignOp::PlanLinkRemove {
            anchor: DesignAnchor::Outcome(id),
            plan,
        } => {
            require(conn, project, &id).await?;
            crate::plans::check_design_plan(conn, project, &plan).await?;
            let sql = if remove {
                "DELETE FROM design_outcome_plan WHERE project_id=? AND outcome_id=? AND plan_id=?"
            } else {
                "INSERT OR IGNORE INTO design_outcome_plan(project_id,outcome_id,plan_id) VALUES (?,?,?)"
            };
            sqlx::query(sql)
                .bind(project.as_str())
                .bind(id.as_str())
                .bind(plan.as_str())
                .execute(&mut *conn)
                .await?;
            changed.insert(id.to_string());
        }
        DesignOp::OutcomePartPut { outcome, part }
        | DesignOp::OutcomePartRemove { outcome, part } => {
            require(conn, project, &outcome).await?;
            super::parts::load(conn, project, &part)
                .await
                .map_err(|e| match e {
                    StorageError::NotFound(_) => StorageError::Constraint(format!(
                        "part {part} does not belong to project {project}"
                    )),
                    other => other,
                })?;
            let sql = if remove {
                "DELETE FROM design_outcome_part WHERE project_id=? AND outcome_id=? AND part_id=?"
            } else {
                "INSERT OR IGNORE INTO design_outcome_part(project_id,outcome_id,part_id) VALUES (?,?,?)"
            };
            sqlx::query(sql)
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
