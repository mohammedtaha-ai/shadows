//! Test-support seeds that put a project at the plans stage without a conversation.

use super::{DesignStore, map_versions::MapVersionsRows};
use crate::{ElementSeed, OutcomeContent, OutcomeId, PartContent, PartId, VisionContent};
use crate::{ProjectId, StorageError};
use core_db::Storage;
use sqlx::SqliteConnection;

impl DesignStore {
    /// Seeds the map if absent, one part, and an approved roadmap; answers the part.
    /// Repeating it changes nothing and answers the same part.
    pub async fn seed_roadmap_for_tests(&self, p: &ProjectId) -> Result<PartId, StorageError> {
        let p = p.clone();
        self.storage
            .write_txn(move |conn| {
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
                                folder: "seeded".into(),
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
                        MapVersionsRows::insert_version(conn, &p, None, &Storage::now()).await?;
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

    /// After the roadmap seed, one named fixture outcome a plan can serve;
    /// repeating it answers the same outcome.
    pub async fn seed_delivery_outcome_for_tests(
        &self,
        project: &ProjectId,
    ) -> Result<OutcomeId, StorageError> {
        self.seed_roadmap_for_tests(project).await?;
        let project = project.clone();
        self.storage
            .write_txn(move |conn| {
                Box::pin(async move {
                    let content = serde_json::to_string(&OutcomeContent {
                        title: "Delivery fixture outcome".into(),
                        intended_result: "Plans in tests serve a result".into(),
                        acceptance: vec![],
                    })?;
                    let existing: Option<String> = sqlx::query_scalar(
                        "SELECT id FROM design_outcome WHERE project_id=? AND content_json=?
                         ORDER BY ordinal,id LIMIT 1",
                    )
                    .bind(project.as_str())
                    .bind(&content)
                    .fetch_optional(&mut *conn)
                    .await?;
                    if let Some(id) = existing {
                        return Ok(OutcomeId::from_stored(id));
                    }
                    let id = OutcomeId::generate();
                    sqlx::query(
                        "INSERT INTO design_outcome
                         (id,project_id,parent_id,revision,ordinal,content_json)
                         SELECT ?,?,NULL,1,COALESCE(MAX(ordinal)+1,0),?
                           FROM design_outcome WHERE project_id=? AND parent_id IS NULL",
                    )
                    .bind(id.as_str())
                    .bind(project.as_str())
                    .bind(content)
                    .bind(project.as_str())
                    .execute(&mut *conn)
                    .await?;
                    Ok(id)
                })
            })
            .await
    }

    pub(crate) async fn seed_elements_for_tests(
        &self,
        project: &ProjectId,
    ) -> Result<ElementSeed, StorageError> {
        self.seed_roadmap_for_tests(project).await?;
        let project = project.clone();
        self.storage
            .write_txn(move |conn| {
                Box::pin(async move {
                    let (database, database_added) =
                        seed_part(conn, &project, "database", "db").await?;
                    let (frontend, frontend_added) =
                        seed_part(conn, &project, "frontend", "web").await?;
                    if database_added || frontend_added {
                        MapVersionsRows::insert_version(conn, &project, None, &Storage::now())
                            .await?;
                    }
                    let content = OutcomeContent {
                        title: "Seeded elements outcome".into(),
                        intended_result: "Exercise tables and screens".into(),
                        acceptance: vec![],
                    };
                    let content = serde_json::to_string(&content)?;
                    let existing: Option<String> = sqlx::query_scalar(
                        "SELECT id FROM design_outcome WHERE project_id=? AND content_json=?
                         ORDER BY ordinal,id LIMIT 1",
                    )
                    .bind(project.as_str())
                    .bind(&content)
                    .fetch_optional(&mut *conn)
                    .await?;
                    let outcome = match existing {
                        Some(id) => OutcomeId::from_stored(id),
                        None => {
                            let id = OutcomeId::generate();
                            sqlx::query(
                                "INSERT INTO design_outcome
                                 (id,project_id,parent_id,revision,ordinal,content_json)
                                 SELECT ?,?,NULL,1,COALESCE(MAX(ordinal)+1,0),?
                                   FROM design_outcome WHERE project_id=? AND parent_id IS NULL",
                            )
                            .bind(id.as_str())
                            .bind(project.as_str())
                            .bind(content)
                            .bind(project.as_str())
                            .execute(&mut *conn)
                            .await?;
                            id
                        }
                    };
                    for part in [&database, &frontend] {
                        sqlx::query(
                            "INSERT OR IGNORE INTO design_outcome_part
                             (project_id,outcome_id,part_id) VALUES (?,?,?)",
                        )
                        .bind(project.as_str())
                        .bind(outcome.as_str())
                        .bind(part.as_str())
                        .execute(&mut *conn)
                        .await?;
                    }
                    Ok(ElementSeed {
                        database,
                        frontend,
                        outcome,
                    })
                })
            })
            .await
    }
}

async fn seed_part(
    conn: &mut SqliteConnection,
    project: &ProjectId,
    kind: &str,
    folder: &str,
) -> Result<(PartId, bool), StorageError> {
    let content = PartContent {
        title: format!("Seeded {kind}"),
        responsibility: "Seeded for tests".into(),
        design: "Seeded for tests".into(),
        kind: Some(kind.into()),
        folder: folder.into(),
        ..Default::default()
    };
    let content = serde_json::to_string(&content)?;
    let existing: Option<String> = sqlx::query_scalar(
        "SELECT id FROM design_part WHERE project_id=? AND content_json=?
         ORDER BY ordinal,id LIMIT 1",
    )
    .bind(project.as_str())
    .bind(&content)
    .fetch_optional(&mut *conn)
    .await?;
    if let Some(id) = existing {
        return Ok((PartId::from_stored(id), false));
    }
    let id = PartId::generate();
    sqlx::query(
        "INSERT INTO design_part (id,project_id,parent_id,revision,ordinal,content_json)
         SELECT ?,?,NULL,1,COALESCE(MAX(ordinal)+1,0),?
           FROM design_part WHERE project_id=? AND parent_id IS NULL",
    )
    .bind(id.as_str())
    .bind(project.as_str())
    .bind(content)
    .bind(project.as_str())
    .execute(conn)
    .await?;
    Ok((id, true))
}
