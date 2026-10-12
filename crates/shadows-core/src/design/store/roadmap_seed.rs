//! Test-support seeds that put a project at the plans stage without a conversation.

use crate::db::{Storage, now};
use crate::design::{PartContent, PartId, VisionContent};
use crate::plans::PlanId;
use crate::{ProjectId, StorageError};

impl Storage {
    /// Seeds the map if absent, one part, and an approved roadmap; answers the part.
    /// Repeating it changes nothing and answers the same part.
    pub async fn seed_roadmap_for_tests(&self, p: &ProjectId) -> Result<PartId, StorageError> {
        let p = p.clone();
        self.write_txn(move |conn| {
            Box::pin(async move {
                sqlx::query(
                    "INSERT OR IGNORE INTO design_workspace
                     (project_id,revision,vision_revision,vision_content) VALUES (?,0,0,?)",
                )
                .bind(p.as_str())
                .bind(serde_json::to_string(&VisionContent::default())?)
                .execute(&mut *conn)
                .await?;
                let existing: Option<String> = sqlx::query_scalar(
                    "SELECT id FROM design_part WHERE project_id=? ORDER BY ordinal,id LIMIT 1",
                )
                .bind(p.as_str())
                .fetch_optional(&mut *conn)
                .await?;
                let part = match existing {
                    Some(id) => PartId::from_stored(id),
                    None => {
                        let id = PartId::generate();
                        let content = PartContent {
                            title: "Seeded part".into(),
                            responsibility: "Seeded for tests".into(),
                            design: "Seeded for tests".into(),
                            ..Default::default()
                        };
                        sqlx::query(
                            "INSERT INTO design_part(id,project_id,parent_id,revision,ordinal,
                             content_json) VALUES (?,?,NULL,1,0,?)",
                        )
                        .bind(id.as_str())
                        .bind(p.as_str())
                        .bind(serde_json::to_string(&content)?)
                        .execute(&mut *conn)
                        .await?;
                        id
                    }
                };
                let map: i64 = sqlx::query_scalar(
                    "SELECT map_revision FROM design_workspace WHERE project_id=?",
                )
                .bind(p.as_str())
                .fetch_one(&mut *conn)
                .await?;
                if map == 0 {
                    super::map_versions::insert_version(conn, &p, None, &now()).await?;
                }
                sqlx::query(
                    "UPDATE design_workspace SET roadmap_revision=MAX(roadmap_revision,1)
                     WHERE project_id=?",
                )
                .bind(p.as_str())
                .execute(&mut *conn)
                .await?;
                Ok(part)
            })
        })
        .await
    }

    /// Ties `plan` to `part`, as the person does on the Project map page.
    pub async fn link_plan_part_for_tests(
        &self,
        p: &ProjectId,
        plan: &PlanId,
        part: &PartId,
    ) -> Result<(), StorageError> {
        let (p, plan, part) = (p.clone(), plan.clone(), part.clone());
        self.write_txn(move |conn| {
            Box::pin(async move {
                super::link_plan_parts_in(conn, &p, &plan, std::slice::from_ref(&part)).await
            })
        })
        .await
    }
}
