//! Persist task choices and immutable command outcomes.
use core_db::{Actor, CommandContext, JournalEntry, Storage, StorageError, Tx};
use core_grants::{Grant, Grants};
use core_plans::{Plan, PlanState, Plans, TaskId, WorkflowId};

use super::RunsStore;
use crate::{Chooser, ExecutorChoice, ExecutorInput, RunsError};

const SCOPE: &str = "run.executor_set";

#[cfg(feature = "test-support")]
impl RunsStore {
    pub fn pause_next_save(
        &self,
    ) -> (
        tokio::sync::oneshot::Receiver<()>,
        tokio::sync::oneshot::Sender<()>,
    ) {
        let (reached, heard) = tokio::sync::oneshot::channel();
        let (go, release) = tokio::sync::oneshot::channel();
        *self.save_gate.lock().unwrap() = Some(super::Gate { reached, release });
        (heard, go)
    }

    pub async fn choose_executor_for_test(
        &self,
        workflow: &WorkflowId,
        task: u32,
        input: ExecutorInput,
    ) -> Result<(), RunsError> {
        let workflow = workflow.clone();
        self.storage
            .write_txn_as(move |tx| {
                Box::pin(async move {
                    let plan = Plans::version_in(tx, &workflow)
                        .await
                        .map_err(RunsError::from_plans)?;
                    let id = plan
                        .tasks
                        .iter()
                        .find(|row| row.content.number == task)
                        .map(|row| row.id.as_str())
                        .ok_or(RunsError::NotFound)?;
                    sqlx::query(
                        "INSERT INTO run_task_executors
                    (task_id, plan_id, harness, model, effort, chosen_by, chosen_at)
                 VALUES (?, ?, ?, ?, ?, 'person', ?)
                 ON CONFLICT(task_id) DO NOTHING",
                    )
                    .bind(id)
                    .bind(plan.plan_id.as_str())
                    .bind(input.harness)
                    .bind(input.model)
                    .bind(input.effort)
                    .bind(Storage::now())
                    .execute(tx)
                    .await?;
                    Ok(())
                })
            })
            .await
    }
}

impl RunsStore {
    pub async fn executor_choice(
        &self,
        workflow: &WorkflowId,
        task: u32,
    ) -> Result<Option<ExecutorChoice>, RunsError> {
        let mut read = self
            .storage
            .reader()
            .begin()
            .await
            .map_err(StorageError::from)?;
        Self::executor_choice_in(&mut read, workflow, task).await
    }

    pub(super) async fn executor_choice_in(
        tx: &mut Tx,
        workflow: &WorkflowId,
        task: u32,
    ) -> Result<Option<ExecutorChoice>, RunsError> {
        let ids = Plans::task_lineage_in(tx, workflow, task)
            .await
            .map_err(RunsError::from_plans)?;
        for id in ids {
            let row = sqlx::query_as::<_, (String, String, Option<String>, String, String)>(
                "SELECT harness, model, effort, chosen_by, chosen_at
                 FROM run_task_executors WHERE task_id = ?",
            )
            .bind(id.as_str())
            .fetch_optional(&mut *tx)
            .await?;
            if let Some((harness, model, effort, by, chosen_at)) = row {
                let chosen_by = if by == "planner" {
                    Chooser::Planner
                } else {
                    Chooser::Person
                };
                return Ok(Some(ExecutorChoice {
                    harness,
                    model,
                    effort,
                    chosen_by,
                    chosen_at,
                }));
            }
        }
        Ok(None)
    }

    pub async fn executor_target(
        &self,
        workflow: &WorkflowId,
        task: u32,
    ) -> Result<TaskId, RunsError> {
        let mut read = self
            .storage
            .reader()
            .begin()
            .await
            .map_err(StorageError::from)?;
        target_in(&mut read, workflow, task).await.map(|(_, id)| id)
    }

    pub async fn replay_executor_save(
        &self,
        command: &CommandContext,
    ) -> Result<Option<ExecutorChoice>, RunsError> {
        let mut read = self
            .storage
            .reader()
            .begin()
            .await
            .map_err(StorageError::from)?;
        replay_in(&mut read, command).await
    }

    pub async fn record_executor_save(
        &self,
        command: CommandContext,
        workflow: WorkflowId,
        task: u32,
        target: TaskId,
        input: ExecutorInput,
        grant: Option<Grant>,
    ) -> Result<ExecutorChoice, RunsError> {
        #[cfg(feature = "test-support")]
        {
            let gate = self.save_gate.lock().unwrap().take();
            if let Some(gate) = gate {
                let _ = gate.reached.send(());
                let _ = gate.release.await;
            }
        }
        self.storage
            .write_txn_as(move |tx| {
                Box::pin(async move {
                    if let Some(saved) = replay_in(tx, &command).await? {
                        return Ok(saved);
                    }
                    let (plan, current) = target_in(tx, &workflow, task).await?;
                    if target != current {
                        return Err(RunsError::NotFound);
                    }
                    let (chosen_by, by, actor) = if let Some(grant) = &grant {
                        Grants::check_live_in(tx, grant).await?;
                        if grant.project_id != plan.project_id {
                            return Err(RunsError::NotFound);
                        }
                        let thread = grant.thread_id.as_ref().ok_or(RunsError::NotFound)?;
                        (
                            Chooser::Planner,
                            "planner",
                            Actor {
                                kind: "Thread".into(),
                                id: thread.as_str().into(),
                            },
                        )
                    } else {
                        (Chooser::Person, "person", Actor::user("local"))
                    };
                    let choice = ExecutorChoice {
                        harness: input.harness,
                        model: input.model,
                        effort: input.effort,
                        chosen_by,
                        chosen_at: Storage::now(),
                    };
                    sqlx::query(
                        "INSERT INTO run_task_executors
                   (task_id, plan_id, harness, model, effort, chosen_by, chosen_at)
                 VALUES (?, ?, ?, ?, ?, ?, ?)
                 ON CONFLICT(task_id) DO UPDATE SET harness=excluded.harness,
                   model=excluded.model, effort=excluded.effort, chosen_by=excluded.chosen_by,
                   chosen_at=excluded.chosen_at",
                    )
                    .bind(target.as_str())
                    .bind(plan.plan_id.as_str())
                    .bind(&choice.harness)
                    .bind(&choice.model)
                    .bind(&choice.effort)
                    .bind(by)
                    .bind(&choice.chosen_at)
                    .execute(&mut *tx)
                    .await?;
                    let save_id = crate::AttemptId::generate();
                    sqlx::query(
                        "INSERT INTO run_executor_saves (id, plan_id, task, result_json, saved_at)
                 VALUES (?, ?, ?, ?, ?)",
                    )
                    .bind(save_id.as_str())
                    .bind(plan.plan_id.as_str())
                    .bind(i64::from(task))
                    .bind(serde_json::to_string(&choice)?)
                    .bind(&choice.chosen_at)
                    .execute(&mut *tx)
                    .await?;
                    CommandContext::record_command(
                        tx,
                        &command,
                        SCOPE,
                        "",
                        "ExecutorChoice",
                        save_id.as_str(),
                        &choice.chosen_at,
                    )
                    .await?;
                    let event = JournalEntry::new("TaskExecutorChosen", actor)
                        .with_project(plan.project_id.as_str())
                        .with_payload(serde_json::json!({
                            "project_id": plan.project_id, "plan_id": plan.plan_id, "task": task,
                        }));
                    JournalEntry::append_event(tx, event, &choice.chosen_at).await?;
                    Ok(choice)
                })
            })
            .await
    }
}

async fn target_in(
    tx: &mut Tx,
    workflow: &WorkflowId,
    task: u32,
) -> Result<(Plan, TaskId), RunsError> {
    let plan = Plans::version_in(tx, workflow)
        .await
        .map_err(RunsError::from_plans)?;
    if plan.plan_state == PlanState::Archived {
        return Err(RunsError::Archived);
    }
    if plan.next.is_some() {
        return Err(RunsError::Superseded);
    }
    let id = plan
        .tasks
        .iter()
        .find(|row| row.content.number == task)
        .map(|row| row.id.clone())
        .ok_or(RunsError::NotFound)?;
    Ok((plan, id))
}

async fn replay_in(
    tx: &mut Tx,
    command: &CommandContext,
) -> Result<Option<ExecutorChoice>, RunsError> {
    let save = CommandContext::classify(tx, command, SCOPE, "")
        .await
        .map_err(|err| match err {
            StorageError::CommandConflict => RunsError::CommandConflict,
            other => other.into(),
        })?;
    let Some(save) = save else { return Ok(None) };
    let json: String =
        sqlx::query_scalar("SELECT result_json FROM run_executor_saves WHERE id = ?")
            .bind(save)
            .fetch_one(tx)
            .await?;
    Ok(Some(serde_json::from_str(&json)?))
}
