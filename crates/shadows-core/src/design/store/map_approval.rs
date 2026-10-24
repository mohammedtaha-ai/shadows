//! Atomic person approval of a map proposal.
use super::DesignStore;
use super::{map_proposal_shape as shape, map_versions as maps};
use crate::design::{DesignChange, MapAction, MapPropose, ProposalStatus, map_rules};
use crate::{PartId, ProjectId, StorageError};
use core_db::{CommandContext, JournalEntry, Storage};
use std::collections::{BTreeMap, BTreeSet};

impl DesignStore {
    pub async fn approve_map_proposal_in(
        &self,
        ctx: &CommandContext,
        p: &ProjectId,
        id: &str,
        approver: &core_grants::Approver,
    ) -> Result<DesignChange, crate::CoreError> {
        let (mut ctx, p, id, ts) = (ctx.clone(), p.clone(), id.to_owned(), Storage::now());
        let approver = approver.clone();
        self.storage
            .write_txn_as::<_, _, crate::CoreError>(move |conn| {
                Box::pin(async move {
                    if let core_grants::Approver::Planner {
                        writer: writer @ core_grants::Writer::Planner { thread, .. },
                        turn,
                        ..
                    } = &approver
                    {
                        core_grants::Grants::check_writer(conn, writer, &p).await?;
                        core_operations::Operations::require_running_in(conn, thread, turn).await?;
                    }
                    let request = core_grants::Grants::authorise_in(conn, &approver, &p).await?;
                    ctx.agent = Some(approver.agent());
                    if let Some(result) =
                        CommandContext::classify(conn, &ctx, "Project", p.as_str()).await?
                    {
                        let json: String = sqlx::query_scalar(
                        "SELECT result_json FROM design_command_result WHERE id=? AND project_id=?",
                    )
                    .bind(result)
                    .bind(p.as_str())
                    .fetch_one(&mut *conn)
                    .await?;
                        return Ok(serde_json::from_str(&json)?);
                    }
                    let vision =
                        super::proposal_read::ProposalReadRows::live_vision(conn, &p).await?;
                    let proposal =
                        super::map_proposal_read::MapProposalReadRows::load(conn, &p, &id).await?;
                    if matches!(approver, core_grants::Approver::Planner { .. })
                        && proposal.action == MapAction::First
                        && !proposal.waivers.is_empty()
                    {
                        return Err(StorageError::Constraint(
                            "a first map with waivers is approved by the person's button".into(),
                        )
                        .into());
                    }
                    if proposal.status != ProposalStatus::Pending || proposal.outdated {
                        return Err(StorageError::Constraint(
                            "proposal is approved, superseded or outdated; \
                     ask the Planner to propose again"
                                .into(),
                        )
                        .into());
                    }
                    shape::MapProposalShapeRows::complete(&vision)?;
                    let mut args = MapPropose {
                        action: proposal.action,
                        parts: proposal.parts,
                        part_id: proposal.part_id,
                        waivers: proposal.waivers,
                        replaces: None,
                    };
                    let standards = maps::MapVersionsRows::effective(conn, &p).await?;
                    shape::MapProposalShapeRows::validate_args(&mut args, &standards)?;
                    let old_rows = maps::MapVersionsRows::rows(conn, &p).await?;
                    let new_rows = shape::MapProposalShapeRows::result(&args, &old_rows)?;
                    shape::MapProposalShapeRows::check(&args, &new_rows, &vision, &standards)?;
                    let mut waived = maps::MapVersionsRows::waived(conn, &p).await?;
                    if args.action == MapAction::First {
                        waived = args.waivers.iter().map(|w| w.part.clone()).collect();
                    }
                    let c = maps::MapVersionsRows::contents(&new_rows)?;
                    let parts = maps::MapVersionsRows::views(&new_rows, &c);
                    let missing = map_rules::MapRules::missing_kinds(&parts, &waived, &standards);
                    let old_c = maps::MapVersionsRows::contents(&old_rows)?;
                    let old_missing = map_rules::MapRules::missing_kinds(
                        &maps::MapVersionsRows::views(&old_rows, &old_c),
                        &waived,
                        &standards,
                    );
                    let missing: Vec<_> = missing
                        .into_iter()
                        .filter(|k| args.action == MapAction::First || !old_missing.contains(k))
                        .collect();
                    if !missing.is_empty() {
                        return Err(crate::design::DesignError::StageBlocked(format!(
                            "missing part: {}",
                            missing.join(", ")
                        ))
                        .into());
                    }
                    if args.action == MapAction::First {
                        let unserved = map_rules::MapRules::unserved_goals(&parts, &vision);
                        if !unserved.is_empty() {
                            return Err(crate::design::DesignError::StageBlocked(format!(
                                "unserved goals: {}",
                                unserved.join(", ")
                            ))
                            .into());
                        }
                        if !old_rows.is_empty() {
                            return Err(StorageError::Constraint(
                                "existing legacy parts need explicit adoption before a first map"
                                    .into(),
                            )
                            .into());
                        }
                    }
                    let mut changed = BTreeSet::new();
                    let keys: BTreeMap<_, _> = args
                        .parts
                        .iter()
                        .map(|part| (part.key.clone(), PartId::generate()))
                        .collect();
                    let mut pending: Vec<_> = args.parts.iter().collect();
                    let mut done = BTreeSet::new();
                    while !pending.is_empty() {
                        let index = pending
                            .iter()
                            .position(|part| {
                                args.action != MapAction::First
                                    || part.parent_key.as_ref().is_none_or(|k| done.contains(k))
                            })
                            .ok_or_else(|| {
                                StorageError::Constraint("part containment cycle".into())
                            })?;
                        let part = pending.remove(index);
                        let op = if args.action == MapAction::Change {
                            crate::design::WorkspaceOp::PartPut {
                                id: args.part_id.clone().expect("validated"),
                                content: part.content.clone(),
                            }
                        } else {
                            let parent = if args.action == MapAction::First {
                                part.parent_key.as_ref().and_then(|k| keys.get(k)).cloned()
                            } else {
                                part.parent.clone()
                            };
                            crate::design::WorkspaceOp::PartCreate {
                                id: keys[&part.key].clone(),
                                parent,
                                before: None,
                                content: part.content.clone(),
                            }
                        };
                        super::part_edit::PartEditRows::apply(
                            conn,
                            &p,
                            op,
                            &mut changed,
                            &Default::default(),
                        )
                        .await?;
                        done.insert(part.key.clone());
                    }
                    if let Some(part) = &args.part_id {
                        sqlx::query(
                        "UPDATE design_part SET revision=revision+1 WHERE project_id=? AND id=?",
                    )
                    .bind(p.as_str())
                    .bind(part.as_str())
                    .execute(&mut *conn)
                    .await?;
                    }
                    for waiver in &args.waivers {
                        sqlx::query("INSERT INTO design_part_waiver VALUES (?,?,?,?,?,?)")
                            .bind(p.as_str())
                            .bind(&waiver.part)
                            .bind(&waiver.reason)
                            .bind(&ctx.principal_id)
                            .bind(&id)
                            .bind(&ts)
                            .execute(&mut *conn)
                            .await?;
                    }
                    let reviewed: BTreeSet<_> = if args.action == MapAction::Change {
                        args.part_id.iter().map(ToString::to_string).collect()
                    } else {
                        keys.values().map(ToString::to_string).collect()
                    };
                    maps::MapVersionsRows::review(conn, &p, &reviewed, &vision).await?;
                    maps::MapVersionsRows::insert_version(conn, &p, Some(&id), &ts).await?;
                    sqlx::query(
                        "UPDATE design_workspace SET revision=revision+1 WHERE project_id=?",
                    )
                    .bind(p.as_str())
                    .execute(&mut *conn)
                    .await?;
                    let revision: i64 = sqlx::query_scalar(
                        "SELECT revision FROM design_workspace WHERE project_id=?",
                    )
                    .bind(p.as_str())
                    .fetch_one(&mut *conn)
                    .await?;
                    let change = DesignChange { revision };
                    sqlx::query(
                        "UPDATE design_map_proposal SET status='Approved',
                     approved_principal='user:local',approved_agent=?,approved_request_entry=?
                     WHERE project_id=? AND id=?",
                    )
                    .bind(approver.agent())
                    .bind(request.as_ref().map(|r| r.as_str()))
                    .bind(p.as_str())
                    .bind(&id)
                    .execute(&mut *conn)
                    .await?;
                    let result = uuid::Uuid::new_v4().to_string();
                    sqlx::query("INSERT INTO design_command_result VALUES (?,?,?)")
                        .bind(&result)
                        .bind(p.as_str())
                        .bind(serde_json::to_string(&change)?)
                        .execute(&mut *conn)
                        .await?;
                    let event =
                        core_db::JournalEntry::new("ProjectDesignChanged", approver.actor())
                            .with_project(p.as_str())
                            .with_payload(serde_json::json!({"project_id":p,"revision":revision,
                    "changed_parts":changed,"changed_outcomes":[],"vision_changed":false}));
                    JournalEntry::append_event(conn, event.clone(), &ts).await?;
                    CommandContext::record_command(
                        conn,
                        &ctx,
                        "Project",
                        p.as_str(),
                        "DesignChange",
                        &result,
                        &ts,
                    )
                    .await?;
                    Ok(change)
                })
            })
            .await
    }
}
