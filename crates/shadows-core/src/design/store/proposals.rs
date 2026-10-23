//! Recording Planner vision proposals.
use super::DesignStore;
use crate::db::{CommandContext, append_event, classify, now, record_command};
use crate::design::{ProposalStatus, VisionAction, VisionProposal, VisionPropose};
use crate::grants::Writer;
use crate::threads::{NewThreadEntry, ThreadEntryKind, Threads};
use crate::{OperationId, ProjectId, StorageError, VisionContent, VisionField};

impl DesignStore {
    pub async fn propose_vision_in(
        &self,
        ctx: &CommandContext,
        writer: &Writer,
        project: &ProjectId,
        turn: &OperationId,
        args: VisionPropose,
    ) -> Result<VisionProposal, crate::CoreError> {
        let (ctx, writer, project, turn, ts) = (
            ctx.clone(),
            writer.clone(),
            project.clone(),
            turn.clone(),
            now(),
        );
        self.storage
            .write_txn_as::<_, _, crate::CoreError>(move |conn| {
                Box::pin(async move {
                    let Writer::Planner { thread, .. } = &writer else {
                        return Err(crate::grants::GrantsError::GrantScope.into());
                    };
                    crate::grants::Grants::check_writer(conn, &writer, &project).await?;
                    if let Some(id) = classify(conn, &ctx, "Thread", thread.as_str()).await? {
                        let entry: String = sqlx::query_scalar(
                            "SELECT entry_id FROM design_vision_proposal WHERE id=?",
                        )
                        .bind(id)
                        .fetch_one(&mut *conn)
                        .await?;
                        let json = Threads::card_in(
                            conn,
                            &crate::threads::ThreadEntryId::from_stored(entry),
                        )
                        .await?;
                        return Ok(serde_json::from_str(&json)?);
                    }
                    crate::operations::Operations::require_running_in(conn, thread, turn.as_str())
                        .await?;
                    let content =
                        super::proposal_read::ProposalReadRows::live_vision(conn, &project).await?;
                    let (field, old_text, expected_item_revision) = validate(&args, &content)?;
                    if let Some(replaces) = &args.replaces {
                        let old = super::proposal_read::ProposalReadRows::load(
                            conn, &project, replaces, &content,
                        )
                        .await?;
                        if old.status != ProposalStatus::Pending {
                            return Err(StorageError::Constraint(
                                "only a pending proposal can be replaced".into(),
                            )
                            .into());
                        }
                        sqlx::query(
                            "UPDATE design_vision_proposal SET status='Superseded'
                    WHERE id=? AND project_id=?",
                        )
                        .bind(replaces)
                        .bind(project.as_str())
                        .execute(&mut *conn)
                        .await?;
                    }
                    if args.action != VisionAction::Add {
                        sqlx::query(
                            "UPDATE design_vision_proposal SET status='Superseded'
                    WHERE project_id=? AND item_id=? AND status='Pending'",
                        )
                        .bind(project.as_str())
                        .bind(&args.item_id)
                        .execute(&mut *conn)
                        .await?;
                    }
                    sqlx::query(
                        "INSERT INTO design_workspace
                (project_id,revision,vision_revision,vision_content) VALUES (?,0,0,?)
                ON CONFLICT(project_id) DO NOTHING",
                    )
                    .bind(project.as_str())
                    .bind(serde_json::to_string(&content)?)
                    .execute(&mut *conn)
                    .await?;
                    let p = VisionProposal {
                        approved_by: None,
                        id: uuid::Uuid::new_v4().to_string(),
                        thread_id: thread.clone(),
                        action: args.action,
                        field,
                        item_id: args.item_id,
                        text: args.text,
                        old_text,
                        expected_item_revision,
                        status: ProposalStatus::Pending,
                        outdated: false,
                        created_at: ts.clone(),
                    };
                    let json = serde_json::to_value(&p)?;
                    let action = match p.action {
                        VisionAction::Add => "Add",
                        VisionAction::Edit => "Edit",
                        VisionAction::Remove => "Remove",
                    };
                    let label = match field {
                        VisionField::Purpose => "purpose",
                        VisionField::Users => "user",
                        VisionField::Goals => "goal",
                        VisionField::Boundaries => "boundary",
                        VisionField::TechnicalDirection => "technical direction",
                    };
                    let body = format!(
                        "{action} {label}: {}",
                        p.text
                            .as_deref()
                            .or(p.item_id.as_deref())
                            .unwrap_or_default()
                    );
                    let entry = Threads::append_entry_in(
                        conn,
                        thread,
                        NewThreadEntry {
                            kind: ThreadEntryKind::VisionProposal,
                            author: writer.actor(),
                            body: &body,
                            refs: &[],
                            card: Some(&json),
                            operation_id: Some(&turn),
                        },
                        &ts,
                    )
                    .await?;
                    let (_, additions) =
                        crate::harness::Harness::invocation_standards_in(conn, turn.as_str())
                            .await?;
                    sqlx::query(
                        "INSERT INTO design_vision_proposal
                (id,project_id,thread_id,operation_id,entry_id,action,field,item_id,text,old_text,
                 expected_item_revision,status,standards_version,standards_additions_version_id,
                 created_at,seq) VALUES (?,?,?,?,?,?,?,?,?,?,?,'Pending',?,?,
                   ?, (SELECT COALESCE(MAX(seq),0)+1 FROM design_vision_proposal
                       WHERE project_id=?))",
                    )
                    .bind(&p.id)
                    .bind(project.as_str())
                    .bind(thread.as_str())
                    .bind(turn.as_str())
                    .bind(entry.id.as_str())
                    .bind(json["action"].as_str())
                    .bind(json["field"].as_str())
                    .bind(&p.item_id)
                    .bind(&p.text)
                    .bind(&p.old_text)
                    .bind(p.expected_item_revision)
                    .bind(crate::standards::Standards::base().version)
                    .bind(additions)
                    .bind(&ts)
                    .bind(project.as_str())
                    .execute(&mut *conn)
                    .await?;
                    let event = crate::events::DurableEvent::new("VisionProposed", writer.actor())
                        .with_project(&project)
                        .with_thread(thread)
                        .with_operation(&turn)
                        .with_payload(serde_json::json!({"project_id":project,"proposal_id":p.id}));
                    append_event(conn, &event, &ts).await?;
                    record_command(
                        conn,
                        &ctx,
                        "Thread",
                        thread.as_str(),
                        "VisionProposal",
                        &p.id,
                        &ts,
                    )
                    .await?;
                    Ok(p)
                })
            })
            .await
    }
}

fn validate(
    args: &VisionPropose,
    content: &VisionContent,
) -> Result<(VisionField, Option<String>, Option<i64>), StorageError> {
    let bad = |s: &str| StorageError::Constraint(s.into());
    match args.action {
        VisionAction::Add => {
            if args.item_id.is_some() {
                return Err(bad("an addition has no item id"));
            }
            let field = args.field.ok_or_else(|| bad("an addition needs a field"))?;
            if args.text.as_ref().is_none_or(|s| s.trim().is_empty()) {
                return Err(bad("an addition needs text"));
            }
            Ok((field, None, None))
        }
        VisionAction::Edit | VisionAction::Remove => {
            if args.field.is_some() {
                return Err(bad("an item edit names its id, not a field"));
            }
            if args.action == VisionAction::Remove && args.text.is_some() {
                return Err(bad("a removal has no text"));
            }
            if args.action == VisionAction::Edit
                && args.text.as_ref().is_none_or(|s| s.trim().is_empty())
            {
                return Err(bad("an edit needs text"));
            }
            let id = args
                .item_id
                .as_deref()
                .ok_or_else(|| bad("an item edit needs its id"))?;
            let item = content
                .item(id)
                .ok_or_else(|| bad("the item is not in the vision"))?;
            Ok((
                VisionField::of_id(id).ok_or_else(|| bad("invalid item id"))?,
                Some(item.text.clone()),
                Some(item.revision),
            ))
        }
    }
}
