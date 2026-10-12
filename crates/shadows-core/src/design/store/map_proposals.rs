//! Persist Planner map proposals without approving parts.
use super::{map_proposal_shape as shape, map_versions as maps};
use crate::command::{CommandContext, Writer};
use crate::db::{Storage, append_event, classify, now, record_command};
use crate::design::{MapAction, MapProposal, MapPropose, ProposalStatus};
use crate::threads::{NewThreadEntry, ThreadEntryKind, append_entry_in};
use crate::{OperationId, ProjectId, StorageError};
use std::collections::BTreeMap;

impl Storage {
    pub async fn propose_map_in(
        &self,
        ctx: &CommandContext,
        writer: &Writer,
        p: &ProjectId,
        turn: &OperationId,
        mut args: MapPropose,
    ) -> Result<MapProposal, StorageError> {
        let (ctx, writer, p, turn, ts) =
            (ctx.clone(), writer.clone(), p.clone(), turn.clone(), now());
        self.write_txn(move |conn| Box::pin(async move {
            let Writer::Planner {thread,..} = &writer else {return Err(StorageError::GrantScope);};
            crate::grants::check_writer(conn,&writer,&p).await?;
            if let Some(id) = classify(conn,&ctx,"Thread",thread.as_str()).await? {
                let json: String = sqlx::query_scalar(
                    "SELECT e.card_json FROM thread_entry e JOIN design_map_proposal p
                     ON p.entry_id=e.id WHERE p.id=?").bind(id).fetch_one(&mut *conn).await?;
                return Ok(serde_json::from_str(&json)?);
            }
            crate::turns::check_running_turn_in(conn,thread,&turn).await?;
            let vision = super::proposal_read::live_vision(conn,&p).await?;
            shape::complete(&vision)?;
            let (v,m): (i64,i64) = sqlx::query_as(
                "SELECT vision_revision,map_revision FROM design_workspace WHERE project_id=?")
                .bind(p.as_str()).fetch_one(&mut *conn).await?;
            if args.action == MapAction::First && m != 0 {
                return Err(StorageError::Constraint("a map is approved; propose a change".into()));
            }
            if args.action != MapAction::First && m == 0 {
                return Err(StorageError::StageBlocked("no map yet: propose the first map".into()));
            }
            let standards = shape::pinned(conn,&p,&turn).await?;
            shape::validate_args(&mut args,&standards)?;
            let rows = maps::rows(conn,&p).await?;
            let old = shape::old_content(&args,&rows)?;
            shape::check(&args,&shape::result(&args,&rows)?,&vision,&standards)?;
            if let Some(id) = &args.replaces {
                let previous = super::map_proposal_read::load(conn,&p,id).await?;
                if previous.status != ProposalStatus::Pending {
                    return Err(StorageError::Constraint("only a pending proposal can be replaced".into()));
                }
                sqlx::query("UPDATE design_map_proposal SET status='Superseded' WHERE project_id=? AND id=?")
                    .bind(p.as_str()).bind(id).execute(&mut *conn).await?;
            }
            sqlx::query("UPDATE design_map_proposal SET status='Superseded'
                WHERE project_id=? AND status='Pending' AND ((action='first' AND ?='first')
                OR (action='change' AND ?='change' AND part_id=?))")
                .bind(p.as_str()).bind(serde_json::to_value(args.action)?.as_str())
                .bind(serde_json::to_value(args.action)?.as_str())
                .bind(args.part_id.as_ref().map(|id| id.as_str())).execute(&mut *conn).await?;
            let expected_part = args.part_id.as_ref().and_then(|id|
                rows.iter().find(|r| r.id == id.as_str()).map(|r| r.revision));
            let expected: BTreeMap<_,_> = args.parts.iter().flat_map(|p| &p.content.serves)
                .filter_map(|id| vision.item(id).map(|i| (id.clone(),i.revision))).collect();
            let proposal = MapProposal {id:uuid::Uuid::new_v4().to_string(),thread_id:thread.clone(),
                action:args.action,parts:args.parts,part_id:args.part_id,old,waivers:args.waivers,
                status:ProposalStatus::Pending,outdated:false,created_at:ts.clone()};
            let card = serde_json::to_value(&proposal)?;
            let entry = append_entry_in(conn,thread,NewThreadEntry {kind:ThreadEntryKind::MapProposal,
                author:writer.actor(),body:"Project map proposal",refs:&[],card:Some(&card),
                operation_id:Some(&turn)},&ts).await?;
            sqlx::query("INSERT INTO design_map_proposal
                (id,project_id,thread_id,operation_id,entry_id,action,part_id,payload_json,
                 vision_revision,map_revision,expected_part_revision,expected_items_json,status,
                 standards_version,standards_additions_version_id,created_at,seq)
                VALUES (?,?,?,?,?,?,?,?,?,?,?,?,'Pending',?,?,?,
                 (SELECT COALESCE(MAX(seq),0)+1 FROM design_map_proposal WHERE project_id=?))")
                .bind(&proposal.id).bind(p.as_str()).bind(thread.as_str()).bind(turn.as_str())
                .bind(entry.id.as_str()).bind(card["action"].as_str())
                .bind(proposal.part_id.as_ref().map(|id| id.as_str()))
                .bind(serde_json::to_string(&proposal)?).bind(v).bind(m).bind(expected_part)
                .bind(serde_json::to_string(&expected)?).bind(standards.base.version)
                .bind(standards.additions.as_ref().map(|a| &a.id)).bind(&ts).bind(p.as_str())
                .execute(&mut *conn).await?;
            let event = crate::events::DurableEvent::new("MapProposed",writer.actor())
                .with_project(&p).with_thread(thread).with_operation(&turn)
                .with_payload(serde_json::json!({"project_id":p,"proposal_id":proposal.id}));
            append_event(conn,&event,&ts).await?;
            record_command(conn,&ctx,"Thread",thread.as_str(),"MapProposal",&proposal.id,&ts).await?;
            Ok(proposal)
        })).await
    }
}
