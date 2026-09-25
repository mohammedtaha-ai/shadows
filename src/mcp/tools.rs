//! One job: each MCP tool's arguments, and the plan storage call it makes
//! (spec §13.5–§13.6).
//!
//! A grant decides the writer and the reach: the internal Planner writes as
//! its thread and reaches that thread's latest plan version only; an external
//! agent writes as its grant and reaches any plan in its project. Storage
//! checks the grant again inside every write's transaction (§13.7). When a
//! caller names no command, the id is derived as §13.5's table says.

use rmcp::handler::server::tool::Extension;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use rmcp::{tool, tool_router};
use serde::Deserialize;
use serde_json::json;

use super::grant::{Grant, GrantKind};
use super::refusal::{Refusal, answer};
use super::server::Shadows;
use crate::command::derive::{Anchor, derived_id};
use crate::command::{CommandContext, Writer, fingerprint};
use crate::error::ErrorCode;
use crate::storage::StorageError;
use crate::workflow::{Plan, PlanOp, WorkflowId};

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct PlanArgs {
    /// The plan version. The Planner leaves it out: its conversation's latest
    /// version is the one it reaches. An external agent must name one.
    #[serde(default)]
    #[schemars(with = "Option<String>")]
    workflow_id: Option<WorkflowId>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct TaskArgs {
    /// As for `workflow_get`.
    #[serde(default)]
    #[schemars(with = "Option<String>")]
    workflow_id: Option<WorkflowId>,
    /// The task's number: 4 for T4.
    number: u32,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct DraftStartArgs {
    /// The first version's title. Needed only when there is no plan yet.
    #[serde(default)]
    title: Option<String>,
    /// The first version's goal. Needed only when there is no plan yet.
    #[serde(default)]
    goal: Option<String>,
    /// External agents: an approved plan in the project, to start its next
    /// version from. Without it, a new conversation and plan are created.
    #[serde(default)]
    #[schemars(with = "Option<String>")]
    from_workflow_id: Option<WorkflowId>,
    /// External agents: the ref `draft_prepare` answered. The same ref always
    /// answers the same plan; a new plan takes a new ref.
    #[serde(default)]
    draft_ref: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct PlanEditArgs {
    /// As for `workflow_get`.
    #[serde(default)]
    #[schemars(with = "Option<String>")]
    workflow_id: Option<WorkflowId>,
    /// The revision the edit was built on, as `workflow_get` answered it.
    expected_revision: i64,
    /// Applied together or not at all; the revision moves once.
    ops: Vec<PlanOp>,
    /// Leave it out: Shadows derives one, so a retry of the same edit
    /// answers the first result.
    #[serde(default)]
    command_id: Option<String>,
}

#[tool_router(vis = "pub(super)")]
impl Shadows {
    #[tool(description = "List the plans in this project: each one's latest version.")]
    async fn workflow_list(&self, Extension(grant): Extension<Grant>) -> CallToolResult {
        answer(
            self.state
                .storage
                .list_plans(&grant.project_id)
                .await
                .map_err(Refusal::from),
        )
    }

    #[tool(
        description = "Read a plan version: title, goal, tasks, links, revision, state, and what blocks approval."
    )]
    async fn workflow_get(
        &self,
        Extension(grant): Extension<Grant>,
        Parameters(args): Parameters<PlanArgs>,
    ) -> CallToolResult {
        answer(self.plan_in_scope(&grant, args.workflow_id.as_ref()).await)
    }

    #[tool(description = "Read one task of a plan version by its number.")]
    async fn task_get(
        &self,
        Extension(grant): Extension<Grant>,
        Parameters(args): Parameters<TaskArgs>,
    ) -> CallToolResult {
        let plan = self.plan_in_scope(&grant, args.workflow_id.as_ref()).await;
        answer(plan.and_then(|plan| {
            plan.tasks
                .into_iter()
                .find(|t| t.content.number == args.number)
                .ok_or_else(|| {
                    Refusal::new(
                        ErrorCode::InvalidCommand,
                        format!("T{} does not exist in this plan", args.number),
                    )
                })
        }))
    }

    #[tool(
        description = "Get a draft_ref for draft_start. Call it once for each new plan you intend; a ref not used within an hour expires."
    )]
    async fn draft_prepare(&self, Extension(grant): Extension<Grant>) -> CallToolResult {
        let issued = self.state.storage.prepare_draft(&grant.id).await;
        answer(
            issued
                .map(|r| json!({ "draft_ref": r }))
                .map_err(Refusal::from),
        )
    }

    #[tool(
        description = "Start a plan version to edit. With no plan yet, creates version 1 from a title and a goal; after an approved version, creates the next version as its copy; with a draft already there, answers it."
    )]
    async fn draft_start(
        &self,
        Extension(grant): Extension<Grant>,
        Parameters(args): Parameters<DraftStartArgs>,
    ) -> CallToolResult {
        answer(match grant.kind {
            GrantKind::Thread => self.planner_draft(&grant, args).await,
            GrantKind::Project => self.external_draft(&grant, args).await,
        })
    }

    #[tool(
        description = "Edit a draft plan version at expected_revision with a list of operations: plan_put, task_add, task_update, task_remove, link_put, link_remove. Put changes made together in one call."
    )]
    async fn plan_edit(
        &self,
        Extension(grant): Extension<Grant>,
        Parameters(args): Parameters<PlanEditArgs>,
    ) -> CallToolResult {
        answer(self.edit(&grant, args).await)
    }
}

impl Shadows {
    /// The plan version a call reaches (§13.6): for the Planner, its thread's
    /// latest version, and nothing else; for an external agent, the version it
    /// names, if that is in its project.
    async fn plan_in_scope(
        &self,
        grant: &Grant,
        named: Option<&WorkflowId>,
    ) -> Result<Plan, Refusal> {
        let storage = &self.state.storage;
        let id = match grant.kind {
            GrantKind::Thread => {
                let latest = storage
                    .thread_plan(own_thread(grant)?)
                    .await?
                    .ok_or_else(|| {
                        Refusal::new(
                            ErrorCode::InvalidCommand,
                            "this conversation has no plan yet; start one with draft_start",
                        )
                    })?;
                if named.is_some_and(|id| id != &latest) {
                    return Err(Refusal::scope(
                        "a Planner reads and edits only its own conversation's latest plan \
                         version; leave workflow_id out",
                    ));
                }
                latest
            }
            GrantKind::Project => named.cloned().ok_or_else(|| {
                Refusal::new(
                    ErrorCode::InvalidCommand,
                    "name the plan version with workflow_id; workflow_list lists them",
                )
            })?,
        };
        let plan = storage.get_plan(&id).await?;
        if plan.project_id != grant.project_id {
            return Err(Refusal::scope("that plan is not in this grant's project"));
        }
        Ok(plan)
    }

    /// The Planner's `draft_start`: in its own thread, anchored to the turn
    /// that is running, so a retry within the turn answers the first result.
    async fn planner_draft(
        &self,
        grant: &Grant,
        args: DraftStartArgs,
    ) -> Result<serde_json::Value, Refusal> {
        let thread = own_thread(grant)?;
        if let Some(from) = &args.from_workflow_id
            && &self.state.storage.get_plan(from).await?.thread_id != thread
        {
            return Err(Refusal::scope(
                "a Planner starts versions only in its own conversation",
            ));
        }
        let op = self
            .state
            .handles
            .running_for(thread)
            .await
            .ok_or_else(|| {
                Refusal::new(
                    ErrorCode::InvalidCommand,
                    "no turn is running for this conversation",
                )
            })?;
        let writer = writer_of(grant)?;
        let params = json!({ "thread": thread, "title": args.title, "goal": args.goal });
        let fp = fingerprint("DraftStart", &params);
        let ctx = command(
            &writer,
            derived_id(Anchor::Operation(&op), &fp),
            "DraftStart",
            fp,
        );
        let fresh = args.title.as_deref().zip(args.goal.as_deref());
        let started = self
            .state
            .storage
            .start_draft(&ctx, &writer, thread, fresh, None)
            .await?;
        Ok(json!(started))
    }

    /// An external agent's `draft_start`: always under a `draft_ref`, which is
    /// the command's id (§13.5) — from an approved plan in its project, or
    /// from scratch with a new thread.
    async fn external_draft(
        &self,
        grant: &Grant,
        args: DraftStartArgs,
    ) -> Result<serde_json::Value, Refusal> {
        let draft_ref = args.draft_ref.as_deref().ok_or_else(|| {
            Refusal::scope("an external agent starts a plan with a draft_ref from draft_prepare")
        })?;
        let writer = writer_of(grant)?;
        let params = json!({
            "title": args.title,
            "goal": args.goal,
            "from_workflow_id": args.from_workflow_id,
        });
        let fp = fingerprint("DraftStart", &params);
        let ctx = command(
            &writer,
            derived_id(Anchor::DraftRef(draft_ref), &fp),
            "DraftStart",
            fp,
        );
        let storage = &self.state.storage;
        let started = match &args.from_workflow_id {
            Some(from) => {
                let plan = storage.get_plan(from).await?;
                if plan.project_id != grant.project_id {
                    return Err(Refusal::scope("that plan is not in this grant's project"));
                }
                storage
                    .start_draft(&ctx, &writer, &plan.thread_id, None, Some(draft_ref))
                    .await?
            }
            None => {
                let (Some(title), Some(goal)) = (&args.title, &args.goal) else {
                    return Err(StorageError::PlanInvalid(vec![crate::workflow::Problem {
                        message: "a plan's first version needs a title and a goal".into(),
                    }])
                    .into());
                };
                storage
                    .start_thread_with_draft(
                        &ctx,
                        &writer,
                        &grant.project_id,
                        title,
                        goal,
                        Some(draft_ref),
                    )
                    .await?
            }
        };
        Ok(json!(started))
    }

    /// `plan_edit`: the whole stored `EditOutcome`, or on a replay the one
    /// the first call recorded.
    async fn edit(&self, grant: &Grant, args: PlanEditArgs) -> Result<serde_json::Value, Refusal> {
        let plan = self.plan_in_scope(grant, args.workflow_id.as_ref()).await?;
        let writer = writer_of(grant)?;
        let expected = args.expected_revision;
        let params = json!({ "workflow": plan.id, "expected_revision": expected, "ops": args.ops });
        let fp = fingerprint("PlanEdit", &params);
        let id = args
            .command_id
            .unwrap_or_else(|| derived_id(Anchor::Revision(expected), &fp));
        let ctx = command(&writer, id, "PlanEdit", fp);
        let outcome = self
            .state
            .storage
            .edit_plan(&ctx, &writer, &plan.id, expected, &args.ops)
            .await?;
        Ok(json!(outcome))
    }
}

/// A thread grant's thread. Storage never issues one without it.
fn own_thread(grant: &Grant) -> Result<&crate::thread::ThreadId, Refusal> {
    grant
        .thread_id
        .as_ref()
        .ok_or_else(|| StorageError::GrantScope.into())
}

/// Who writes under `grant`: the Planner as its thread, an external agent as
/// its grant (§13.5).
fn writer_of(grant: &Grant) -> Result<Writer, Refusal> {
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

fn command(writer: &Writer, command_id: String, kind: &str, fp: String) -> CommandContext {
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
