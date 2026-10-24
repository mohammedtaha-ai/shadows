//! Person approval of one vision proposal.
use super::DesignStore;
use crate::design::WorkspaceOp;
use crate::design::{ProposalStatus, VisionAction};
use crate::{DesignChange, ProjectId, StorageError, VisionContent};
use core_db::{CommandContext, JournalEntry, Storage};

impl DesignStore {
    pub async fn approve_vision_proposal_in(
        &self,
        ctx: &CommandContext,
        project: &ProjectId,
        proposal: &str,
        approver: &crate::grants::Approver,
    ) -> Result<DesignChange, crate::CoreError> {
        let (mut ctx, project, proposal, ts) = (
            ctx.clone(),
            project.clone(),
            proposal.to_string(),
            Storage::now(),
        );
        let approver = approver.clone();
        self.storage
            .write_txn_as::<_, _, crate::CoreError>(move |conn| {
                Box::pin(async move {
                    if let crate::grants::Approver::Planner {
                        writer: writer @ crate::grants::Writer::Planner { thread, .. },
                        turn,
                        ..
                    } = &approver
                    {
                        crate::grants::Grants::check_writer(conn, writer, &project).await?;
                        crate::operations::Operations::require_running_in(conn, thread, turn)
                            .await?;
                    }
                    let request =
                        crate::grants::Grants::authorise_in(conn, &approver, &project).await?;
                    ctx.agent = Some(approver.agent());
                    if let Some(id) =
                        CommandContext::classify(conn, &ctx, "Project", project.as_str()).await?
                    {
                        let json: String = sqlx::query_scalar(
                            "SELECT result_json FROM design_command_result
                    WHERE id=? AND project_id=?",
                        )
                        .bind(id)
                        .bind(project.as_str())
                        .fetch_one(&mut *conn)
                        .await?;
                        return Ok(serde_json::from_str(&json)?);
                    }
                    let current: VisionContent =
                        super::proposal_read::ProposalReadRows::live_vision(conn, &project).await?;
                    let p = super::proposal_read::ProposalReadRows::load(
                        conn, &project, &proposal, &current,
                    )
                    .await?;
                    let (revision, number, json, counter_json): (i64, i64, String, String) =
                        sqlx::query_as(
                            "SELECT revision,vision_revision,vision_content,vision_counters
                 FROM design_workspace WHERE project_id=?",
                        )
                        .bind(project.as_str())
                        .fetch_one(&mut *conn)
                        .await?;
                    let mut content = serde_json::from_str(&json)?;
                    let mut counters = serde_json::from_str(&counter_json)?;
                    match p.status {
                        ProposalStatus::Superseded => {
                            return Err(StorageError::Constraint(
                                "this proposal was superseded".into(),
                            )
                            .into());
                        }
                        ProposalStatus::Approved => {
                            return Err(StorageError::Constraint(
                                "this proposal was already approved".into(),
                            )
                            .into());
                        }
                        ProposalStatus::Pending => {}
                    }
                    if p.outdated {
                        return Err(StorageError::Constraint(format!(
                            "{} changed after this proposal; ask the Planner to propose again",
                            p.item_id.as_deref().unwrap_or_default()
                        ))
                        .into());
                    }
                    let new_id = if p.action == VisionAction::Add {
                        Some(
                            super::super::vision_items::VisionItems::next_id(
                                p.field,
                                &mut counters,
                            )
                            .map_err(StorageError::Constraint)?,
                        )
                    } else {
                        None
                    };
                    let op = match p.action {
                        VisionAction::Add => WorkspaceOp::VisionItemAdd {
                            field: p.field,
                            text: p
                                .text
                                .clone()
                                .ok_or(StorageError::NotFound("proposal text"))?,
                        },
                        VisionAction::Edit => WorkspaceOp::VisionItemPut {
                            id: p
                                .item_id
                                .clone()
                                .ok_or(StorageError::NotFound("proposal item"))?,
                            text: p
                                .text
                                .clone()
                                .ok_or(StorageError::NotFound("proposal text"))?,
                        },
                        VisionAction::Remove => WorkspaceOp::VisionItemRemove {
                            id: p
                                .item_id
                                .clone()
                                .ok_or(StorageError::NotFound("proposal item"))?,
                        },
                    };
                    super::super::vision_items::VisionItems::apply(
                        &mut content,
                        &op,
                        new_id.clone(),
                        Some(&p.thread_id),
                    )
                    .map_err(StorageError::Constraint)?;
                    let next = revision.checked_add(1).ok_or_else(|| {
                        StorageError::Unavailable("design revision exhausted".into())
                    })?;
                    let number = number.checked_add(1).ok_or_else(|| {
                        StorageError::Unavailable("vision revision exhausted".into())
                    })?;
                    sqlx::query(
                        "UPDATE design_workspace SET revision=?,vision_revision=?,vision_content=?,
                vision_counters=? WHERE project_id=?",
                    )
                    .bind(next)
                    .bind(number)
                    .bind(serde_json::to_string(&content)?)
                    .bind(serde_json::to_string(&counters)?)
                    .bind(project.as_str())
                    .execute(&mut *conn)
                    .await?;
                    super::vision_versions::VisionVersionsRows::insert_version(
                        conn,
                        &project,
                        number,
                        &content,
                        Some(&proposal),
                        &ts,
                    )
                    .await?;
                    sqlx::query(
                        "UPDATE design_vision_proposal SET status='Approved',item_id=?,
                 approved_principal='user:local',approved_agent=?,approved_request_entry=?
                WHERE id=? AND project_id=?",
                    )
                    .bind(new_id.or(p.item_id))
                    .bind(approver.agent())
                    .bind(request.as_ref().map(|r| r.as_str()))
                    .bind(&proposal)
                    .bind(project.as_str())
                    .execute(&mut *conn)
                    .await?;
                    let change = DesignChange { revision: next };
                    let id = uuid::Uuid::new_v4().to_string();
                    sqlx::query(
                    "INSERT INTO design_command_result (id,project_id,result_json) VALUES (?,?,?)",
                )
                .bind(&id)
                .bind(project.as_str())
                .bind(serde_json::to_string(&change)?)
                .execute(&mut *conn)
                .await?;
                    let event =
                        core_db::JournalEntry::new("ProjectDesignChanged", approver.actor())
                            .with_project(project.as_str())
                            .with_payload(serde_json::json!({"project_id":project,"revision":next,
                    "changed_parts":[],"changed_outcomes":[],"vision_changed":true}));
                    JournalEntry::append_event(conn, event.clone(), &ts).await?;
                    CommandContext::record_command(
                        conn,
                        &ctx,
                        "Project",
                        project.as_str(),
                        "DesignChange",
                        &id,
                        &ts,
                    )
                    .await?;
                    Ok(change)
                })
            })
            .await
    }
}
