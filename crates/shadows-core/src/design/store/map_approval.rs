//! Atomic person approval of a map proposal.
use super::{map_proposal_shape as shape, map_versions as maps};
use crate::db::CommandContext;
use crate::db::{Storage, append_event, classify, now, record_command};
use crate::design::{DesignChange, MapAction, MapPropose, ProposalStatus, map_rules};
use crate::{PartId, ProjectId, StorageError};
use std::collections::{BTreeMap, BTreeSet};

impl Storage {
    pub async fn approve_map_proposal_in(
        &self,
        ctx: &CommandContext,
        p: &ProjectId,
        id: &str,
        approver: &crate::command::Approver,
    ) -> Result<DesignChange, StorageError> {
        let (mut ctx, p, id, ts) = (ctx.clone(), p.clone(), id.to_owned(), now());
        let approver = approver.clone();
        self.write_txn(move |conn| {
            Box::pin(async move {
                let request = crate::db::approval::authorise_in(conn, &approver, &p).await?;
                ctx.agent = Some(approver.agent());
                if let Some(result) = classify(conn, &ctx, "Project", p.as_str()).await? {
                    let json: String = sqlx::query_scalar(
                        "SELECT result_json FROM design_command_result WHERE id=? AND project_id=?",
                    )
                    .bind(result)
                    .bind(p.as_str())
                    .fetch_one(&mut *conn)
                    .await?;
                    return Ok(serde_json::from_str(&json)?);
                }
                let vision = super::proposal_read::live_vision(conn, &p).await?;
                let proposal = super::map_proposal_read::load(conn, &p, &id).await?;
                if matches!(approver, crate::command::Approver::Planner { .. })
                    && proposal.action == MapAction::First
                    && !proposal.waivers.is_empty()
                {
                    return Err(StorageError::Constraint(
                        "a first map with waivers is approved by the person's button".into(),
                    ));
                }
                if proposal.status != ProposalStatus::Pending || proposal.outdated {
                    return Err(StorageError::Constraint(
                        "proposal is approved, superseded or outdated; \
                     ask the Planner to propose again"
                            .into(),
                    ));
                }
                shape::complete(&vision)?;
                let mut args = MapPropose {
                    action: proposal.action,
                    parts: proposal.parts,
                    part_id: proposal.part_id,
                    waivers: proposal.waivers,
                    replaces: None,
                };
                let standards = maps::effective(conn, &p).await?;
                shape::validate_args(&mut args, &standards)?;
                let old_rows = maps::rows(conn, &p).await?;
                let new_rows = shape::result(&args, &old_rows)?;
                shape::check(&args, &new_rows, &vision, &standards)?;
                let mut waived = maps::waived(conn, &p).await?;
                if args.action == MapAction::First {
                    waived = args.waivers.iter().map(|w| w.part.clone()).collect();
                }
                let c = maps::contents(&new_rows)?;
                let parts = maps::views(&new_rows, &c);
                let missing = map_rules::missing_kinds(&parts, &waived, &standards);
                let old_c = maps::contents(&old_rows)?;
                let old_missing =
                    map_rules::missing_kinds(&maps::views(&old_rows, &old_c), &waived, &standards);
                let missing: Vec<_> = missing
                    .into_iter()
                    .filter(|k| args.action == MapAction::First || !old_missing.contains(k))
                    .collect();
                if !missing.is_empty() {
                    return Err(StorageError::StageBlocked(format!(
                        "missing part: {}",
                        missing.join(", ")
                    )));
                }
                if args.action == MapAction::First {
                    let unserved = map_rules::unserved_goals(&parts, &vision);
                    if !unserved.is_empty() {
                        return Err(StorageError::StageBlocked(format!(
                            "unserved goals: {}",
                            unserved.join(", ")
                        )));
                    }
                    if !old_rows.is_empty() {
                        return Err(StorageError::Constraint(
                            "existing legacy parts need explicit adoption before a first map"
                                .into(),
                        ));
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
                        .ok_or_else(|| StorageError::Constraint("part containment cycle".into()))?;
                    let part = pending.remove(index);
                    let op = if args.action == MapAction::Change {
                        crate::DesignOp::PartPut {
                            id: args.part_id.clone().expect("validated"),
                            content: part.content.clone(),
                        }
                    } else {
                        let parent = if args.action == MapAction::First {
                            part.parent_key.as_ref().and_then(|k| keys.get(k)).cloned()
                        } else {
                            part.parent.clone()
                        };
                        crate::DesignOp::PartCreate {
                            id: keys[&part.key].clone(),
                            parent,
                            before: None,
                            content: part.content.clone(),
                        }
                    };
                    super::part_edit::apply(conn, &p, op, &mut changed).await?;
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
                maps::review(conn, &p, &reviewed, &vision).await?;
                maps::insert_version(conn, &p, Some(&id), &ts).await?;
                sqlx::query("UPDATE design_workspace SET revision=revision+1 WHERE project_id=?")
                    .bind(p.as_str())
                    .execute(&mut *conn)
                    .await?;
                let revision: i64 =
                    sqlx::query_scalar("SELECT revision FROM design_workspace WHERE project_id=?")
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
                    crate::events::DurableEvent::new("ProjectDesignChanged", approver.actor())
                        .with_project(&p)
                        .with_payload(serde_json::json!({"project_id":p,"revision":revision,
                    "changed_parts":changed,"changed_outcomes":[],"vision_changed":false}));
                append_event(conn, &event, &ts).await?;
                record_command(
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
