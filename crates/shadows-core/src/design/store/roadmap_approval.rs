//! Atomic approval of a roadmap proposal by the person or at their request.
use super::{outcome_edit, roadmap_proposal_read as read};
use crate::db::CommandContext;
use crate::db::{Storage, append_event, classify, now, record_command};
use crate::design::{
    DesignChange, DesignOp, OutcomeId, ProposalStatus, ProposedOutcome, RoadmapAction,
    RoadmapProposal,
};
use crate::{ProjectId, StorageError};
use sqlx::SqliteConnection;
use std::collections::{BTreeMap, BTreeSet};

impl Storage {
    pub async fn approve_roadmap_proposal_in(
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
                super::roadmap_shape::require_structure(conn, &p).await?;
                let proposal = read::load(conn, &p, &id).await?;
                if proposal.status != ProposalStatus::Pending {
                    return Err(StorageError::Constraint(
                        "proposal is approved or superseded; ask the Planner to propose again"
                            .into(),
                    ));
                }
                if proposal.outdated {
                    return Err(StorageError::StageBlocked(
                        "proposal outdated; propose again".into(),
                    ));
                }
                let mut changed = BTreeSet::new();
                write(conn, &p, &proposal, &mut changed).await?;
                sqlx::query(
                    "UPDATE design_workspace SET revision=revision+1,
                     roadmap_revision=roadmap_revision+1 WHERE project_id=?",
                )
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
                    "UPDATE design_roadmap_proposal SET status='Approved',
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
                let approved =
                    crate::events::DurableEvent::new("RoadmapApproved", approver.actor())
                        .with_project(&p)
                        .with_payload(serde_json::json!({"project_id":p,"proposal_id":id}));
                append_event(conn, &approved, &ts).await?;
                let event =
                    crate::events::DurableEvent::new("ProjectDesignChanged", approver.actor())
                        .with_project(&p)
                        .with_payload(serde_json::json!({"project_id":p,"revision":revision,
                    "changed_parts":[],"changed_outcomes":changed,"vision_changed":false}));
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

/// Applies the proposal to outcomes and their part references.
async fn write(
    conn: &mut SqliteConnection,
    p: &ProjectId,
    proposal: &RoadmapProposal,
    changed: &mut BTreeSet<String>,
) -> Result<(), StorageError> {
    match proposal.action {
        RoadmapAction::First | RoadmapAction::Add => create(conn, p, proposal, changed).await,
        RoadmapAction::Change => change(conn, p, proposal, changed).await,
        RoadmapAction::Remove => remove(conn, p, proposal, changed).await,
    }
}

async fn create(
    conn: &mut SqliteConnection,
    p: &ProjectId,
    proposal: &RoadmapProposal,
    changed: &mut BTreeSet<String>,
) -> Result<(), StorageError> {
    let first = proposal.action == RoadmapAction::First;
    let ids: BTreeMap<_, _> = proposal
        .outcomes
        .iter()
        .map(|o| (o.key.clone(), OutcomeId::generate()))
        .collect();
    let mut pending: Vec<&ProposedOutcome> = proposal.outcomes.iter().collect();
    let mut done = BTreeSet::new();
    while !pending.is_empty() {
        let next = pending
            .iter()
            .position(|o| !first || o.parent_key.as_ref().is_none_or(|k| done.contains(k)))
            .ok_or_else(|| StorageError::Constraint("outcome containment cycle".into()))?;
        let outcome = pending.remove(next);
        let parent = if first {
            outcome
                .parent_key
                .as_ref()
                .and_then(|k| ids.get(k))
                .cloned()
        } else {
            outcome.parent.clone()
        };
        let id = ids[&outcome.key].clone();
        let create = DesignOp::OutcomeCreate {
            id: id.clone(),
            parent,
            before: None,
            content: outcome.content.clone(),
        };
        outcome_edit::apply(conn, p, create, changed).await?;
        for part in &outcome.parts {
            let link = DesignOp::OutcomePartPut {
                outcome: id.clone(),
                part: part.clone(),
            };
            outcome_edit::apply(conn, p, link, changed).await?;
        }
        done.insert(outcome.key.clone());
    }
    Ok(())
}

async fn change(
    conn: &mut SqliteConnection,
    p: &ProjectId,
    proposal: &RoadmapProposal,
    changed: &mut BTreeSet<String>,
) -> Result<(), StorageError> {
    let (Some(id), [outcome]) = (&proposal.outcome_id, proposal.outcomes.as_slice()) else {
        return Err(StorageError::Constraint(
            "a change names one outcome".into(),
        ));
    };
    let put = DesignOp::OutcomePut {
        id: id.clone(),
        content: outcome.content.clone(),
    };
    outcome_edit::apply(conn, p, put, changed).await?;
    if outcome.parent != proposal.old_parent {
        let move_to = DesignOp::OutcomeMove {
            id: id.clone(),
            parent: outcome.parent.clone(),
            before: None,
        };
        outcome_edit::apply(conn, p, move_to, changed).await?;
    }
    let old: Vec<String> = sqlx::query_scalar(
        "SELECT part_id FROM design_outcome_part WHERE project_id=? AND outcome_id=?
         ORDER BY part_id",
    )
    .bind(p.as_str())
    .bind(id.as_str())
    .fetch_all(&mut *conn)
    .await?;
    for part in old.iter().filter(|old| {
        !outcome
            .parts
            .iter()
            .any(|part| part.as_str() == old.as_str())
    }) {
        let drop = DesignOp::OutcomePartRemove {
            outcome: id.clone(),
            part: crate::design::PartId::from_stored(part.clone()),
        };
        outcome_edit::apply(conn, p, drop, changed).await?;
    }
    for part in &outcome.parts {
        let link = DesignOp::OutcomePartPut {
            outcome: id.clone(),
            part: part.clone(),
        };
        outcome_edit::apply(conn, p, link, changed).await?;
    }
    sqlx::query("UPDATE design_outcome SET revision=revision+1 WHERE project_id=? AND id=?")
        .bind(p.as_str())
        .bind(id.as_str())
        .execute(&mut *conn)
        .await?;
    Ok(())
}

async fn remove(
    conn: &mut SqliteConnection,
    p: &ProjectId,
    proposal: &RoadmapProposal,
    changed: &mut BTreeSet<String>,
) -> Result<(), StorageError> {
    let id = proposal
        .outcome_id
        .as_ref()
        .ok_or_else(|| StorageError::Constraint("a remove names one outcome".into()))?;
    let (children, plans): (i64, i64) = sqlx::query_as(
        "SELECT (SELECT COUNT(*) FROM design_outcome WHERE project_id=? AND parent_id=?),
                (SELECT COUNT(*) FROM design_outcome_plan WHERE project_id=? AND outcome_id=?)",
    )
    .bind(p.as_str())
    .bind(id.as_str())
    .bind(p.as_str())
    .bind(id.as_str())
    .fetch_one(&mut *conn)
    .await?;
    if children + plans > 0 {
        return Err(StorageError::StageBlocked(
            "outcome has children or plans".into(),
        ));
    }
    sqlx::query("DELETE FROM design_outcome_part WHERE project_id=? AND outcome_id=?")
        .bind(p.as_str())
        .bind(id.as_str())
        .execute(&mut *conn)
        .await?;
    sqlx::query("DELETE FROM design_outcome WHERE project_id=? AND id=?")
        .bind(p.as_str())
        .bind(id.as_str())
        .execute(&mut *conn)
        .await?;
    changed.insert(id.to_string());
    Ok(())
}
