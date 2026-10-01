//! One job: plan calls made under an MCP grant (spec §13.5–§13.7) — which
//! plan version a grant reaches, who writes under it, and the command
//! identity its writes carry. `Plans`' public methods call these; nothing
//! outside `plans` does, so no caller can skip or reorder the scope check.

use serde_json::json;

use super::model::{DraftStarted, Plan, WorkflowId, WorkflowState};
use super::rules::Problem;
use super::{DraftStart, Plans};
use crate::command::derive::{Anchor, derived_id};
use crate::command::{CommandContext, Writer, fingerprint};
use crate::db::StorageError;
use crate::error::{CoreError, ErrorCode};
use crate::grants::{Grant, GrantKind};
use crate::threads::ThreadId;

impl Plans {
    /// The plan version a call reaches (§13.6): reads and cards can name an
    /// older version of the plan the Planner's thread wrote, while edits
    /// require its latest. An external agent names a version in its project.
    pub(super) async fn in_scope(
        &self,
        grant: &Grant,
        named: Option<&WorkflowId>,
        latest_only: bool,
    ) -> Result<Plan, CoreError> {
        let storage = &self.storage;
        let (id, latest) = match grant.kind {
            GrantKind::Thread => {
                let latest = self
                    .thread_latest(own_thread(grant)?)
                    .await?
                    .ok_or_else(|| {
                        refused(
                            ErrorCode::InvalidCommand,
                            "this conversation has no plan yet; start one with draft_start",
                        )
                    })?;
                if latest_only && named.is_some_and(|id| id != &latest.id) {
                    return Err(scope(
                        "a Planner edits only its own conversation's latest plan version; \
                         leave workflow_id out",
                    ));
                }
                (named.cloned().unwrap_or(latest.id.clone()), Some(latest))
            }
            GrantKind::Project => (
                named.cloned().ok_or_else(|| {
                    scope("name the plan version with workflow_id; workflow_list lists them")
                })?,
                None,
            ),
        };
        let plan = match storage.get_plan(&id).await {
            Err(StorageError::NotFound(_)) if grant.kind == GrantKind::Project => {
                return Err(scope("that plan is not in this grant's project"));
            }
            result => result?,
        };
        if plan.project_id != grant.project_id {
            return Err(scope("that plan is not in this grant's project"));
        }
        // Until Task 3 widens it, a Planner reaches only the plan its thread wrote.
        if let Some(latest) = latest
            && plan.plan_id != latest.plan_id
        {
            return Err(scope("that plan is not in this grant's conversation"));
        }
        Ok(plan)
    }

    /// The latest version of the plan `thread` wrote (`written_by_thread`).
    async fn thread_latest(&self, thread: &ThreadId) -> Result<Option<Plan>, CoreError> {
        match self.storage.thread_plan(thread).await? {
            Some(id) => Ok(Some(self.storage.get_plan(&id).await?)),
            None => Ok(None),
        }
    }

    /// The Planner's `draft_start`: in the plan its thread wrote, or a new
    /// one, anchored to the turn that is running, so the same call within the
    /// turn answers the first result and a call naming another source is
    /// another command (§13.5). A source must be in that plan, and be the
    /// version the start answers anyway (§13.6): its Draft, or with none its
    /// latest Frozen version. Storage checks the second after the replay,
    /// which a start moves on.
    pub(super) async fn planner_draft(
        &self,
        grant: &Grant,
        args: DraftStart,
    ) -> Result<DraftStarted, CoreError> {
        let thread = own_thread(grant)?;
        let plan = self.thread_latest(thread).await?.map(|p| p.plan_id);
        if let Some(from) = &args.from_workflow_id
            && Some(self.storage.get_plan(from).await?.plan_id) != plan
        {
            return Err(scope(
                "a Planner starts versions only in its own conversation",
            ));
        }
        let op = self.handles.running_for(thread).await.ok_or_else(|| {
            refused(
                ErrorCode::InvalidCommand,
                "no turn is running for this conversation",
            )
        })?;
        let writer = writer_of(grant)?;
        let mut params = json!({ "thread": thread, "title": args.title, "goal": args.goal });
        // Absent when not given, so a start recorded without them replays as it did.
        if let Some(from) = &args.from_workflow_id {
            params["from_workflow_id"] = json!(from);
        }
        if let Some(reason) = &args.reason {
            params["reason"] = json!(reason);
        }
        let fp = fingerprint("DraftStart", &params);
        let ctx = command(
            &writer,
            derived_id(Anchor::Operation(&op), &fp),
            "DraftStart",
            fp,
        );
        let fresh = args.title.as_deref().zip(args.goal.as_deref());
        let source = args.from_workflow_id.as_ref();
        match self
            .storage
            .start_draft(
                &ctx,
                &writer,
                &grant.project_id,
                plan.as_ref(),
                source,
                fresh,
                args.reason.as_deref(),
                Some(&op),
                None,
            )
            .await
        {
            // A request naming the wrong version, not one outside the grant.
            Err(StorageError::NotLatestVersion(latest)) => Err(refused(
                ErrorCode::InvalidCommand,
                format!(
                    "a Planner starts a draft from its conversation's latest version, \
                     {latest}; name it, or leave from_workflow_id out"
                ),
            )),
            result => Ok(result?),
        }
    }

    /// An external agent's `draft_start`: always under a `draft_ref`, which is
    /// the command's id (§13.5) — from an approved plan in its project, or
    /// from scratch, a new plan with no conversation (§16.3).
    pub(super) async fn external_draft(
        &self,
        grant: &Grant,
        args: DraftStart,
    ) -> Result<DraftStarted, CoreError> {
        let draft_ref = args.draft_ref.as_deref().ok_or_else(|| {
            scope("an external agent starts a plan with a draft_ref from draft_prepare")
        })?;
        let writer = writer_of(grant)?;
        let mut params = json!({
            "title": args.title,
            "goal": args.goal,
            "from_workflow_id": args.from_workflow_id,
        });
        // Absent when not given, so a start recorded without one replays as it did.
        if let Some(reason) = &args.reason {
            params["reason"] = json!(reason);
        }
        let fp = fingerprint("DraftStart", &params);
        let ctx = command(
            &writer,
            derived_id(Anchor::DraftRef(draft_ref), &fp),
            "DraftStart",
            fp,
        );
        let storage = &self.storage;
        let plan = match &args.from_workflow_id {
            Some(from) => {
                let plan = match storage.get_plan(from).await {
                    Err(StorageError::NotFound(_)) => {
                        return Err(scope("that plan is not in this grant's project"));
                    }
                    result => result?,
                };
                if plan.project_id != grant.project_id {
                    return Err(scope("that plan is not in this grant's project"));
                }
                // A state error, not a scope one: the plan is in the project.
                if plan.state != WorkflowState::Frozen {
                    return Err(refused(
                        ErrorCode::InvalidCommand,
                        "that plan version is a draft: edit it with plan_edit, or start a \
                         new version from an approved one",
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
                return Err(StorageError::PlanInvalid(vec![Problem {
                    message: "a plan's first version needs a title and a goal".into(),
                }])
                .into());
            }
        };
        Ok(storage
            .start_draft(
                &ctx,
                &writer,
                &grant.project_id,
                plan.as_ref(),
                None,
                fresh,
                args.reason.as_deref(),
                None,
                Some(draft_ref),
            )
            .await?)
    }
}

/// A refusal whose code and text this service writes.
pub(super) fn refused(code: ErrorCode, message: impl Into<String>) -> CoreError {
    CoreError::Refused {
        code,
        message: message.into(),
    }
}

/// A plan, thread or ref outside what the grant reaches (§13.6).
pub(super) fn scope(message: impl Into<String>) -> CoreError {
    refused(ErrorCode::GrantScope, message)
}

/// A thread grant's thread. Storage never issues one without it.
pub(super) fn own_thread(grant: &Grant) -> Result<&ThreadId, CoreError> {
    grant
        .thread_id
        .as_ref()
        .ok_or_else(|| StorageError::GrantScope.into())
}

/// Who writes under `grant`: the Planner as its thread, an external agent as
/// its grant (§13.5).
pub(super) fn writer_of(grant: &Grant) -> Result<Writer, CoreError> {
    Ok(match grant.kind {
        GrantKind::Thread => Writer::Planner {
            thread: own_thread(grant)?.clone(),
            grant: grant.id.clone(),
        },
        GrantKind::Project => Writer::External {
            grant: grant.id.clone(),
        },
    })
}

pub(super) fn command(
    writer: &Writer,
    command_id: String,
    kind: &str,
    fp: String,
) -> CommandContext {
    let (principal_kind, principal_id) = writer.principal();
    CommandContext {
        principal_kind: principal_kind.into(),
        principal_id,
        command_id,
        command_kind: kind.into(),
        command_schema_ver: 1,
        request_fingerprint: fp,
    }
}
