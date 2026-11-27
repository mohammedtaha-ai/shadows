//! One job: changing a plan version: an edit under a
//! revision check, or the approval that freezes it. Every command here
//! answers only what was fixed when it committed: a replay reads that answer
//! back from its event, never the plan as it is later. Starting a version is
//! `draft.rs`; reading one is `read.rs`.

use super::PlansStore;
use sqlx::SqliteConnection;

use super::plan::PlanRows;
use super::plan_event;
use super::read::ReadRows;
use super::task::TaskRows;
use crate::model::{Approved, EditOutcome, PlanId, WorkflowId, WorkflowState, WrittenBy};
use crate::ops::Ops;
use crate::ops::PlanOp;
use core_db::{CommandContext, JournalEntry, Storage, StorageError};
use core_grants::Grants;
use core_grants::Writer;
use core_operations::OperationId;
use core_projects::ProjectId;
use core_threads::{EntryRef, NewThreadEntry, ThreadEntryKind, Threads};

impl PlansStore {
    /// The order of work: the grant, then a recorded command (answered
    /// from its event, before any revision check), then the revision, then
    /// the change with its event and command record in one transaction.
    /// `operation` is a Planner's running turn, which its event names.
    /// `present` is which worked files the project has, `None` when that
    /// cannot be told and the check is skipped.
    #[allow(clippy::too_many_arguments)]
    pub async fn edit_plan(
        &self,
        ctx: &CommandContext,
        writer: &Writer,
        operation: Option<&OperationId>,
        workflow: &WorkflowId,
        expected_revision: i64,
        ops: &[PlanOp],
        present: Option<&std::collections::HashSet<String>>,
    ) -> Result<EditOutcome, crate::PlansError> {
        let present = present.cloned();
        let (ctx, writer, workflow, ops, ts) = (
            ctx.clone(),
            writer.clone(),
            workflow.clone(),
            ops.to_vec(),
            Storage::now(),
        );
        let operation = operation.cloned();
        self.storage
            .write_txn_as::<_, _, crate::PlansError>(move |conn| {
                Box::pin(async move {
                    let (_, project) = owner(conn, &workflow).await?;
                    Grants::check_writer(conn, &writer, &project).await?;
                    if let Some(event) =
                        CommandContext::classify(conn, &ctx, "Workflow", workflow.as_str()).await?
                    {
                        return ReadRows::recorded_outcome(conn, &event)
                            .await
                            .map_err(Into::into);
                    }
                    core_design::Design::require_roadmap_in(conn, &project).await?;
                    let plan = ReadRows::load_plan(conn, &workflow, None).await?;
                    PlanRows::require_active(conn, &plan.plan_id).await?;
                    writable(
                        conn,
                        &plan.state,
                        plan.revision,
                        expected_revision,
                        &project,
                        &workflow,
                    )
                    .await?;
                    let applied = Ops::apply(&plan.content(), &ops)
                        .map_err(crate::PlansError::PlanInvalid)?;
                    pins(conn, &project, &applied.content, &applied.changed_tasks).await?;
                    if let Some(present) = &present {
                        let missing = crate::task_contract::ContractRules::file_problems(
                            &plan.content(),
                            &applied.content,
                            &applied.changed_tasks,
                            present,
                        );
                        if !missing.is_empty() {
                            return Err(crate::PlansError::PlanInvalid(missing));
                        }
                    }
                    let project_only = matches!(writer, Writer::External { .. });
                    super::graph::GraphRows::check_parents(conn, &plan, &ops, project_only).await?;
                    let graph = super::graph::GraphRows::check(
                        conn,
                        &plan,
                        None,
                        Some(&applied.content),
                        false,
                    )
                    .await?;
                    if graph.has_phases && !graph.problems.is_empty() {
                        if project_only {
                            let scoped = super::graph::GraphRows::check(
                                conn,
                                &plan,
                                None,
                                Some(&applied.content),
                                true,
                            )
                            .await?;
                            let problems = if scoped.problems.is_empty() {
                                vec![crate::Problem::new(
                                    "A linked dependency prevents this edit.",
                                )]
                            } else {
                                scoped.problems
                            };
                            return Err(crate::PlansError::PlanInvalid(problems));
                        }
                        return Err(crate::PlansError::PlanInvalid(graph.problems));
                    }
                    super::bindings::BindingsRows::validate(
                        conn,
                        &project,
                        &applied.content.bindings,
                    )
                    .await?;
                    TaskRows::write_content(conn, &workflow, &plan.tasks, &applied.content, &ts)
                        .await?;
                    sqlx::query(
                        "UPDATE workflow SET title = ?, goal = ?, revision = revision + 1,
                            updated_at = ?
                      WHERE id = ?",
                    )
                    .bind(&applied.content.title)
                    .bind(&applied.content.goal)
                    .bind(&ts)
                    .bind(workflow.as_str())
                    .execute(&mut *conn)
                    .await?;
                    let outcome = EditOutcome {
                        workflow_id: workflow.clone(),
                        version: plan.version,
                        revision: plan.revision + 1,
                        summary: applied.summary,
                        changed_tasks: applied.changed_tasks,
                    };
                    let event = plan_event("WorkflowEdited", &writer, &project, operation.as_ref())
                        .with_payload(serde_json::to_value(&outcome)?);
                    JournalEntry::append_event(conn, event.clone(), &ts).await?;
                    super::notifications::NotificationsRows::notify_plan(
                        conn,
                        &project,
                        &plan.plan_id,
                        &plan.links,
                        &writer.actor(),
                        &ts,
                    )
                    .await?;
                    CommandContext::record_command(
                        conn,
                        &ctx,
                        "Workflow",
                        workflow.as_str(),
                        "WorkflowEdited",
                        &event.event_id,
                        &ts,
                    )
                    .await?;
                    Ok(outcome)
                })
            })
            .await
    }

    /// A person's approval freezes a Draft that has no blockers, and
    /// says so in the conversation that wrote it: none when
    /// that conversation is deleted or an external agent wrote the version.
    /// A replay answers the recorded `WorkflowFrozen` payload, never the
    /// plan, whose `next` changes once the next version exists.
    pub async fn approve_plan(
        &self,
        ctx: &CommandContext,
        workflow: &WorkflowId,
        expected_revision: i64,
    ) -> Result<Approved, crate::PlansError> {
        self.approve_plan_with(
            ctx,
            workflow,
            expected_revision,
            &core_grants::Approver::Person,
        )
        .await
    }

    pub async fn approve_plan_with(
        &self,
        ctx: &CommandContext,
        workflow: &WorkflowId,
        expected_revision: i64,
        approver: &core_grants::Approver,
    ) -> Result<Approved, crate::PlansError> {
        let (mut ctx, workflow, ts) = (ctx.clone(), workflow.clone(), Storage::now());
        let approver = approver.clone();
        self.storage
            .write_txn_as::<_, _, crate::PlansError>(move |conn| {
                Box::pin(async move {
                    let (_, project) = owner(conn, &workflow).await?;
                    if let core_grants::Approver::Planner(core_grants::PlannerApprover {
                        writer: writer @ core_grants::Writer::Planner { thread, .. },
                        turn,
                        ..
                    }) = &approver
                    {
                        core_grants::Grants::check_writer(conn, writer, &project).await?;
                        core_operations::Operations::require_running_in(conn, thread, turn).await?;
                    }
                    let request =
                        core_grants::Grants::authorise_in(conn, &approver, &project).await?;
                    ctx.agent = Some(approver.agent());
                    if let Some(event) =
                        CommandContext::classify(conn, &ctx, "Workflow", workflow.as_str()).await?
                    {
                        return ReadRows::recorded_outcome(conn, &event)
                            .await
                            .map_err(Into::into);
                    }
                    core_design::Design::require_roadmap_in(conn, &project).await?;
                    let plan = ReadRows::load_plan(conn, &workflow, None).await?;
                    PlanRows::require_active(conn, &plan.plan_id).await?;
                    writable(
                        conn,
                        &plan.state,
                        plan.revision,
                        expected_revision,
                        &project,
                        &workflow,
                    )
                    .await?;
                    let explicit =
                        crate::Plans::plan_parts_in(conn, &project, &plan.plan_id).await?;
                    let layout = core_design::Design::layout_in(conn, &project).await?;
                    let ownership: Vec<_> = plan
                        .tasks
                        .iter()
                        .map(|t| crate::task_context::TaskOwnership::resolve(&layout, &t.content))
                        .collect();
                    let parts = crate::Plans::effective_parts(&explicit, &ownership);
                    let outcomes =
                        crate::Plans::plan_outcome_ids_in(conn, &project, &plan.plan_id).await?;
                    if let Some(missing) =
                        crate::Plans::missing_delivery_requirements(&parts, &outcomes).first()
                    {
                        let reason = match missing {
                            crate::model::DeliveryRequirement::Part => {
                                "plan has no part: link it to a part on the Project map page, \
                                 or start it with parts"
                            }
                            crate::model::DeliveryRequirement::Outcome => {
                                "plan has no roadmap outcome: link it on the Roadmap page"
                            }
                        };
                        return Err(core_design::DesignError::StageBlocked(reason.into()).into());
                    }
                    let blockers = plan.blockers;
                    super::bindings::BindingsRows::validate(conn, &project, &plan.bindings).await?;
                    if !blockers.is_empty() {
                        return Err(crate::PlansError::PlanInvalid(blockers));
                    }
                    sqlx::query(
                        "UPDATE workflow SET state = 'Frozen', frozen_at = ?, updated_at = ?,
                     approved_principal='user:local',approved_agent=?,approved_request_entry=?
                      WHERE id = ?",
                    )
                    .bind(&ts)
                    .bind(&ts)
                    .bind(approver.agent())
                    .bind(request.as_ref().map(|r| r.as_str()))
                    .bind(workflow.as_str())
                    .execute(&mut *conn)
                    .await?;
                    let actor = approver.actor();
                    let thread = match &plan.written_by {
                        WrittenBy::Planner {
                            thread_id,
                            thread_removed: false,
                            ..
                        } => Some(thread_id),
                        _ => None,
                    };
                    if let Some(thread) = thread {
                        let body = match &approver {
                            core_grants::Approver::Person => {
                                format!("Plan v{} approved", plan.version)
                            }
                            _ => format!(
                                "Plan v{} approved by the Planner at your request",
                                plan.version
                            ),
                        };
                        let refs = [EntryRef::Workflow(workflow.clone().as_str().into())];
                        let entry = NewThreadEntry {
                            kind: ThreadEntryKind::PlanApproved,
                            author: actor.clone(),
                            body: &body,
                            refs: &refs,
                            card: None,
                            operation_id: None,
                        };
                        Threads::append_entry_in(conn, thread, entry, &ts).await?;
                    }
                    let approved = Approved {
                        workflow_id: workflow.clone(),
                        version: plan.version,
                        revision: plan.revision,
                        frozen_at: ts.clone(),
                    };
                    let mut event =
                        JournalEntry::new("WorkflowFrozen", actor).with_project(project.as_str());
                    if let Some(thread) = thread {
                        event = event.with_thread(thread.as_str());
                    }
                    let event = event.with_payload(serde_json::to_value(&approved)?);
                    JournalEntry::append_event(conn, event.clone(), &ts).await?;
                    super::notifications::NotificationsRows::notify_plan(
                        conn,
                        &project,
                        &plan.plan_id,
                        &[],
                        &event.actor,
                        &ts,
                    )
                    .await?;
                    CommandContext::record_command(
                        conn,
                        &ctx,
                        "Workflow",
                        workflow.as_str(),
                        "WorkflowFrozen",
                        &event.event_id,
                        &ts,
                    )
                    .await?;
                    Ok(approved)
                })
            })
            .await
    }
}

/// Each changed task's decision pins name an accepted revision of `project`,
/// as Design holds it inside this write.
async fn pins(
    conn: &mut SqliteConnection,
    project: &ProjectId,
    content: &crate::PlanContent,
    changed: &[u32],
) -> Result<(), crate::PlansError> {
    let mut problems = Vec::new();
    let mut layout = None;
    for number in changed {
        let Some(contract) = content.tasks.get(number).and_then(|t| t.contract.as_ref()) else {
            continue;
        };
        for pin in &contract.decisions {
            let found =
                core_design::Design::decision_in(conn, project, &pin.id, pin.version).await?;
            if found.is_none_or(|d| d.state != core_design::DecisionState::Accepted) {
                problems.push(crate::Problem::new(format!(
                    "T{number} pins {} v{}, which is not an accepted decision",
                    pin.id, pin.version
                )));
            }
        }
        for pin in &contract.elements {
            let layout = match &layout {
                Some(layout) => layout,
                None => layout.insert(core_design::Design::layout_in(conn, project).await?),
            };
            if let Some(message) = element_pin_problem(conn, project, *number, pin, layout).await? {
                problems.push(crate::Problem::new(message));
            }
        }
    }
    if problems.is_empty() {
        Ok(())
    } else {
        Err(crate::PlansError::PlanInvalid(problems))
    }
}

async fn element_pin_problem(
    conn: &mut SqliteConnection,
    project: &ProjectId,
    number: u32,
    pin: &crate::ElementPin,
    layout: &core_design::ProjectLayout,
) -> Result<Option<String>, crate::PlansError> {
    let (id, revision, file) = (&pin.id, pin.revision, &pin.file);
    let latest = core_design::Design::element_in(conn, project, id, None).await?;
    let Some(latest) = latest.filter(|element| element.revision >= revision) else {
        return Ok(Some(format!(
            "T{number} pins {id} r{revision}, which this project's design does not have"
        )));
    };
    if latest.revision != revision {
        let latest_revision = latest.revision;
        return Ok(Some(format!(
            "T{number} pins {id} r{revision}, but its latest revision is r{latest_revision}; \
             pin r{latest_revision}"
        )));
    }
    match (pin.action == crate::ElementAction::Delete, latest.deleted) {
        (true, false) => {
            return Ok(Some(format!(
                "T{number} deletes {id}, but r{revision} does not delete it; delete it with \
             design_element_delete first"
            )));
        }
        (false, true) => {
            return Ok(Some(format!(
                "T{number} builds {id} r{revision}, which deletes it; use action delete"
            )));
        }
        _ => {}
    }
    let part = &latest.part_id;
    Ok(
        (!layout.owners(file).iter().any(|owner| &owner.part == part)).then(|| {
            format!("T{number} pins {id} in `{file}`, which its part {part} does not own")
        }),
    )
}

/// A version's plan, and that plan's project.
async fn owner(
    conn: &mut SqliteConnection,
    workflow: &WorkflowId,
) -> Result<(PlanId, ProjectId), StorageError> {
    let (plan, project): (String, String) = sqlx::query_as(
        "SELECT p.id, p.project_id
           FROM workflow w JOIN plan p ON p.id = w.plan_id
          WHERE w.id = ?",
    )
    .bind(workflow.as_str())
    .fetch_optional(&mut *conn)
    .await?
    .ok_or(StorageError::NotFound("workflow"))?;
    Ok((PlanId::from_stored(plan), ProjectId::from_recorded(project)))
}

/// A version may change only while it is a Draft at the revision the writer
/// read. The conflict's summary joins what every edit since then did.
async fn writable(
    conn: &mut SqliteConnection,
    state: &WorkflowState,
    current: i64,
    expected: i64,
    project: &ProjectId,
    workflow: &WorkflowId,
) -> Result<(), crate::PlansError> {
    if *state == WorkflowState::Frozen {
        return Err(crate::PlansError::WorkflowFrozen);
    }
    if current == expected {
        return Ok(());
    }
    let summary = ReadRows::edits_of(conn, project, workflow)
        .await?
        .into_iter()
        .filter(|e| e.revision > expected)
        .map(|e| e.summary)
        .collect::<Vec<_>>()
        .join("; ");
    Err(StorageError::RevisionConflict { current, summary }.into())
}
