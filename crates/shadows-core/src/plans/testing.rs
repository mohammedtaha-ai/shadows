//! Test-only entry points with explicit command contexts.
use super::{
    Approved, DraftStarted, EditOutcome, Place, Plan, PlanId, PlanListing, PlanMap, PlanOp,
    PlanShown, PlanVersions, Plans, WorkflowId,
};
use crate::design::PartId;
use core_db::{CommandContext, StorageError};
use core_grants::Writer;
use core_operations::OperationId;
use core_projects::ProjectId;
use core_threads::ThreadId;
impl Plans {
    pub async fn link_plan_part_for_test(
        &self,
        p: &ProjectId,
        plan: &PlanId,
        part: &PartId,
    ) -> Result<(), StorageError> {
        self.store.link_plan_part_for_tests(p, plan, part).await
    }
    #[allow(clippy::too_many_arguments)]
    pub async fn start_draft_for_test(
        &self,
        ctx: &CommandContext,
        writer: &Writer,
        project: &ProjectId,
        plan: Option<&PlanId>,
        source: Option<&WorkflowId>,
        fresh: Option<(&str, &str)>,
        reason: Option<&str>,
        operation: Option<&OperationId>,
        draft_ref: Option<&str>,
        parts: &[PartId],
    ) -> Result<DraftStarted, crate::CoreError> {
        self.store
            .start_draft(
                ctx, writer, project, plan, source, fresh, reason, operation, draft_ref, parts,
            )
            .await
    }
    pub async fn edit_plan_for_test(
        &self,
        ctx: &CommandContext,
        writer: &Writer,
        operation: Option<&OperationId>,
        workflow: &WorkflowId,
        expected_revision: i64,
        ops: &[PlanOp],
    ) -> Result<EditOutcome, crate::CoreError> {
        self.store
            .edit_plan(ctx, writer, operation, workflow, expected_revision, ops)
            .await
    }
    pub async fn approve_plan_for_test(
        &self,
        ctx: &CommandContext,
        workflow: &WorkflowId,
        expected_revision: i64,
    ) -> Result<Approved, crate::CoreError> {
        self.store
            .approve_plan(ctx, workflow, expected_revision)
            .await
    }
    pub async fn approve_plan_with_for_test(
        &self,
        ctx: &CommandContext,
        workflow: &WorkflowId,
        expected_revision: i64,
        approver: &core_grants::Approver,
    ) -> Result<Approved, crate::CoreError> {
        self.store
            .approve_plan_with(ctx, workflow, expected_revision, approver)
            .await
    }
    pub async fn plan_map_for_test(&self, project: &ProjectId) -> Result<PlanMap, StorageError> {
        self.store.plan_map(project).await
    }
    pub async fn get_plan_versions_for_test(
        &self,
        plan: &PlanId,
    ) -> Result<PlanVersions, StorageError> {
        self.store.get_plan_versions(plan).await
    }
    pub async fn archive_plan_for_test(
        &self,
        ctx: &CommandContext,
        plan: &PlanId,
    ) -> Result<PlanVersions, StorageError> {
        self.store.archive_plan(ctx, plan).await
    }
    pub async fn unarchive_plan_for_test(
        &self,
        ctx: &CommandContext,
        plan: &PlanId,
    ) -> Result<PlanVersions, StorageError> {
        self.store.unarchive_plan(ctx, plan).await
    }
    pub async fn get_plan_for_test(&self, workflow: &WorkflowId) -> Result<Plan, crate::CoreError> {
        self.store.get_plan(workflow).await
    }
    pub async fn get_plan_scoped_for_test(
        &self,
        workflow: &WorkflowId,
        origin: Option<&ProjectId>,
        project: Option<&str>,
    ) -> Result<Plan, crate::CoreError> {
        self.store.get_plan_scoped(workflow, origin, project).await
    }
    pub async fn list_plans_scoped_for_test(
        &self,
        origin: &ProjectId,
        archived: bool,
        project: Option<&str>,
    ) -> Result<Vec<PlanListing>, crate::CoreError> {
        self.store
            .list_plans_scoped(origin, archived, project)
            .await
    }
    pub async fn list_plans_for_test(
        &self,
        project: &ProjectId,
        archived: bool,
    ) -> Result<Vec<PlanListing>, StorageError> {
        self.store.list_plans(project, archived).await
    }
    pub async fn thread_plan_for_test(
        &self,
        thread: &ThreadId,
    ) -> Result<Option<WorkflowId>, StorageError> {
        self.store.thread_plan(thread).await
    }
    pub async fn show_plan_for_test(
        &self,
        ctx: &CommandContext,
        writer: &Writer,
        turn: &OperationId,
        workflow: &WorkflowId,
        task_number: Option<u32>,
        place: Place,
    ) -> Result<PlanShown, crate::CoreError> {
        self.store
            .show_plan(ctx, writer, turn, workflow, task_number, place)
            .await
    }
}

impl Plans {
    pub async fn edit_design_for_test(
        &self,
        ctx: &CommandContext,
        project: &ProjectId,
        revision: i64,
        ops: Vec<super::DesignOp>,
    ) -> Result<crate::design::DesignChange, crate::CoreError> {
        self.edit_workspace_with(
            ctx.clone(),
            project,
            revision,
            ops.into_iter().map(Into::into).collect(),
        )
        .await
    }
}
