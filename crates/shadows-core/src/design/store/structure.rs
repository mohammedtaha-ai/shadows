//! Intent, snapshot and completion records for structure writing.
use crate::ProjectId;
use crate::command::CommandContext;
use crate::db::{Storage, StorageError, append_event, classify, now, record_command};
use crate::design::{PartContent, StructureWritten, map_rules};
use crate::events::{Actor, DurableEvent};

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(crate) struct FileHash {
    pub path: String,
    pub sha256: String,
}
#[derive(sqlx::FromRow)]
pub(crate) struct Intent {
    pub map_number: i64,
    pub standards_version: i64,
    pub standards_additions_version_id: Option<String>,
    pub files_json: String,
    pub folders_json: String,
    pub completed_at: Option<String>,
}
pub(crate) struct Snapshot {
    pub name: String,
    pub directory: String,
    pub revision: i64,
    pub parts: Vec<PartContent>,
    pub intent: Intent,
    pub resumed: bool,
}
pub(crate) enum Read {
    Replay(StructureWritten),
    Snapshot(Snapshot),
}

impl Storage {
    pub(in crate::design) async fn structure_snapshot(
        &self,
        ctx: &CommandContext,
        p: &ProjectId,
    ) -> Result<Read, StorageError> {
        let mut tx = self.reader().begin().await?;
        if let Some(id) = classify(&mut tx, ctx, "Project", p.as_str()).await? {
            let json: String = sqlx::query_scalar(
                "SELECT result_json FROM design_command_result WHERE id=? AND project_id=?",
            )
            .bind(id)
            .bind(p.as_str())
            .fetch_one(&mut *tx)
            .await?;
            return Ok(Read::Replay(serde_json::from_str(&json)?));
        }
        let row: Option<(String, Option<String>, i64)> = sqlx::query_as(
            "SELECT p.name,p.directory,COALESCE(w.map_revision,0) FROM project p
             LEFT JOIN design_workspace w ON w.project_id=p.id
             WHERE p.id=? AND p.removed_at IS NULL",
        )
        .bind(p.as_str())
        .fetch_optional(&mut *tx)
        .await?;
        let (name, directory, revision) = row.ok_or(StorageError::NotFound("project"))?;
        let recorded: Option<Intent> = sqlx::query_as(
            "SELECT map_number,standards_version,standards_additions_version_id,
             files_json,folders_json,completed_at FROM design_structure_write WHERE project_id=?",
        )
        .bind(p.as_str())
        .fetch_optional(&mut *tx)
        .await?;
        if recorded.as_ref().is_some_and(|i| i.completed_at.is_some()) {
            return Err(StorageError::StageBlocked(
                "structure already written".into(),
            ));
        }
        if revision == 0 {
            return Err(StorageError::StageBlocked("map not approved".into()));
        }
        let effective = super::map_versions::effective(&mut tx, p).await?;
        let rows = super::map_versions::rows(&mut tx, p).await?;
        let contents = super::map_versions::contents(&rows)?;
        let waived = super::map_versions::waived(&mut tx, p).await?;
        let missing = map_rules::missing_kinds(
            &super::map_versions::views(&rows, &contents),
            &waived,
            &effective,
        );
        if !missing.is_empty() {
            return Err(StorageError::StageBlocked(format!(
                "missing part: {}",
                missing.join(", ")
            )));
        }
        let resumed = recorded.is_some();
        let intent = recorded.unwrap_or(Intent {
            map_number: revision,
            standards_version: effective.base.version,
            standards_additions_version_id: effective.additions.map(|a| a.id),
            files_json: "[]".into(),
            folders_json: "[]".into(),
            completed_at: None,
        });
        let json: String = sqlx::query_scalar(
            "SELECT parts_json FROM design_map_version WHERE project_id=? AND number=?",
        )
        .bind(p.as_str())
        .bind(intent.map_number)
        .fetch_one(&mut *tx)
        .await?;
        let rows: Vec<super::map_versions::MapRow> = serde_json::from_str(&json)?;
        let parts = super::map_versions::contents(&rows)?;
        tx.commit().await?;
        Ok(Read::Snapshot(Snapshot {
            name,
            directory: directory.ok_or_else(|| {
                StorageError::StageBlocked("project directory missing: not configured".into())
            })?,
            revision,
            parts,
            intent,
            resumed,
        }))
    }

    pub(in crate::design) async fn start_structure(
        &self,
        p: &ProjectId,
        revision: i64,
        intent: Intent,
    ) -> Result<(), StorageError> {
        let p = p.clone();
        self.write_txn(move |conn| {
            Box::pin(async move {
                let current: Option<i64> = sqlx::query_scalar(
                "SELECT w.map_revision FROM design_workspace w JOIN project p ON p.id=w.project_id
                 WHERE p.id=? AND p.removed_at IS NULL")
                .bind(p.as_str()).fetch_optional(&mut *conn).await?;
                if current != Some(revision) {
                    return Err(StorageError::StageBlocked("map changed; try again".into()));
                }
                let standards = super::map_versions::effective(conn, &p).await?;
                if standards.base.version != intent.standards_version
                    || standards.additions.as_ref().map(|a| &a.id)
                        != intent.standards_additions_version_id.as_ref()
                {
                    return Err(StorageError::StageBlocked(
                        "standards changed; try again".into(),
                    ));
                }
                sqlx::query("INSERT INTO design_structure_write VALUES (?,?,?,?,?,?,?,NULL)")
                    .bind(p.as_str())
                    .bind(intent.map_number)
                    .bind(intent.standards_version)
                    .bind(intent.standards_additions_version_id)
                    .bind(intent.files_json)
                    .bind(intent.folders_json)
                    .bind(now())
                    .execute(conn)
                    .await?;
                Ok(())
            })
        })
        .await
    }

    pub(in crate::design) async fn finish_structure(
        &self,
        ctx: &CommandContext,
        p: &ProjectId,
        result: StructureWritten,
    ) -> Result<StructureWritten, StorageError> {
        let (ctx, p) = (ctx.clone(), p.clone());
        self.write_txn(move |conn| {
            Box::pin(async move {
                sqlx::query("UPDATE design_structure_write SET completed_at=? WHERE project_id=?")
                    .bind(&result.completed_at)
                    .bind(p.as_str())
                    .execute(&mut *conn)
                    .await?;
                let id = uuid::Uuid::new_v4().to_string();
                sqlx::query(
                    "INSERT INTO design_command_result (id,project_id,result_json) VALUES (?,?,?)",
                )
                .bind(&id)
                .bind(p.as_str())
                .bind(serde_json::to_string(&result)?)
                .execute(&mut *conn)
                .await?;
                let event = DurableEvent::new("ProjectStructureWritten", Actor::user("local"))
                    .with_project(&p)
                    .with_payload(serde_json::json!({"project_id":p,
                    "map_version":result.map_version,"paths":result.paths}));
                append_event(conn, &event, &result.completed_at).await?;
                record_command(
                    conn,
                    &ctx,
                    "Project",
                    p.as_str(),
                    "StructureWritten",
                    &id,
                    &result.completed_at,
                )
                .await?;
                Ok(result)
            })
        })
        .await
    }
}
