//! Transactional mutations of part content, containment and plan associations.
use super::{
    hierarchy::{self, Tree},
    parts::load,
};
use crate::{
    ProjectId,
    db::StorageError,
    design::{DesignAnchor, DesignOp, PartId},
};
use sqlx::SqliteConnection;
use std::collections::BTreeSet;

pub(super) async fn apply(
    conn: &mut SqliteConnection,
    project: &ProjectId,
    op: DesignOp,
    changed: &mut BTreeSet<String>,
) -> Result<(), StorageError> {
    let remove = matches!(&op, DesignOp::PlanLinkRemove { .. });
    match op {
        DesignOp::PartCreate {
            id,
            parent,
            before,
            content,
        } => {
            hierarchy::check_parent(
                conn,
                Tree::Parts,
                project,
                id.as_str(),
                parent.as_ref().map(PartId::as_str),
            )
            .await?;
            sqlx::query(
                "INSERT INTO design_part(id,project_id,parent_id,revision,ordinal,content_json) \
                 VALUES (?,?,?,1,0,?)",
            )
            .bind(id.as_str())
            .bind(project.as_str())
            .bind(parent.as_ref().map(PartId::as_str))
            .bind(serde_json::to_string(&content)?)
            .execute(&mut *conn)
            .await?;
            hierarchy::place(
                conn,
                Tree::Parts,
                project,
                id.as_str(),
                parent.as_ref().map(PartId::as_str),
                before.as_ref().map(PartId::as_str),
                changed,
            )
            .await?;
            changed.insert(id.to_string());
        }
        DesignOp::PartPut { id, content } => {
            require(conn, project, &id).await?;
            sqlx::query("UPDATE design_part SET content_json=? WHERE project_id=? AND id=?")
                .bind(serde_json::to_string(&content)?)
                .bind(project.as_str())
                .bind(id.as_str())
                .execute(&mut *conn)
                .await?;
            changed.insert(id.to_string());
        }
        DesignOp::PartMove { id, parent, before } => {
            require(conn, project, &id).await?;
            hierarchy::check_parent(
                conn,
                Tree::Parts,
                project,
                id.as_str(),
                parent.as_ref().map(PartId::as_str),
            )
            .await?;
            sqlx::query("UPDATE design_part SET parent_id=? WHERE project_id=? AND id=?")
                .bind(parent.as_ref().map(PartId::as_str))
                .bind(project.as_str())
                .bind(id.as_str())
                .execute(&mut *conn)
                .await?;
            hierarchy::place(
                conn,
                Tree::Parts,
                project,
                id.as_str(),
                parent.as_ref().map(PartId::as_str),
                before.as_ref().map(PartId::as_str),
                changed,
            )
            .await?;
            changed.insert(id.to_string());
        }
        DesignOp::PlanLinkPut {
            anchor: DesignAnchor::Part(id),
            plan,
        }
        | DesignOp::PlanLinkRemove {
            anchor: DesignAnchor::Part(id),
            plan,
        } => {
            require(conn, project, &id).await?;
            crate::plans::check_design_plan(conn, project, &plan).await?;
            let sql = if remove {
                "DELETE FROM design_part_plan WHERE project_id=? AND part_id=? AND plan_id=?"
            } else {
                "INSERT OR IGNORE INTO design_part_plan(project_id,part_id,plan_id) VALUES (?,?,?)"
            };
            sqlx::query(sql)
                .bind(project.as_str())
                .bind(id.as_str())
                .bind(plan.as_str())
                .execute(&mut *conn)
                .await?;
            changed.insert(id.to_string());
        }
        _ => unreachable!("only part operations are dispatched here"),
    }
    Ok(())
}

async fn require(
    conn: &mut SqliteConnection,
    project: &ProjectId,
    id: &PartId,
) -> Result<(), StorageError> {
    load(conn, project, id)
        .await
        .map(|_| ())
        .map_err(|error| match error {
            StorageError::NotFound(_) => {
                StorageError::Constraint(format!("part {id} does not belong to project {project}"))
            }
            other => other,
        })
}
