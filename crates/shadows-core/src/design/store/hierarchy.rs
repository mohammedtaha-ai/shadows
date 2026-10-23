//! Transactional containment checks with explicit sibling ordering.
use crate::{ProjectId, db::StorageError};
use sqlx::SqliteConnection;
use std::collections::BTreeSet;

#[derive(Clone, Copy)]
pub(super) enum Tree {
    Parts,
    Outcomes,
}
impl Tree {
    fn name(self) -> &'static str {
        match self {
            Self::Parts => "part",
            Self::Outcomes => "outcome",
        }
    }
}
impl HierarchyRows {
    pub(super) async fn check_parent(
        conn: &mut SqliteConnection,
        tree: Tree,
        project: &ProjectId,
        id: &str,
        parent: Option<&str>,
    ) -> Result<(), StorageError> {
        let mut cursor = parent.map(str::to_owned);
        while let Some(ancestor) = cursor {
            if ancestor == id {
                return Err(StorageError::Constraint(format!(
                    "moving {} {id} here would create a containment cycle",
                    tree.name()
                )));
            }
            cursor = (match tree {
                Tree::Parts => sqlx::query_scalar::<_, Option<String>>(
                    "SELECT parent_id FROM design_part WHERE project_id=? AND id=?",
                ),
                Tree::Outcomes => sqlx::query_scalar::<_, Option<String>>(
                    "SELECT parent_id FROM design_outcome WHERE project_id=? AND id=?",
                ),
            })
            .bind(project.as_str())
            .bind(&ancestor)
            .fetch_optional(&mut *conn)
            .await?
            .ok_or_else(|| {
                StorageError::Constraint(format!(
                    "{} {ancestor} does not belong to project {project}",
                    tree.name()
                ))
            })?;
        }
        Ok(())
    }
}

impl HierarchyRows {
    pub(super) async fn place(
        conn: &mut SqliteConnection,
        tree: Tree,
        project: &ProjectId,
        id: &str,
        parent: Option<&str>,
        before: Option<&str>,
        changed: &mut BTreeSet<String>,
    ) -> Result<(), StorageError> {
        if before == Some(id) {
            return Err(StorageError::Constraint(format!(
                "{} {id} cannot be placed before itself",
                tree.name()
            )));
        }
        let mut ordered: Vec<(String, i64)> = (match tree {
            Tree::Parts => sqlx::query_as(
                "SELECT id,ordinal FROM design_part WHERE project_id=? \
             AND parent_id IS ? AND id<>? ORDER BY ordinal,id",
            ),
            Tree::Outcomes => sqlx::query_as(
                "SELECT id,ordinal FROM design_outcome WHERE project_id=? \
             AND parent_id IS ? AND id<>? ORDER BY ordinal,id",
            ),
        })
        .bind(project.as_str())
        .bind(parent)
        .bind(id)
        .fetch_all(&mut *conn)
        .await?;
        let position = match before {
            Some(cursor) => ordered
                .iter()
                .position(|(s, _)| s == cursor)
                .ok_or_else(|| {
                    StorageError::Constraint(format!(
                        "before {} {cursor} is outside the destination parent",
                        tree.name()
                    ))
                })?,
            None => ordered.len(),
        };
        ordered.insert(position, (id.to_owned(), -1));
        for (ordinal, (s, previous)) in ordered.into_iter().enumerate() {
            let ordinal = i64::try_from(ordinal)
                .map_err(|_| StorageError::Unavailable("hierarchy order exhausted".into()))?;
            if ordinal != previous {
                (match tree {
                    Tree::Parts => {
                        sqlx::query("UPDATE design_part SET ordinal=? WHERE project_id=? AND id=?")
                    }
                    Tree::Outcomes => sqlx::query(
                        "UPDATE design_outcome SET ordinal=? WHERE project_id=? AND id=?",
                    ),
                })
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
}

pub(in crate::design) struct HierarchyRows;
