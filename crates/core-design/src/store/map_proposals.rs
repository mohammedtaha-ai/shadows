//! Persist map proposals, a Planner's or an external agent's, without approving parts.
use super::DesignStore;
use super::proposal_origin::ProposalOrigin;
use super::{map_proposal_shape as shape, map_versions as maps};
use crate::{MapAction, MapProposal, MapPropose, ProposalStatus};
use crate::{OperationId, ProjectId, StorageError};
use core_db::{CommandContext, JournalEntry, Storage};
use core_grants::Writer;
use core_threads::ThreadEntryKind;
use std::collections::BTreeMap;

impl DesignStore {
    pub async fn propose_map_in(
        &self,
        ctx: &CommandContext,
        writer: &Writer,
        p: &ProjectId,
        turn: Option<&OperationId>,
        mut args: MapPropose,
    ) -> Result<MapProposal, crate::DesignError> {
        let (ctx, writer, p, turn, ts) = (
            ctx.clone(),
            writer.clone(),
            p.clone(),
            turn.cloned(),
            Storage::now(),
        );
        self.storage
            .write_txn_as::<_, _, crate::DesignError>(move |conn| {
                Box::pin(async move {
                    let origin = ProposalOrigin::new(conn, &writer, &p, turn.as_ref()).await?;
                    let (scope, key) = origin.scope();
                    if let Some(id) = CommandContext::classify(conn, &ctx, scope, key).await? {
                        let entry = if origin.thread.is_some() {
                            Some(
                                sqlx::query_scalar(
                                    "SELECT entry_id FROM design_map_proposal WHERE id=?",
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
                    let vision =
                        super::proposal_read::ProposalReadRows::live_vision(conn, &p).await?;
                    shape::MapProposalShapeRows::complete(&vision)?;
                    let (v, m): (i64, i64) = sqlx::query_as(
                    "SELECT vision_revision,map_revision FROM design_workspace WHERE project_id=?",
                )
                .bind(p.as_str())
                .fetch_one(&mut *conn)
                .await?;
                    if args.action == MapAction::First && m != 0 {
                        return Err(StorageError::Constraint(
                            "a map is approved; propose a change".into(),
                        )
                        .into());
                    }
                    if args.action != MapAction::First && m == 0 {
                        return Err(crate::DesignError::StageBlocked(
                            "no map yet: propose the first map".into(),
                        ));
                    }
                    let standards = match &turn {
                        Some(turn) => shape::MapProposalShapeRows::pinned(conn, &p, turn).await?,
                        None => maps::MapVersionsRows::effective(conn, &p).await?,
                    };
                    shape::MapProposalShapeRows::validate_args(&mut args, &standards)?;
                    let rows = maps::MapVersionsRows::rows(conn, &p).await?;
                    let old = shape::MapProposalShapeRows::old_content(&args, &rows)?;
                    let old_parent = shape::MapProposalShapeRows::old_parent(&args, &rows);
                    let proposed_rows = shape::MapProposalShapeRows::result(&args, &rows)?;
                    shape::MapProposalShapeRows::check(&args, &proposed_rows, &vision, &standards)?;
                    let contents = maps::MapVersionsRows::contents(&proposed_rows)?;
                    let unserved = crate::map_rules::MapRules::unserved_items(
                        &maps::MapVersionsRows::views(&proposed_rows, &contents),
                        &vision,
                    );
                    if let Some(id) = &args.replaces {
                        let previous =
                            super::map_proposal_read::MapProposalReadRows::load(conn, &p, id)
                                .await?;
                        if previous.status != ProposalStatus::Pending {
                            return Err(StorageError::Constraint(
                                "only a pending proposal can be replaced".into(),
                            )
                            .into());
                        }
                        sqlx::query(
                            "UPDATE design_map_proposal SET status='Superseded'
                    WHERE project_id=? AND id=?",
                        )
                        .bind(p.as_str())
                        .bind(id)
                        .execute(&mut *conn)
                        .await?;
                    }
                    sqlx::query(
                        "UPDATE design_map_proposal SET status='Superseded'
                WHERE project_id=? AND status='Pending' AND ((action='first' AND ?='first')
                OR (action='change' AND ?='change' AND part_id=?))",
                    )
                    .bind(p.as_str())
                    .bind(serde_json::to_value(args.action)?.as_str())
                    .bind(serde_json::to_value(args.action)?.as_str())
                    .bind(args.part_id.as_ref().map(|id| id.as_str()))
                    .execute(&mut *conn)
                    .await?;
                    let expected_part = args.part_id.as_ref().and_then(|id| {
                        rows.iter()
                            .find(|r| r.id == id.as_str())
                            .map(|r| r.revision)
                    });
                    let expected: BTreeMap<_, _> = args
                        .parts
                        .iter()
                        .flat_map(|p| &p.content.serves)
                        .filter_map(|id| vision.item(id).map(|i| (id.clone(), i.revision)))
                        .collect();
                    let proposal = MapProposal {
                        unserved,
                        approved_by: None,
                        id: uuid::Uuid::new_v4().to_string(),
                        thread_id: origin.thread.clone(),
                        grant_id: origin.grant.clone(),
                        connection_id: origin.connection_id(conn).await?,
                        action: args.action,
                        parts: args.parts,
                        part_id: args.part_id,
                        old,
                        old_parent,
                        waivers: args.waivers,
                        status: ProposalStatus::Pending,
                        outdated: false,
                        created_at: ts.clone(),
                    };
                    let card = serde_json::to_value(&proposal)?;
                    let entry = origin
                        .append_card(
                            conn,
                            ThreadEntryKind::MapProposal,
                            "Project map proposal",
                            &card,
                            &ts,
                        )
                        .await?;
                    sqlx::query(
                        "INSERT INTO design_map_proposal
                (id,project_id,thread_id,operation_id,entry_id,grant_id,action,part_id,payload_json,
                 vision_revision,map_revision,expected_part_revision,expected_items_json,status,
                 standards_version,standards_additions_version_id,created_at,seq)
                VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,'Pending',?,?,?,
                 (SELECT COALESCE(MAX(seq),0)+1 FROM design_map_proposal WHERE project_id=?))",
                    )
                    .bind(&proposal.id)
                    .bind(p.as_str())
                    .bind(origin.thread.as_ref().map(crate::ThreadId::as_str))
                    .bind(turn.as_ref().map(OperationId::as_str))
                    .bind(entry.as_ref().map(core_threads::ThreadEntryId::as_str))
                    .bind(origin.grant.as_ref().map(core_grants::GrantId::as_str))
                    .bind(card["action"].as_str())
                    .bind(proposal.part_id.as_ref().map(|id| id.as_str()))
                    .bind(serde_json::to_string(&proposal)?)
                    .bind(v)
                    .bind(m)
                    .bind(expected_part)
                    .bind(serde_json::to_string(&expected)?)
                    .bind(standards.base.version)
                    .bind(standards.additions.as_ref().map(|a| &a.id))
                    .bind(&ts)
                    .bind(p.as_str())
                    .execute(&mut *conn)
                    .await?;
                    let event = origin.event("MapProposed", &proposal.id);
                    JournalEntry::append_event(conn, event, &ts).await?;
                    origin
                        .record(conn, &ctx, "MapProposal", &proposal.id, &proposal, &ts)
                        .await?;
                    Ok(proposal)
                })
            })
            .await
    }
}
