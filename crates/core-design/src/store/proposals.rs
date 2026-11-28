//! Recording Planner vision proposals.
use super::DesignStore;
use super::proposal_origin::ProposalOrigin;
use crate::{OperationId, ProjectId, StorageError, VisionContent, VisionField};
use crate::{ProposalStatus, VisionAction, VisionProposal, VisionPropose};
use core_db::{CommandContext, JournalEntry, Storage};
use core_grants::Writer;
use core_threads::ThreadEntryKind;

impl DesignStore {
    pub async fn propose_vision_in(
        &self,
        ctx: &CommandContext,
        writer: &Writer,
        project: &ProjectId,
        turn: Option<&OperationId>,
        args: VisionPropose,
    ) -> Result<VisionProposal, crate::DesignError> {
        let (ctx, writer, project, turn, ts) = (
            ctx.clone(),
            writer.clone(),
            project.clone(),
            turn.cloned(),
            Storage::now(),
        );
        self.storage
            .write_txn_as::<_, _, crate::DesignError>(move |conn| {
                Box::pin(async move {
                    let origin =
                        ProposalOrigin::new(conn, &writer, &project, turn.as_ref()).await?;
                    let (scope, key) = origin.scope();
                    if let Some(id) = CommandContext::classify(conn, &ctx, scope, key).await? {
                        let entry = if origin.thread.is_some() {
                            Some(
                                sqlx::query_scalar(
                                    "SELECT entry_id FROM design_vision_proposal WHERE id=?",
                                )
                                .bind(&id)
                                .fetch_one(&mut *conn)
                                .await?,
                            )
                        } else {
                            None
                        };
                        return origin.replay(conn, &id, entry).await;
                    }
                    origin.require_running(conn).await?;
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
                    let reserved_id = if args.action == VisionAction::Add {
                        let counters: String = sqlx::query_scalar(
                            "SELECT vision_counters FROM design_workspace WHERE project_id=?",
                        )
                        .bind(project.as_str())
                        .fetch_one(&mut *conn)
                        .await?;
                        let mut counters = serde_json::from_str(&counters)?;
                        let id = crate::vision_items::VisionItems::next_id(field, &mut counters)
                            .map_err(StorageError::Constraint)?;
                        sqlx::query(
                            "UPDATE design_workspace SET vision_counters=? WHERE project_id=?",
                        )
                        .bind(serde_json::to_string(&counters)?)
                        .bind(project.as_str())
                        .execute(&mut *conn)
                        .await?;
                        Some(id)
                    } else {
                        None
                    };
                    let p = VisionProposal {
                        approved_by: None,
                        id: uuid::Uuid::new_v4().to_string(),
                        thread_id: origin.thread.clone(),
                        grant_id: origin.grant.clone(),
                        connection_id: origin.connection_id(conn).await?,
                        action: args.action,
                        field,
                        item_id: reserved_id.clone().or(args.item_id),
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
                    let entry = origin
                        .append_card(conn, ThreadEntryKind::VisionProposal, &body, &json, &ts)
                        .await?;
                    let additions = match &turn {
                        Some(turn) => {
                            core_harness::Harness::invocation_standards_in(conn, turn.as_str())
                                .await?
                                .1
                        }
                        None => super::map_versions::MapVersionsRows::effective(conn, &project)
                            .await?
                            .additions
                            .map(|version| version.id),
                    };
                    sqlx::query(
                        "INSERT INTO design_vision_proposal
                (id,project_id,thread_id,operation_id,entry_id,grant_id,action,field,
                 item_id,reserved_item_id,
                 text,old_text,
                 expected_item_revision,status,standards_version,standards_additions_version_id,
                 created_at,seq) VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,'Pending',?,?,
                   ?, (SELECT COALESCE(MAX(seq),0)+1 FROM design_vision_proposal
                       WHERE project_id=?))",
                    )
                    .bind(&p.id)
                    .bind(project.as_str())
                    .bind(origin.thread.as_ref().map(crate::ThreadId::as_str))
                    .bind(turn.as_ref().map(OperationId::as_str))
                    .bind(entry.as_ref().map(core_threads::ThreadEntryId::as_str))
                    .bind(origin.grant.as_ref().map(core_grants::GrantId::as_str))
                    .bind(json["action"].as_str())
                    .bind(json["field"].as_str())
                    .bind(&p.item_id)
                    .bind(&reserved_id)
                    .bind(&p.text)
                    .bind(&p.old_text)
                    .bind(p.expected_item_revision)
                    .bind(core_standards::Standards::base().version)
                    .bind(additions)
                    .bind(&ts)
                    .bind(project.as_str())
                    .execute(&mut *conn)
                    .await?;
                    let event = origin.event("VisionProposed", &p.id);
                    JournalEntry::append_event(conn, event, &ts).await?;
                    origin
                        .record(conn, &ctx, "VisionProposal", &p.id, &p, &ts)
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
