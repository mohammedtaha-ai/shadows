//! Serialized agreement starts and Draft content edits.
use super::DesignStore;
use super::{agreements::AgreementsRows, parts};
use crate::ProjectId;
use crate::design::AgreementOrigin;
use crate::design::agreement_validation::AgreementValidation;
use crate::design::{
    AgreementContent, AgreementId, AgreementState, AgreementVersion, AgreementWriter,
};
use crate::grants::Writer;
use core_db::{Actor, CommandContext, JournalEntry, Storage, StorageError};
use sqlx::SqliteConnection;

impl DesignStore {
    pub(in crate::design) async fn start_design_agreement(
        &self,
        ctx: &CommandContext,
        origin: Option<&AgreementOrigin>,
        project: &ProjectId,
        id: Option<&AgreementId>,
        content: Option<AgreementContent>,
        reason: Option<String>,
    ) -> Result<AgreementVersion, crate::CoreError> {
        let (ctx, project, id, ts) = (ctx.clone(), project.clone(), id.cloned(), Storage::now());
        let origin = origin.cloned();
        self.storage
            .write_txn_as::<_, _, crate::CoreError>(move |conn| {
                Box::pin(async move {
                    if let Some(origin) = &origin {
                        crate::grants::Grants::check_writer(conn, &origin.writer, &project).await?;
                    }
                    if let Some(result) = AgreementWriteRows::replay(conn, &ctx, &project).await? {
                        return Ok(result);
                    }
                    parts::PartsRows::revision(conn, &project).await?;
                    let (id, version, content) = match id {
                        Some(id) => {
                            let prior = AgreementsRows::load(conn, &project, &id, None).await?;
                            if prior.state == AgreementState::Draft {
                                return AgreementWriteRows::record(
                                    conn,
                                    &ctx,
                                    &project,
                                    &prior,
                                    &ts,
                                    false,
                                    origin.as_ref(),
                                )
                                .await
                                .map_err(Into::into);
                            }
                            if reason.as_deref().is_none_or(|s| s.trim().is_empty()) {
                                return Err(crate::design::DesignError::ReasonMissing.into());
                            }
                            let version = prior.version.checked_add(1).ok_or_else(|| {
                                StorageError::Unavailable("agreement versions exhausted".into())
                            })?;
                            (id, version, prior.content)
                        }
                        None => {
                            let id = AgreementId::generate();
                            let mut content = content.ok_or_else(|| {
                                StorageError::Constraint("a new agreement needs content".into())
                            })?;
                            AgreementValidation::assign_operation_ids(&mut content);
                            sqlx::query(
                                "INSERT INTO agreement(id,project_id,created_at) VALUES(?,?,?)",
                            )
                            .bind(id.as_str())
                            .bind(project.as_str())
                            .bind(&ts)
                            .execute(&mut *conn)
                            .await?;
                            (id, 1, content)
                        }
                    };
                    check_parties(conn, &project, &content).await?;
                    reserve_operations(conn, &id, None, &content).await?;
                    let writer = AgreementWriter {
                        kind: ctx.principal_kind.clone(),
                        id: ctx.principal_id.clone(),
                        operation_id: origin.as_ref().and_then(|o| o.operation.clone()),
                    };
                    sqlx::query(
                        "INSERT INTO agreement_version \
                (agreement_id,project_id,version,revision,state,reason,writer_json, \
                 created_at,content_json) \
                VALUES(?,?,?,0,'Draft',?,?,?,?)",
                    )
                    .bind(id.as_str())
                    .bind(project.as_str())
                    .bind(version)
                    .bind(if version == 1 {
                        None
                    } else {
                        reason.as_deref()
                    })
                    .bind(serde_json::to_string(&writer)?)
                    .bind(&ts)
                    .bind(serde_json::to_string(&content)?)
                    .execute(&mut *conn)
                    .await?;
                    let result = AgreementsRows::load(conn, &project, &id, Some(version)).await?;
                    let origin = origin.as_ref();
                    AgreementWriteRows::record(conn, &ctx, &project, &result, &ts, true, origin)
                        .await
                        .map_err(Into::into)
                })
            })
            .await
    }
    #[expect(
        clippy::too_many_arguments,
        reason = "the edit names its Draft version and revision"
    )]
    pub(in crate::design) async fn edit_design_agreement(
        &self,
        ctx: &CommandContext,
        origin: Option<&AgreementOrigin>,
        project: &ProjectId,
        id: &AgreementId,
        version: i64,
        expected_revision: i64,
        content: AgreementContent,
    ) -> Result<AgreementVersion, crate::CoreError> {
        let (ctx, project, id, ts) = (ctx.clone(), project.clone(), id.clone(), Storage::now());
        let origin = origin.cloned();
        self.storage
            .write_txn_as::<_, _, crate::CoreError>(move |conn| {
                Box::pin(async move {
                    if let Some(origin) = &origin {
                        crate::grants::Grants::check_writer(conn, &origin.writer, &project).await?;
                    }
                    if let Some(result) = AgreementWriteRows::replay(conn, &ctx, &project).await? {
                        return Ok(result);
                    }
                    parts::PartsRows::revision(conn, &project).await?;
                    let current = AgreementsRows::load(conn, &project, &id, None).await?;
                    if current.version != version {
                        return Err(StorageError::Constraint(format!(
                            "v{version} is no longer the Draft; read v{}",
                            current.version
                        ))
                        .into());
                    }
                    if current.state != AgreementState::Draft {
                        return Err(StorageError::Constraint(
                            "Agreed version is read only; start a Draft".into(),
                        )
                        .into());
                    }
                    if current.revision != expected_revision {
                        return Err(StorageError::RevisionConflict {
                            current: current.revision,
                            summary: "agreement Draft changed".into(),
                        }
                        .into());
                    }
                    let mut content = content.clone();
                    super::super::agreement_validation::AgreementValidation::assign_operation_ids(
                        &mut content,
                    );
                    check_parties(conn, &project, &content).await?;
                    reserve_operations(conn, &id, Some(&current.content), &content).await?;
                    sqlx::query(
                        "UPDATE agreement_version SET content_json=?,revision=revision+1 \
                WHERE agreement_id=? AND version=?",
                    )
                    .bind(serde_json::to_string(&content)?)
                    .bind(id.as_str())
                    .bind(current.version)
                    .execute(&mut *conn)
                    .await?;
                    let result =
                        AgreementsRows::load(conn, &project, &id, Some(current.version)).await?;
                    AgreementWriteRows::record(
                        conn,
                        &ctx,
                        &project,
                        &result,
                        &ts,
                        true,
                        origin.as_ref(),
                    )
                    .await
                    .map_err(Into::into)
                })
            })
            .await
    }
}
async fn check_parties(
    conn: &mut SqliteConnection,
    project: &ProjectId,
    content: &AgreementContent,
) -> Result<(), StorageError> {
    for party in &content.parties {
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM design_part WHERE project_id=? AND id=?)",
        )
        .bind(project.as_str())
        .bind(party.part_id.as_str())
        .fetch_one(&mut *conn)
        .await?;
        if !exists {
            return Err(StorageError::Constraint(format!(
                "participant part {} is not in this project",
                party.part_id
            )));
        }
    }
    Ok(())
}
async fn reserve_operations(
    conn: &mut SqliteConnection,
    id: &AgreementId,
    previous: Option<&AgreementContent>,
    content: &AgreementContent,
) -> Result<(), StorageError> {
    let before = previous
        .map(super::super::agreement_validation::AgreementValidation::operation_ids)
        .unwrap_or_default();
    for operation in super::super::agreement_validation::AgreementValidation::operation_ids(content)
    {
        if uuid::Uuid::parse_str(&operation).is_err() {
            continue;
        }
        let reserved: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM agreement_operation_identity \
             WHERE agreement_id=? AND operation_id=?)",
        )
        .bind(id.as_str())
        .bind(&operation)
        .fetch_one(&mut *conn)
        .await?;
        if reserved && previous.is_some() && !before.contains(&operation) {
            return Err(StorageError::Constraint(
                "removed operation identity cannot be reused".into(),
            ));
        }
        sqlx::query(
            "INSERT OR IGNORE INTO agreement_operation_identity(agreement_id,operation_id) \
            VALUES(?,?)",
        )
        .bind(id.as_str())
        .bind(operation)
        .execute(&mut *conn)
        .await?;
    }
    Ok(())
}
impl AgreementWriteRows {
    pub(super) async fn replay(
        conn: &mut SqliteConnection,
        ctx: &CommandContext,
        project: &ProjectId,
    ) -> Result<Option<AgreementVersion>, StorageError> {
        let Some(id) = CommandContext::classify(conn, ctx, "Project", project.as_str()).await?
        else {
            return Ok(None);
        };
        let json: String = sqlx::query_scalar(
            "SELECT result_json FROM design_command_result WHERE id=? AND project_id=?",
        )
        .bind(id)
        .bind(project.as_str())
        .fetch_one(conn)
        .await?;
        Ok(Some(serde_json::from_str(&json)?))
    }
}

impl AgreementWriteRows {
    pub(super) async fn record(
        conn: &mut SqliteConnection,
        ctx: &CommandContext,
        project: &ProjectId,
        result: &AgreementVersion,
        ts: &str,
        changed: bool,
        origin: Option<&AgreementOrigin>,
    ) -> Result<AgreementVersion, StorageError> {
        let id = uuid::Uuid::new_v4().to_string();
        sqlx::query("INSERT INTO design_command_result(id,project_id,result_json) VALUES(?,?,?)")
            .bind(&id)
            .bind(project.as_str())
            .bind(serde_json::to_string(result)?)
            .execute(&mut *conn)
            .await?;
        if changed {
            let mut event = JournalEntry::new(
                "AgreementChanged",
                Actor {
                    kind: ctx.principal_kind.clone(),
                    id: ctx.principal_id.clone(),
                },
            )
            .with_project(project.as_str())
            .with_payload(serde_json::json!({
                "agreement_id": result.agreement_id, "version": result.version,
                "revision": result.revision, "state": result.state
            }));
            if let Some(origin) = origin {
                if let Writer::Planner { thread, .. } = &origin.writer {
                    event = event.with_thread(thread.as_str());
                }
                if let Some(operation) = &origin.operation {
                    event = event.with_operation(operation.as_str());
                }
            }
            JournalEntry::append_event(conn, event.clone(), ts).await?;
        }
        CommandContext::record_command(
            conn,
            ctx,
            "Project",
            project.as_str(),
            "AgreementVersion",
            &id,
            ts,
        )
        .await?;
        Ok(result.clone())
    }
}

impl crate::design::Design {
    pub(crate) async fn agreement_replay_in(
        conn: &mut SqliteConnection,
        ctx: &CommandContext,
        project: &ProjectId,
    ) -> Result<Option<AgreementVersion>, StorageError> {
        AgreementWriteRows::replay(conn, ctx, project).await
    }
    pub(crate) async fn record_agreement_in(
        conn: &mut SqliteConnection,
        ctx: &CommandContext,
        project: &ProjectId,
        result: &AgreementVersion,
        ts: &str,
        changed: bool,
        origin: Option<&AgreementOrigin>,
    ) -> Result<AgreementVersion, StorageError> {
        AgreementWriteRows::record(conn, ctx, project, result, ts, changed, origin).await
    }
}

pub(in crate::design) struct AgreementWriteRows;
