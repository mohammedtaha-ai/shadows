//! One job: plan versions under the rules of spec §13 — the `Plans` service.
//!
//! Every way to read, start, edit, show or approve a plan ends in a method
//! here: the HTTP routes as a person, the MCP tools under a grant (§14.4). A
//! grant decides the writer and the reach: the internal Planner writes as its
//! thread and reaches that thread's latest plan version only; an external
//! agent writes as its grant and reaches any plan in its project. Storage
//! checks the grant again inside every write's transaction (§13.7). When a
//! caller names no command, the id is derived as §13.5's table says.
//!
//! `model` holds the types, `ops` and `rules` the pure edit and check rules,
//! `conversation` the plan in the conversation, `scope` what a grant reaches,
//! and `store` the queries. All six are private: a caller reaches a plan
//! through `Plans` only.

mod conversation;
mod model;
mod ops;
mod rules;
mod scope;
mod store;

use std::sync::Arc;

use serde_json::json;

pub use conversation::{Focus, Place, PlanShown};
pub use model::{
    AcceptanceItem, Approved, DraftStarted, EditOutcome, LastEdit, Link, LinkKind, Plan,
    PlanContent, PlanListing, PlanTask, TaskContent, TaskId, WorkflowId, WorkflowState,
};
pub use ops::PlanOp;
pub use rules::Problem;
pub(crate) use store::task_of;

use scope::{command, own_thread, refused, writer_of};

use crate::app::user_command;
use crate::command::derive::{Anchor, derived_id};
use crate::command::fingerprint;
use crate::error::{CoreError, ErrorCode};
use crate::events::UiSignal;
use crate::grant::{Grant, GrantKind};
use crate::planner::LiveHandles;
use crate::project::ProjectId;
use crate::storage::Storage;

/// The pure rules, for `shadows_core::testing` only: its rule tests call them
/// directly, and nothing outside the crate does.
#[cfg(feature = "test-support")]
pub(crate) mod for_tests {
    pub use super::ops::{Applied, apply};
    pub use super::rules::{approval_problems, edit_problems};
}

/// Plans: what storage holds, the live turns a Planner's `draft_start` and
/// `plan_show` anchor to, and the live-only signal `plan_show` sends (§13.9).
pub struct Plans {
    storage: Arc<Storage>,
    handles: Arc<LiveHandles>,
    ui: tokio::sync::broadcast::Sender<UiSignal>,
}

/// `draft_start`'s arguments, as the tool received them.
pub struct DraftStart {
    pub title: Option<String>,
    pub goal: Option<String>,
    pub from_workflow_id: Option<WorkflowId>,
    pub draft_ref: Option<String>,
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
    pub(crate) fn new(
        storage: Arc<Storage>,
        handles: Arc<LiveHandles>,
        ui: tokio::sync::broadcast::Sender<UiSignal>,
    ) -> Self {
        Self {
            storage,
            handles,
            ui,
        }
    }

    /// Each planning thread's latest plan version in `project`.
    pub async fn list(&self, project: &ProjectId) -> Result<Vec<PlanListing>, CoreError> {
        Ok(self.storage.list_plans(project).await?)
    }

    /// One plan version, as a person reads it.
    pub async fn get(&self, workflow: &WorkflowId) -> Result<Plan, CoreError> {
        Ok(self.storage.get_plan(workflow).await?)
    }

    /// A person's approval (§13.2): "PlanApprove", params { "workflow",
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
        let c = user_command(command_id, "PlanApprove", params);
        Ok(self
            .storage
            .approve_plan(&c, workflow, expected_revision)
            .await?)
    }

    /// `workflow_list`: the plans in the grant's project.
    pub async fn list_for(&self, grant: &Grant) -> Result<Vec<PlanListing>, CoreError> {
        Ok(self.storage.list_plans(&grant.project_id).await?)
    }

    /// `workflow_get`: a plan version the grant reaches.
    pub async fn get_for(
        &self,
        grant: &Grant,
        named: Option<&WorkflowId>,
    ) -> Result<Plan, CoreError> {
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
                refused(
                    ErrorCode::InvalidCommand,
                    format!("T{number} does not exist in this plan"),
                )
            })
    }

    /// `draft_prepare`: a new `draft_ref` for the grant.
    pub async fn prepare_draft(&self, grant: &Grant) -> Result<String, CoreError> {
        Ok(self.storage.prepare_draft(&grant.id).await?)
    }

    /// `draft_start`: the Planner's in its own thread, an external agent's
    /// under a `draft_ref`.
    pub async fn start_draft(
        &self,
        grant: &Grant,
        args: DraftStart,
    ) -> Result<DraftStarted, CoreError> {
        match grant.kind {
            GrantKind::Thread => self.planner_draft(grant, args).await,
            GrantKind::Project => self.external_draft(grant, args).await,
        }
    }

    /// `plan_edit`: the whole stored `EditOutcome`, or on a replay the one
    /// the first call recorded.
    pub async fn edit(&self, grant: &Grant, args: PlanEdit) -> Result<EditOutcome, CoreError> {
        let plan = self
            .in_scope(grant, args.workflow_id.as_ref(), true)
            .await?;
        let writer = writer_of(grant)?;
        let expected = args.expected_revision;
        let params = json!({ "workflow": plan.id, "expected_revision": expected, "ops": args.ops });
        let fp = fingerprint("PlanEdit", &params);
        let id = args
            .command_id
            .unwrap_or_else(|| derived_id(Anchor::Revision(expected), &fp));
        let ctx = command(&writer, id, "PlanEdit", fp);
        Ok(self
            .storage
            .edit_plan(&ctx, &writer, &plan.id, expected, &args.ops)
            .await?)
    }

    /// `plan_show` (§13.9): records the card, under the running turn's
    /// command identity, then sends one live signal naming the tab that sent
    /// the turn — unless it was a replay, whose card is already there.
    pub async fn show(&self, grant: &Grant, args: PlanShow) -> Result<PlanShown, CoreError> {
        let plan = self
            .in_scope(grant, args.workflow_id.as_ref(), false)
            .await?;
        let thread = own_thread(grant)?;
        let (op, tab) = (self.handles.running_turn(thread).await).ok_or_else(|| {
            refused(
                ErrorCode::InvalidCommand,
                "no turn is running for this conversation",
            )
        })?;
        let writer = writer_of(grant)?;
        let params =
            json!({ "workflow": plan.id, "task_number": args.task_number, "place": args.place });
        let fp = fingerprint("PlanShow", &params);
        let ctx = command(
            &writer,
            derived_id(Anchor::Operation(&op), &fp),
            "PlanShow",
            fp,
        );
        let shown = (self.storage)
            .show_plan(&ctx, &writer, &op, &plan.id, args.task_number, args.place)
            .await?;
        if !shown.replayed {
            // No live subscriber is not a failure: the card is journaled.
            let _ = self.ui.send(UiSignal {
                thread_id: thread.clone(),
                target_tab: tab,
                workflow_id: shown.workflow_id.clone(),
                version: shown.version,
                task_number: shown.task_number,
                place: shown.place,
            });
        }
        Ok(shown)
    }
}
