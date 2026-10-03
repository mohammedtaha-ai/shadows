//! Transactional mutations of part content, containment and plan associations.
use super::parts::load;
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
            check_parent(conn, project, &id, parent.as_ref()).await?;
            sqlx::query("INSERT INTO design_part(id,project_id,parent_id,revision,ordinal,content_json) VALUES (?,?,?,1,0,?)")
                .bind(id.as_str()).bind(project.as_str()).bind(parent.as_ref().map(PartId::as_str)).bind(serde_json::to_string(&content)?).execute(&mut *conn).await?;
            place(
                conn,
                project,
                &id,
                parent.as_ref(),
                before.as_ref(),
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
            check_parent(conn, project, &id, parent.as_ref()).await?;
            sqlx::query("UPDATE design_part SET parent_id=? WHERE project_id=? AND id=?")
                .bind(parent.as_ref().map(PartId::as_str))
                .bind(project.as_str())
                .bind(id.as_str())
                .execute(&mut *conn)
                .await?;
            place(
                conn,
                project,
                &id,
                parent.as_ref(),
                before.as_ref(),
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
        DesignOp::VisionPut { .. } => unreachable!("vision handled by workspace edit"),
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

async fn check_parent(
    conn: &mut SqliteConnection,
    project: &ProjectId,
    id: &PartId,
    parent: Option<&PartId>,
) -> Result<(), StorageError> {
    let mut cursor = parent.cloned();
    while let Some(ancestor) = cursor {
        if ancestor == *id {
            return Err(StorageError::Constraint(format!(
                "moving part {id} here would create a containment cycle"
            )));
        }
        require(conn, project, &ancestor).await?;
        cursor = load(conn, project, &ancestor).await?.parent;
    }
    Ok(())
}

async fn place(
    conn: &mut SqliteConnection,
    project: &ProjectId,
    id: &PartId,
    parent: Option<&PartId>,
    before: Option<&PartId>,
    changed: &mut BTreeSet<String>,
) -> Result<(), StorageError> {
    if before == Some(id) {
        return Err(StorageError::Constraint(format!(
            "part {id} cannot be placed before itself"
        )));
    }
    let rows:Vec<(String,i64)>=sqlx::query_as("SELECT id,ordinal FROM design_part WHERE project_id=? AND parent_id IS ? AND id<>? ORDER BY ordinal,id")
        .bind(project.as_str()).bind(parent.map(PartId::as_str)).bind(id.as_str()).fetch_all(&mut *conn).await?;
    let position = match before {
        Some(cursor) => rows
            .iter()
            .position(|(s, _)| s == cursor.as_str())
            .ok_or_else(|| {
                StorageError::Constraint(format!(
                    "before part {cursor} is outside the destination parent"
                ))
            })?,
        None => rows.len(),
    };
    let mut ordered = rows;
    ordered.insert(position, (id.to_string(), -1));
    for (ordinal, (s, previous)) in ordered.into_iter().enumerate() {
        let ordinal = i64::try_from(ordinal)
            .map_err(|_| StorageError::Unavailable("part order exhausted".into()))?;
        if ordinal != previous {
            sqlx::query("UPDATE design_part SET ordinal=? WHERE project_id=? AND id=?")
                .bind(ordinal)
                .bind(project.as_str())
                .bind(&s)
                .execute(&mut *conn)
                .await?;
            changed.insert(s);
        }
    }
    Ok(())
}
