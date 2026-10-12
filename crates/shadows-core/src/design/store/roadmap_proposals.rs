//! Persist Planner roadmap proposals without approving outcomes.
use super::roadmap_shape as shape;
use crate::command::{CommandContext, Writer};
use crate::db::{Storage, append_event, classify, now, record_command};
use crate::design::{ProposalStatus, RoadmapAction, RoadmapProposal, RoadmapPropose};
use crate::threads::{NewThreadEntry, ThreadEntryKind, append_entry_in};
use crate::{OperationId, ProjectId, StorageError};

impl Storage {
    pub async fn propose_roadmap_in(
        &self,
        ctx: &CommandContext,
        writer: &Writer,
        p: &ProjectId,
        turn: &OperationId,
        mut args: RoadmapPropose,
    ) -> Result<RoadmapProposal, StorageError> {
        let (ctx, writer, p, turn, ts) =
            (ctx.clone(), writer.clone(), p.clone(), turn.clone(), now());
        self.write_txn(move |conn| {
            Box::pin(async move {
                let Writer::Planner { thread, .. } = &writer else {
                    return Err(StorageError::GrantScope);
                };
                crate::grants::check_writer(conn, &writer, &p).await?;
                if let Some(id) = classify(conn, &ctx, "Thread", thread.as_str()).await? {
                    let json: String = sqlx::query_scalar(
                        "SELECT e.card_json FROM thread_entry e JOIN design_roadmap_proposal p
                         ON p.entry_id=e.id WHERE p.id=?",
                    )
                    .bind(id)
                    .fetch_one(&mut *conn)
                    .await?;
                    return Ok(serde_json::from_str(&json)?);
                }
                crate::turns::check_running_turn_in(conn, thread, &turn).await?;
                shape::require_structure(conn, &p).await?;
                let revision: i64 = sqlx::query_scalar(
                    "SELECT roadmap_revision FROM design_workspace WHERE project_id=?",
                )
                .bind(p.as_str())
                .fetch_one(&mut *conn)
                .await?;
                if args.action == RoadmapAction::First && revision != 0 {
                    return Err(StorageError::Constraint(
                        "a roadmap is approved; propose a change".into(),
                    ));
                }
                if args.action != RoadmapAction::First && revision == 0 {
                    return Err(StorageError::StageBlocked(
                        "no roadmap yet: propose the first roadmap".into(),
                    ));
                }
                shape::validate(&mut args)?;
                let expected_parts = shape::part_snapshots(conn, &p, &args).await?;
                let existing = shape::existing(conn, &p, &args).await?;
                supersede(conn, &p, &args).await?;
                let (old, old_parent, expected_outcome) = match existing {
                    Some(outcome) => {
                        let snapshot = shape::outcome_snapshot(&outcome)?;
                        (Some(outcome.content), outcome.parent, Some(snapshot))
                    }
                    None => (None, None, None),
                };
                let proposal = RoadmapProposal {
                    approved_by: None,
                    id: uuid::Uuid::new_v4().to_string(),
                    thread_id: thread.clone(),
                    action: args.action,
                    outcomes: args.outcomes,
                    outcome_id: args.outcome_id,
                    old,
                    old_parent,
                    status: ProposalStatus::Pending,
                    outdated: false,
                    created_at: ts.clone(),
                };
                let card = serde_json::to_value(&proposal)?;
                let entry = append_entry_in(
                    conn,
                    thread,
                    NewThreadEntry {
                        kind: ThreadEntryKind::RoadmapProposal,
                        author: writer.actor(),
                        body: "Roadmap proposal",
                        refs: &[],
                        card: Some(&card),
                        operation_id: Some(&turn),
                    },
                    &ts,
                )
                .await?;
                sqlx::query(
                    "INSERT INTO design_roadmap_proposal
                     (id,project_id,thread_id,operation_id,entry_id,action,outcome_id,
                      payload_json,roadmap_revision,expected_outcome_json,
                      expected_parts_json,status,created_at,seq)
                     VALUES (?,?,?,?,?,?,?,?,?,?,?,'Pending',?,
                      (SELECT COALESCE(MAX(seq),0)+1 FROM design_roadmap_proposal
                       WHERE project_id=?))",
                )
                .bind(&proposal.id)
                .bind(p.as_str())
                .bind(thread.as_str())
                .bind(turn.as_str())
                .bind(entry.id.as_str())
                .bind(card["action"].as_str())
                .bind(proposal.outcome_id.as_ref().map(|id| id.as_str()))
                .bind(serde_json::to_string(&proposal)?)
                .bind(revision)
                .bind(expected_outcome)
                .bind(serde_json::to_string(&expected_parts)?)
                .bind(&ts)
                .bind(p.as_str())
                .execute(&mut *conn)
                .await?;
                let event = crate::events::DurableEvent::new("RoadmapProposed", writer.actor())
                    .with_project(&p)
                    .with_thread(thread)
                    .with_operation(&turn)
                    .with_payload(serde_json::json!({"project_id":p,"proposal_id":proposal.id}));
                append_event(conn, &event, &ts).await?;
                record_command(
                    conn,
                    &ctx,
                    "Thread",
                    thread.as_str(),
                    "RoadmapProposal",
                    &proposal.id,
                    &ts,
                )
                .await?;
                Ok(proposal)
            })
        })
        .await
    }
}

/// The pending proposals the new one replaces: the one it names, a pending first for a
/// first, and a pending change or remove of the same outcome.
async fn supersede(
    conn: &mut sqlx::SqliteConnection,
    p: &ProjectId,
    args: &RoadmapPropose,
) -> Result<(), StorageError> {
    if let Some(id) = &args.replaces {
        let status: Option<String> = sqlx::query_scalar(
            "SELECT status FROM design_roadmap_proposal WHERE project_id=? AND id=?",
        )
        .bind(p.as_str())
        .bind(id)
        .fetch_optional(&mut *conn)
        .await?;
        if status.ok_or(StorageError::NotFound("roadmap proposal"))? != "Pending" {
            return Err(StorageError::Constraint(
                "only a pending proposal can be replaced".into(),
            ));
        }
        sqlx::query(
            "UPDATE design_roadmap_proposal SET status='Superseded' WHERE project_id=? AND id=?",
        )
        .bind(p.as_str())
        .bind(id)
        .execute(&mut *conn)
        .await?;
    }
    let edits = matches!(args.action, RoadmapAction::Change | RoadmapAction::Remove);
    sqlx::query(
        "UPDATE design_roadmap_proposal SET status='Superseded'
         WHERE project_id=? AND status='Pending'
         AND ((action='first' AND ?) OR (action IN ('change','remove') AND ? AND outcome_id=?))",
    )
    .bind(p.as_str())
    .bind(args.action == RoadmapAction::First)
    .bind(edits)
    .bind(args.outcome_id.as_ref().map(|id| id.as_str()))
    .execute(&mut *conn)
    .await?;
    Ok(())
}
