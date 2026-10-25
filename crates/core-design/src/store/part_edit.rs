//! Transactional mutations of part content, containment and plan associations.
use super::{
    hierarchy::{self, Tree},
    parts::PartsRows,
};
use crate::{DesignAnchor, PartId, ProjectId, WorkspaceOp};
use core_db::StorageError;
use sqlx::SqliteConnection;
use std::collections::BTreeSet;

impl PartEditRows {
    pub(super) async fn apply(
        conn: &mut SqliteConnection,
        project: &ProjectId,
        op: WorkspaceOp,
        changed: &mut BTreeSet<String>,
        facts: &std::collections::BTreeMap<String, Option<String>>,
    ) -> Result<(), StorageError> {
        match op {
            WorkspaceOp::PartCreate {
                id,
                parent,
                before,
                content,
            } => {
                hierarchy::HierarchyRows::check_parent(
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
                hierarchy::HierarchyRows::place(
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
            WorkspaceOp::PartPut { id, content } => {
                require(conn, project, &id).await?;
                sqlx::query("UPDATE design_part SET content_json=? WHERE project_id=? AND id=?")
                    .bind(serde_json::to_string(&content)?)
                    .bind(project.as_str())
                    .bind(id.as_str())
                    .execute(&mut *conn)
                    .await?;
                changed.insert(id.to_string());
            }
            WorkspaceOp::PartMove { id, parent, before } => {
                require(conn, project, &id).await?;
                hierarchy::HierarchyRows::check_parent(
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
                hierarchy::HierarchyRows::place(
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
            WorkspaceOp::PlanLinkPut {
                anchor: DesignAnchor::Part(id),
                plan,
            }
            | WorkspaceOp::PlanLinkRemove {
                anchor: DesignAnchor::Part(id),
                plan,
            } => {
                require(conn, project, &id).await?;
                if let Some(message) = &facts[&plan] {
                    return Err(StorageError::Constraint(message.clone()));
                }
                changed.insert(id.to_string());
            }
            _ => unreachable!("only part operations are dispatched here"),
        }
        Ok(())
    }
}

async fn require(
    conn: &mut SqliteConnection,
    project: &ProjectId,
    id: &PartId,
) -> Result<(), StorageError> {
    PartsRows::load(conn, project, id)
        .await
        .map(|_| ())
        .map_err(|error| match error {
            StorageError::NotFound(_) => {
                StorageError::Constraint(format!("part {id} does not belong to project {project}"))
            }
            other => other,
        })
}

pub(crate) struct PartEditRows;
