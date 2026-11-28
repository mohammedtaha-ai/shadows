//! Persistence of project decision revisions.
use super::DesignStore;
use super::proposal_origin::ProposalOrigin;
use crate::decisions::DecisionRules;
use crate::{DecisionInput, DecisionRecord, DecisionState, DesignError, OperationId, ProjectId};
use core_db::{CommandContext, JournalEntry, Storage, StorageError};
use core_grants::{ApprovedBy, Approver, Writer};
use core_threads::ThreadEntryKind;
use sqlx::SqliteConnection;

type Row = (
    String,
    i64,
    String,
    String,
    String,
    String,
    String,
    Option<String>,
    String,
);

impl DesignStore {
    pub(crate) async fn propose_decision_in(
        &self,
        ctx: &CommandContext,
        writer: &Writer,
        project: &ProjectId,
        turn: Option<&OperationId>,
        input: DecisionInput,
    ) -> Result<DecisionRecord, DesignError> {
        let (ctx, writer, project, turn, ts) = (
            ctx.clone(),
            writer.clone(),
            project.clone(),
            turn.cloned(),
            Storage::now(),
        );
        self.storage
            .write_txn_as::<_, _, DesignError>(move |conn| {
                Box::pin(async move {
                    let origin =
                        ProposalOrigin::new(conn, &writer, &project, turn.as_ref()).await?;
                    if let Some(at) =
                        CommandContext::classify(conn, &ctx, "Project", project.as_str()).await?
                    {
                        return recorded(conn, &project, &at).await;
                    }
                    origin.require_running(conn).await?;
                    let input = DecisionRules::normalise(input)?;
                    let (id, version) = match &input.id {
                        Some(id) => {
                            let latest = latest(conn, &project, id).await?;
                            if latest.is_none() || latest != input.expected_version {
                                return Err(DesignError::DecisionStale(format!(
                                    "{id} is at version {}, not {}",
                                    latest.unwrap_or(0),
                                    input.expected_version.unwrap_or(0)
                                )));
                            }
                            (id.clone(), latest.unwrap_or(0) + 1)
                        }
                        None => (next_id(conn, &project).await?, 1),
                    };
                    let record = DecisionRecord {
                        id: id.clone(),
                        version,
                        project_id: project.clone(),
                        title: input.title,
                        decision: input.decision,
                        rationale: input.rationale,
                        source: input.source,
                        state: DecisionState::Proposed,
                        approved_by: None,
                        created_at: ts.clone(),
                    };
                    sqlx::query(
                        "INSERT INTO design_decisions
                           (project_id,decision_id,version,title,decision,rationale,source,
                            state,proposed_by,created_at)
                         VALUES (?,?,?,?,?,?,?,'proposed',?,?)",
                    )
                    .bind(project.as_str())
                    .bind(&record.id)
                    .bind(version)
                    .bind(&record.title)
                    .bind(&record.decision)
                    .bind(&record.rationale)
                    .bind(&record.source)
                    .bind(&ctx.principal_id)
                    .bind(&ts)
                    .execute(&mut *conn)
                    .await?;
                    let card = serde_json::to_value(&record)?;
                    let body = format!("Decision {id} v{version}: {}", record.title);
                    origin
                        .append_card(conn, ThreadEntryKind::DecisionProposal, &body, &card, &ts)
                        .await?;
                    let event =
                        origin
                            .journal("DecisionProposed")
                            .with_payload(serde_json::json!({
                                "project_id": project, "decision_id": id, "version": version
                            }));
                    JournalEntry::append_event(conn, event, &ts).await?;
                    CommandContext::record_command(
                        conn,
                        &ctx,
                        "Project",
                        project.as_str(),
                        "Decision",
                        &format!("{id}@{version}"),
                        &ts,
                    )
                    .await?;
                    Ok(record)
                })
            })
            .await
    }

    pub(crate) async fn accept_decision_in(
        &self,
        ctx: &CommandContext,
        project: &ProjectId,
        id: &str,
        version: i64,
    ) -> Result<DecisionRecord, DesignError> {
        let (mut ctx, project, id, ts) =
            (ctx.clone(), project.clone(), id.to_string(), Storage::now());
        self.storage
            .write_txn_as::<_, _, DesignError>(move |conn| {
                Box::pin(async move {
                    let approver = Approver::Person;
                    ctx.agent = Some(approver.agent());
                    if let Some(at) =
                        CommandContext::classify(conn, &ctx, "Project", project.as_str()).await?
                    {
                        return recorded(conn, &project, &at).await;
                    }
                    let Some(record) = Self::decision_in(conn, &project, &id, version).await?
                    else {
                        return Err(DesignError::DecisionStale(format!(
                            "{id} has no version {version}"
                        )));
                    };
                    if record.state != DecisionState::Proposed
                        || latest(conn, &project, &id).await? != Some(version)
                    {
                        return Err(DesignError::DecisionStale(format!(
                            "{id} v{version} is not its latest proposed revision"
                        )));
                    }
                    let approved = ApprovedBy {
                        principal: "user:local".into(),
                        agent: approver.agent(),
                        request: None,
                    };
                    sqlx::query(
                        "UPDATE design_decisions SET state='superseded'
                          WHERE project_id=? AND decision_id=? AND state='accepted'",
                    )
                    .bind(project.as_str())
                    .bind(&id)
                    .execute(&mut *conn)
                    .await?;
                    sqlx::query(
                        "UPDATE design_decisions
                            SET state='accepted', approved_by_json=?, accepted_at=?
                          WHERE project_id=? AND decision_id=? AND version=?",
                    )
                    .bind(serde_json::to_string(&approved)?)
                    .bind(&ts)
                    .bind(project.as_str())
                    .bind(&id)
                    .bind(version)
                    .execute(&mut *conn)
                    .await?;
                    let event = JournalEntry::new("DecisionAccepted", approver.actor())
                        .with_project(project.as_str())
                        .with_payload(serde_json::json!({
                            "project_id": project, "decision_id": id, "version": version
                        }));
                    JournalEntry::append_event(conn, event, &ts).await?;
                    CommandContext::record_command(
                        conn,
                        &ctx,
                        "Project",
                        project.as_str(),
                        "Decision",
                        &format!("{id}@{version}"),
                        &ts,
                    )
                    .await?;
                    Ok(DecisionRecord {
                        state: DecisionState::Accepted,
                        approved_by: Some(approved),
                        ..record
                    })
                })
            })
            .await
    }

    pub(crate) async fn decisions(
        &self,
        project: &ProjectId,
    ) -> Result<Vec<DecisionRecord>, DesignError> {
        let mut conn = self
            .storage
            .reader()
            .acquire()
            .await
            .map_err(StorageError::from)?;
        let rows: Vec<Row> = sqlx::query_as(
            "SELECT decision_id,version,title,decision,rationale,source,state,
                    approved_by_json,created_at
               FROM design_decisions WHERE project_id=?
              ORDER BY CAST(substr(decision_id,2) AS INTEGER), decision_id, version",
        )
        .bind(project.as_str())
        .fetch_all(&mut *conn)
        .await?;
        rows.into_iter().map(|row| view(project, row)).collect()
    }

    pub(crate) async fn decision_in(
        conn: &mut SqliteConnection,
        project: &ProjectId,
        id: &str,
        version: i64,
    ) -> Result<Option<DecisionRecord>, DesignError> {
        let row: Option<Row> = sqlx::query_as(
            "SELECT decision_id,version,title,decision,rationale,source,state,
                    approved_by_json,created_at
               FROM design_decisions
              WHERE project_id=? AND decision_id=? AND version=?",
        )
        .bind(project.as_str())
        .bind(id)
        .bind(version)
        .fetch_optional(&mut *conn)
        .await?;
        row.map(|row| view(project, row)).transpose()
    }
}

/// The revision a recorded command answered, read as it is now.
async fn recorded(
    conn: &mut SqliteConnection,
    project: &ProjectId,
    at: &str,
) -> Result<DecisionRecord, DesignError> {
    let (id, version) = at
        .rsplit_once('@')
        .and_then(|(id, v)| Some((id, v.parse::<i64>().ok()?)))
        .ok_or(StorageError::NotFound("decision command outcome"))?;
    DesignStore::decision_in(conn, project, id, version)
        .await?
        .ok_or(StorageError::NotFound("decision").into())
}

async fn latest(
    conn: &mut SqliteConnection,
    project: &ProjectId,
    id: &str,
) -> Result<Option<i64>, StorageError> {
    Ok(sqlx::query_scalar(
        "SELECT MAX(version) FROM design_decisions WHERE project_id=? AND decision_id=?",
    )
    .bind(project.as_str())
    .bind(id)
    .fetch_one(&mut *conn)
    .await?)
}

/// `D{n}`, one past the highest number the project has used.
async fn next_id(conn: &mut SqliteConnection, project: &ProjectId) -> Result<String, StorageError> {
    let ids: Vec<String> =
        sqlx::query_scalar("SELECT DISTINCT decision_id FROM design_decisions WHERE project_id=?")
            .bind(project.as_str())
            .fetch_all(&mut *conn)
            .await?;
    let highest = ids
        .iter()
        .filter_map(|id| id.strip_prefix('D')?.parse::<u64>().ok())
        .max()
        .unwrap_or(0);
    Ok(format!("D{}", highest + 1))
}

fn view(project: &ProjectId, row: Row) -> Result<DecisionRecord, DesignError> {
    let (id, version, title, decision, rationale, source, state, approved, created_at) = row;
    Ok(DecisionRecord {
        id,
        version,
        project_id: project.clone(),
        title,
        decision,
        rationale,
        source,
        state: serde_json::from_value(serde_json::Value::String(state))?,
        approved_by: approved.map(|a| serde_json::from_str(&a)).transpose()?,
        created_at,
    })
}
