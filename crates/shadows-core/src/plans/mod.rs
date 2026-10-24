//! One job: plan versions under their lifecycle rules — the `Plans` service.
//!
//! Every way to read, start, edit, show or approve a plan ends in a method
//! here: the HTTP routes as a person, the MCP tools under a grant. A
//! grant decides the writer and the reach: the internal Planner writes as its
//! thread and reaches every plan in its project; an external agent writes as
//! its grant and reaches any plan in its project. Storage
//! checks the grant again inside every write's transaction. When a
//! caller names no command, the id is derived from the command's anchor and
//! fingerprint.
//!
//! `model` holds the types, `ops` and `rules` the pure edit and check rules,
//! `conversation` the plan in the conversation, `scope` what a grant reaches,
//! and `store` the queries. All six are private: a caller reaches a plan
//! through `Plans` only.

mod binding_edit;
mod bindings;
mod conversation;
mod dependencies;
mod design_edit;
mod design_ops;
pub use design_ops::DesignOp;
mod dependency_graph;
mod map;
mod model;
mod ops;
mod rules;
mod scope;
mod signal;
pub use signal::UiSignal;
mod store;

use std::sync::Arc;

use serde_json::json;

pub use bindings::{AgreementBinding, BindingParticipant};
pub use conversation::{Focus, Place, PlanShown};
pub use dependencies::{LinkedTask, TaskParent, TaskPreview};
pub use map::{MapLink, MapPlan, PlanMap};
pub use model::{
    AcceptanceItem, Approved, DraftStarted, EditOutcome, LastEdit, Link, LinkKind, Plan,
    PlanContent, PlanId, PlanListing, PlanState, PlanTask, PlanVersions, TaskContent, TaskId,
    VersionLine, WorkflowId, WorkflowState, WrittenBy,
};
pub use ops::PlanOp;
pub use rules::Problem;

use scope::Scope;

use crate::error::{CoreError, ErrorCode};
use core_db::{Anchor, CommandContext, Storage};
use core_grants::{Grant, GrantKind};
use core_projects::ProjectId;

/// The pure rules, for `shadows_core::testing` only: its rule tests call them
/// directly, and nothing outside the crate does.
#[cfg(feature = "test-support")]
pub(crate) mod for_tests {
    pub use super::ops::Applied;
    pub use super::ops::Ops;
    pub use super::rules::Rules;
}

/// Plans: what storage holds and the running turns a Planner's `draft_start`
/// and `plan_show` anchor to. `plan_show`'s live signal is sent by Turns.
#[derive(Clone)]
pub struct Plans {
    store: Arc<store::PlansStore>,
    grants: core_grants::Grants,
    operations: core_operations::Operations,
    design: crate::design::Design,
}

/// `draft_start`'s arguments, as the tool received them.
pub struct DraftStart {
    pub title: Option<String>,
    pub goal: Option<String>,
    pub plan_id: Option<PlanId>,
    /// Why a next version is started: needed from v2 on.
    pub reason: Option<String>,
    pub draft_ref: Option<String>,
    /// The parts this plan changes; a plan cannot be approved untied.
    pub parts: Vec<crate::design::PartId>,
}

/// `plan_edit`'s arguments, as the tool received them.
pub struct PlanEdit {
    pub workflow_id: Option<WorkflowId>,
    pub expected_revision: i64,
    pub ops: Vec<PlanOp>,
    pub command_id: Option<String>,
}

/// `plan_show`'s arguments, as the tool received them.
pub struct PlanShow {
    pub workflow_id: Option<WorkflowId>,
    pub task_number: Option<u32>,
    pub place: Place,
}

impl Plans {
    pub async fn approve_as_planner(
        &self,
        grant: &Grant,
        workflow: &WorkflowId,
        expected_revision: i64,
    ) -> Result<Approved, CoreError> {
        let writer = scope::Scope::writer_of(grant)?;
        let core_grants::Writer::Planner { .. } = &writer else {
            return Err(CoreError::Refused {
                code: crate::ErrorCode::GrantScope,
                message: "approval needs a Planner's conversation".into(),
            });
        };
        let plan = self.store.get_plan(workflow).await?;
        if plan.project_id != grant.project_id {
            return Err(crate::StorageError::NotFound("workflow").into());
        }
        let approver = self.design.planner_approver(grant).await?;
        let ctx = approver.command(
            "PlanApprove",
            serde_json::json!({"workflow":workflow,"expected_revision":expected_revision}),
        );
        self.store
            .approve_plan_with(&ctx, workflow, expected_revision, &approver)
            .await
    }

    /// Active plans with latest metadata and counted links in either direction.
    pub async fn map(&self, project: &ProjectId) -> Result<PlanMap, CoreError> {
        Ok(self.store.plan_map(project).await?)
    }

    pub(crate) fn new(
        storage: Arc<Storage>,
        grants: core_grants::Grants,
        operations: core_operations::Operations,
        design: crate::design::Design,
    ) -> Self {
        Self {
            store: Arc::new(store::PlansStore { storage }),
            grants,
            operations,
            design,
        }
    }

    /// The plans of `project` by their latest versions: the Active ones, or
    /// all of them when `archived`.
    pub async fn list(
        &self,
        project: &ProjectId,
        archived: bool,
    ) -> Result<Vec<PlanListing>, CoreError> {
        Ok(self.store.list_plans(project, archived).await?)
    }

    /// One plan version, as a person reads it.
    pub async fn get(&self, workflow: &WorkflowId) -> Result<Plan, CoreError> {
        self.store.get_plan(workflow).await
    }

    /// A person's approval: "PlanApprove", params { "workflow",
    /// "expected_revision" }.
    pub async fn approve(
        &self,
        command_id: String,
        workflow: &WorkflowId,
        expected_revision: i64,
    ) -> Result<Approved, CoreError> {
        let params = serde_json::json!({
            "workflow": workflow, "expected_revision": expected_revision,
        });
        let c = CommandContext::user(command_id, "PlanApprove", params);
        self.store
            .approve_plan(&c, workflow, expected_revision)
            .await
    }

    /// One plan with every version, oldest first.
    pub async fn plan(&self, plan: &PlanId) -> Result<PlanVersions, CoreError> {
        Ok(self.store.get_plan_versions(plan).await?)
    }

    /// A person's archive command:"PlanArchive", params { "plan" }.
    pub async fn archive(
        &self,
        command_id: String,
        plan: &PlanId,
    ) -> Result<PlanVersions, CoreError> {
        let params = serde_json::json!({ "plan": plan });
        let c = CommandContext::user(command_id, "PlanArchive", params);
        Ok(self.store.archive_plan(&c, plan).await?)
    }

    /// A person's unarchive command:"PlanUnarchive", params { "plan" }.
    pub async fn unarchive(
        &self,
        command_id: String,
        plan: &PlanId,
    ) -> Result<PlanVersions, CoreError> {
        let params = serde_json::json!({ "plan": plan });
        let c = CommandContext::user(command_id, "PlanUnarchive", params);
        Ok(self.store.unarchive_plan(&c, plan).await?)
    }

    /// `workflow_list`: own or explicitly selected linked project's plans.
    pub async fn list_for(
        &self,
        grant: &Grant,
        archived: bool,
        project: Option<&str>,
    ) -> Result<Vec<PlanListing>, CoreError> {
        self.store
            .list_plans_scoped(&grant.project_id, archived, project)
            .await
    }

    /// `workflow_get`: a plan version the grant reaches.
    pub async fn get_for(
        &self,
        grant: &Grant,
        named: Option<&WorkflowId>,
        project: Option<&str>,
    ) -> Result<Plan, CoreError> {
        if let Some(project) = project {
            let id = named.ok_or_else(|| {
                Scope::refused(
                    ErrorCode::GrantScope,
                    "name the plan version with workflow_id",
                )
            })?;
            return self
                .store
                .get_plan_scoped(id, Some(&grant.project_id), Some(project))
                .await;
        }
        self.in_scope(grant, named, false).await
    }

    /// `task_get`: one task, by its number, of a plan version the grant reaches.
    pub async fn task_for(
        &self,
        grant: &Grant,
        named: Option<&WorkflowId>,
        number: u32,
    ) -> Result<PlanTask, CoreError> {
        let plan = self.in_scope(grant, named, false).await?;
        plan.tasks
            .into_iter()
            .find(|t| t.content.number == number)
            .ok_or_else(|| {
                Scope::refused(
                    ErrorCode::InvalidCommand,
                    format!("T{number} does not exist in this plan"),
                )
            })
    }

    /// `draft_prepare`: a new `draft_ref` for the grant.
    pub async fn prepare_draft(&self, grant: &Grant) -> Result<String, CoreError> {
        self.grants
            .prepare_draft(&grant.id)
            .await
            .map_err(Into::into)
    }

    /// `draft_start`: the Planner's in its own thread, an external agent's
    /// under a `draft_ref`.
    pub async fn start_draft(
        &self,
        grant: &Grant,
        args: DraftStart,
    ) -> Result<DraftStarted, CoreError> {
        let started = match grant.kind {
            GrantKind::Thread => self.planner_draft(grant, args).await,
            GrantKind::Project => self.external_draft(grant, args).await,
        };
        match started {
            // A request missing its reason, not an invalid plan.
            Err(CoreError::ReasonMissing) => Err(Scope::refused(
                ErrorCode::InvalidCommand,
                "a new version needs its reason: say in a sentence or two what \
                 changed the plan",
            )),
            other => other,
        }
    }

    /// `plan_edit`: the whole stored `EditOutcome`, or on a replay the one
    /// the first call recorded.
    pub async fn edit(&self, grant: &Grant, args: PlanEdit) -> Result<EditOutcome, CoreError> {
        let plan = self
            .in_scope(grant, args.workflow_id.as_ref(), true)
            .await?;
        let writer = Scope::writer_of(grant)?;
        // A Planner's edit names the turn it was made in, when one is running.
        let turn = match grant.kind {
            GrantKind::Thread => {
                self.operations
                    .running_for(Scope::own_thread(grant)?)
                    .await?
            }
            GrantKind::Project => None,
        };
        let expected = args.expected_revision;
        let params = json!({ "workflow": plan.id, "expected_revision": expected, "ops": args.ops });
        let fp = CommandContext::fingerprint("PlanEdit", &params);
        let id = args
            .command_id
            .unwrap_or_else(|| Anchor::derived_id(Anchor::Revision(expected), &fp));
        let ctx = Scope::command(&writer, id, "PlanEdit", fp);
        self.store
            .edit_plan(&ctx, &writer, turn.as_ref(), &plan.id, expected, &args.ops)
            .await
    }

    /// `plan_show`: records the card under the running turn's command
    /// identity and answers it with that turn and its thread. Turns names the
    /// tab and sends the live signal, which plans cannot reach.
    pub async fn show(
        &self,
        grant: &Grant,
        args: PlanShow,
    ) -> Result<
        (
            PlanShown,
            core_operations::OperationId,
            core_threads::ThreadId,
        ),
        CoreError,
    > {
        let plan = self
            .in_scope(grant, args.workflow_id.as_ref(), false)
            .await?;
        let thread = Scope::own_thread(grant)?;
        let op = self.operations.running_for(thread).await?.ok_or_else(|| {
            Scope::refused(
                ErrorCode::InvalidCommand,
                "no turn is running for this conversation",
            )
        })?;
        let writer = Scope::writer_of(grant)?;
        let params =
            json!({ "workflow": plan.id, "task_number": args.task_number, "place": args.place });
        let fp = CommandContext::fingerprint("PlanShow", &params);
        let ctx = Scope::command(
            &writer,
            Anchor::derived_id(Anchor::Operation(op.as_str()), &fp),
            "PlanShow",
            fp,
        );
        let shown = (self.store)
            .show_plan(&ctx, &writer, &op, &plan.id, args.task_number, args.place)
            .await?;
        Ok((shown, op, thread.clone()))
    }
}

mod project_links;
mod roadmap_approval;

mod design_views;
pub use design_views::{OutcomeView, PartView};

mod agreement_review;
pub use agreement_review::{AgreementParticipantImpact, AgreementPartyReview, AgreementReview};

#[cfg(feature = "test-support")]
mod testing;

mod agreements;

mod error;
pub(crate) use error::PlansError;
