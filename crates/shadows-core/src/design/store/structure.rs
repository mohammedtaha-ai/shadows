//! Intent, snapshot and completion records for structure writing.
use super::DesignStore;
use crate::ProjectId;
use crate::design::{PartContent, StructureWritten, map_rules};
use core_db::{Actor, CommandContext, JournalEntry, Storage};

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

impl DesignStore {
    pub(in crate::design) async fn structure_snapshot(
        &self,
        ctx: &CommandContext,
        p: &ProjectId,
    ) -> Result<Read, crate::CoreError> {
        let mut tx = self.storage.reader().begin().await?;
        if let Some(id) = CommandContext::classify(&mut tx, ctx, "Project", p.as_str()).await? {
            let json: String = sqlx::query_scalar(
                "SELECT result_json FROM design_command_result WHERE id=? AND project_id=?",
            )
            .bind(id)
            .bind(p.as_str())
            .fetch_one(&mut *tx)
            .await?;
            return Ok(Read::Replay(serde_json::from_str(&json)?));
        }
        let metadata = core_projects::Projects::get_in(&mut tx, p).await?;
        let (name, directory) = (metadata.name, metadata.directory);
        let revision: i64 =
            sqlx::query_scalar("SELECT map_revision FROM design_workspace WHERE project_id=?")
                .bind(p.as_str())
                .fetch_optional(&mut *tx)
                .await?
                .unwrap_or(0);
        let recorded: Option<Intent> = sqlx::query_as(
            "SELECT map_number,standards_version,standards_additions_version_id,
             files_json,folders_json,completed_at FROM design_structure_write WHERE project_id=?",
        )
        .bind(p.as_str())
        .fetch_optional(&mut *tx)
        .await?;
        if recorded.as_ref().is_some_and(|i| i.completed_at.is_some()) {
            return Err(crate::design::DesignError::StageBlocked(
                "structure already written".into(),
            )
            .into());
        }
        if revision == 0 {
            return Err(crate::design::DesignError::StageBlocked("map not approved".into()).into());
        }
        let effective = super::map_versions::MapVersionsRows::effective(&mut tx, p).await?;
        let rows = super::map_versions::MapVersionsRows::rows(&mut tx, p).await?;
        let contents = super::map_versions::MapVersionsRows::contents(&rows)?;
        let waived = super::map_versions::MapVersionsRows::waived(&mut tx, p).await?;
        let missing = map_rules::MapRules::missing_kinds(
            &super::map_versions::MapVersionsRows::views(&rows, &contents),
            &waived,
            &effective,
        );
        if !missing.is_empty() {
            return Err(crate::design::DesignError::StageBlocked(format!(
                "missing part: {}",
                missing.join(", ")
            ))
            .into());
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
        let parts = super::map_versions::MapVersionsRows::contents(&rows)?;
        tx.commit().await?;
        Ok(Read::Snapshot(Snapshot {
            name,
            directory: directory.ok_or_else(|| {
                crate::design::DesignError::StageBlocked(
                    "project directory missing: not configured".into(),
                )
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
    ) -> Result<(), crate::CoreError> {
        let p = p.clone();
        self.storage
            .write_txn_as::<_, _, crate::CoreError>(move |conn| {
                Box::pin(async move {
                    let live = core_projects::Projects::is_live_in(conn, &p).await?;
                    let current: Option<i64> = if live {
                        sqlx::query_scalar(
                            "SELECT map_revision FROM design_workspace WHERE project_id=?",
                        )
                        .bind(p.as_str())
                        .fetch_optional(&mut *conn)
                        .await?
                    } else {
                        None
                    };
                    if current != Some(revision) {
                        return Err(crate::design::DesignError::StageBlocked(
                            "map changed; try again".into(),
                        )
                        .into());
                    }
                    let standards =
                        super::map_versions::MapVersionsRows::effective(conn, &p).await?;
                    if standards.base.version != intent.standards_version
                        || standards.additions.as_ref().map(|a| &a.id)
                            != intent.standards_additions_version_id.as_ref()
                    {
                        return Err(crate::design::DesignError::StageBlocked(
                            "standards changed; try again".into(),
                        )
                        .into());
                    }
                    sqlx::query("INSERT INTO design_structure_write VALUES (?,?,?,?,?,?,?,NULL)")
                        .bind(p.as_str())
                        .bind(intent.map_number)
                        .bind(intent.standards_version)
                        .bind(intent.standards_additions_version_id)
                        .bind(intent.files_json)
                        .bind(intent.folders_json)
                        .bind(Storage::now())
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
    ) -> Result<StructureWritten, crate::CoreError> {
        let (ctx, p) = (ctx.clone(), p.clone());
        self.storage
            .write_txn_as::<_, _, crate::CoreError>(move |conn| {
                Box::pin(async move {
                    sqlx::query(
                        "UPDATE design_structure_write SET completed_at=? WHERE project_id=?",
                    )
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
                    let event = JournalEntry::new("ProjectStructureWritten", Actor::user("local"))
                        .with_project(p.as_str())
                        .with_payload(serde_json::json!({"project_id":p,
                    "map_version":result.map_version,"paths":result.paths}));
                    JournalEntry::append_event(conn, event.clone(), &result.completed_at).await?;
                    CommandContext::record_command(
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
