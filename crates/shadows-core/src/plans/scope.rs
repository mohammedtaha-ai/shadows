//! One job: plan calls made under an MCP grant — which
//! plan version a grant reaches, who writes under it, and the command
//! identity its writes carry. `Plans`' public methods call these; nothing
//! outside `plans` does, so no caller can skip or reorder the scope check.

use serde_json::json;

use super::model::{DraftStarted, Plan, PlanId, WorkflowId};
use super::rules::Problem;
use super::{DraftStart, Plans};
use crate::error::{CoreError, ErrorCode};
use core_db::{Anchor, CommandContext, StorageError};
use core_grants::Grant;
use core_grants::Writer;
use core_threads::ThreadId;

impl Plans {
    /// The plan version a call reaches: every grant names a version
    /// in its project, while edits require an Active plan's latest version.
    pub(super) async fn in_scope(
        &self,
        grant: &Grant,
        named: Option<&WorkflowId>,
        latest_only: bool,
    ) -> Result<Plan, CoreError> {
        let id = named.cloned().ok_or_else(|| {
            Scope::scope(
                "name the plan version with workflow_id; workflow_list lists the project's plans",
            )
        })?;
        let plan = match self
            .store
            .get_plan_scoped(&id, Some(&grant.project_id), None)
            .await
        {
            Err(
                CoreError::Storage(StorageError::NotFound(_))
                | CoreError::Refused {
                    code: ErrorCode::GrantScope,
                    ..
                },
            ) => {
                return Err(Scope::scope("that plan is not in this grant's project"));
            }
            result => result?,
        };
        if plan.project_id != grant.project_id {
            return Err(Scope::scope("that plan is not in this grant's project"));
        }
        if latest_only {
            if plan.plan_state == super::model::PlanState::Archived {
                return Err(Scope::refused(
                    ErrorCode::InvalidCommand,
                    format!(
                        "plan {} is archived; a person can unarchive it",
                        plan.plan_id
                    ),
                ));
            }
            let latest = self
                .store
                .list_plans(&grant.project_id, true)
                .await?
                .into_iter()
                .find(|listing| listing.plan_id == plan.plan_id)
                .ok_or_else(|| Scope::scope("that plan is not in this grant's project"))?;
            if plan.id != latest.id {
                return Err(Scope::scope("plan_edit needs the plan's latest version"));
            }
        }
        Ok(plan)
    }

    /// The Planner's `draft_start`: in a named Active plan, or a new one,
    /// anchored to the turn that is running.
    pub(super) async fn planner_draft(
        &self,
        grant: &Grant,
        args: DraftStart,
    ) -> Result<DraftStarted, CoreError> {
        let thread = Scope::own_thread(grant)?;
        let plan = self
            .plan_in_project(&grant.project_id, args.plan_id.as_ref())
            .await?;
        if let Some(plan) = &plan
            && plan.plan_state == super::model::PlanState::Archived
        {
            return Err(Scope::refused(
                ErrorCode::InvalidCommand,
                format!(
                    "plan {} is archived; a person can unarchive it",
                    plan.plan_id
                ),
            ));
        }
        let op = self.operations.running_for(thread).await?.ok_or_else(|| {
            Scope::refused(
                ErrorCode::InvalidCommand,
                "no turn is running for this conversation",
            )
        })?;
        let writer = Scope::writer_of(grant)?;
        let params = json!({
            "project": grant.project_id,
            "plan_id": args.plan_id,
            "title": args.title,
            "goal": args.goal,
            "reason": args.reason,
        });
        let params = with_parts(params, &args.parts);
        let fp = CommandContext::fingerprint("DraftStart", &params);
        let ctx = Scope::command(
            &writer,
            Anchor::derived_id(Anchor::Operation(op.as_str()), &fp),
            "DraftStart",
            fp,
        );
        let plan_id = args.plan_id.as_ref();
        let fresh = if plan_id.is_none() {
            args.title.as_deref().zip(args.goal.as_deref())
        } else {
            None
        };
        self.store
            .start_draft(
                &ctx,
                &writer,
                &grant.project_id,
                plan_id,
                None,
                fresh,
                args.reason.as_deref(),
                Some(&op),
                None,
                &args.parts,
            )
            .await
    }

    async fn plan_in_project(
        &self,
        project: &core_projects::ProjectId,
        plan_id: Option<&PlanId>,
    ) -> Result<Option<super::model::PlanListing>, CoreError> {
        let Some(plan_id) = plan_id else {
            return Ok(None);
        };
        self.store
            .list_plans(project, true)
            .await?
            .into_iter()
            .find(|plan| &plan.plan_id == plan_id)
            .map(Some)
            .ok_or_else(|| Scope::scope("that plan is not in this grant's project"))
    }

    /// An external agent's `draft_start`: always under a `draft_ref`, either
    /// in a named plan or from scratch.
    pub(super) async fn external_draft(
        &self,
        grant: &Grant,
        args: DraftStart,
    ) -> Result<DraftStarted, CoreError> {
        let draft_ref = args.draft_ref.as_deref().ok_or_else(|| {
            Scope::scope("an external agent starts a plan with a draft_ref from draft_prepare")
        })?;
        let writer = Scope::writer_of(grant)?;
        let params = json!({
            "title": args.title,
            "goal": args.goal,
            "plan_id": args.plan_id,
            "project": grant.project_id,
            "reason": args.reason,
        });
        let params = with_parts(params, &args.parts);
        let fp = CommandContext::fingerprint("DraftStart", &params);
        let ctx = Scope::command(
            &writer,
            Anchor::derived_id(Anchor::DraftRef(draft_ref), &fp),
            "DraftStart",
            fp,
        );
        let storage = &self.store;
        let plan = match self
            .plan_in_project(&grant.project_id, args.plan_id.as_ref())
            .await?
        {
            Some(plan) => {
                if plan.plan_state == super::model::PlanState::Archived {
                    return Err(Scope::refused(
                        ErrorCode::InvalidCommand,
                        format!(
                            "plan {} is archived; a person can unarchive it",
                            plan.plan_id
                        ),
                    ));
                }
                Some(plan.plan_id)
            }
            None => None,
        };
        let fresh = match (&plan, &args.title, &args.goal) {
            (Some(_), _, _) => None,
            (None, Some(title), Some(goal)) => Some((title.as_str(), goal.as_str())),
            (None, _, _) => {
                return Err(crate::plans::PlansError::PlanInvalid(vec![Problem {
                    message: "a plan's first version needs a title and a goal".into(),
                }])
                .into());
            }
        };
        storage
            .start_draft(
                &ctx,
                &writer,
                &grant.project_id,
                args.plan_id.as_ref(),
                None,
                fresh,
                args.reason.as_deref(),
                None,
                Some(draft_ref),
                &args.parts,
            )
            .await
    }
}

/// The fingerprint names `parts` only when given, so older starts replay unchanged.
fn with_parts(mut params: serde_json::Value, parts: &[crate::design::PartId]) -> serde_json::Value {
    if !parts.is_empty() {
        params["parts"] = json!(parts);
    }
    params
}

/// A refusal whose code and text this service writes.
impl Scope {
    pub(super) fn refused(code: ErrorCode, message: impl Into<String>) -> CoreError {
        CoreError::Refused {
            code,
            message: message.into(),
        }
    }
}

/// A plan, thread or ref outside what the grant reaches.
impl Scope {
    pub(super) fn scope(message: impl Into<String>) -> CoreError {
        Scope::refused(ErrorCode::GrantScope, message)
    }
}

/// A thread grant's thread. Storage never issues one without it.
impl Scope {
    pub(super) fn own_thread(grant: &Grant) -> Result<&ThreadId, CoreError> {
        grant
            .thread_id
            .as_ref()
            .ok_or_else(|| core_grants::GrantsError::GrantScope.into())
    }
}

/// Who writes under `grant`: the Planner as its thread, an external agent as
/// its grant.
impl Scope {
    pub(super) fn writer_of(grant: &Grant) -> Result<Writer, CoreError> {
        Writer::from_grant(grant, None).map_err(Into::into)
    }
}

impl Scope {
    pub(super) fn command(
        writer: &Writer,
        command_id: String,
        kind: &str,
        fp: String,
    ) -> CommandContext {
        let (principal_kind, principal_id) = writer.principal();
        CommandContext {
            agent: None,
            principal_kind: principal_kind.into(),
            principal_id,
            command_id,
            command_kind: kind.into(),
            command_schema_ver: 1,
            request_fingerprint: fp,
        }
    }
}

pub(in crate::plans) struct Scope;
