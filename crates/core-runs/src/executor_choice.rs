//! Select the executor for the newest version's task.
use core_db::CommandContext;
use core_grants::Grant;
use core_plans::WorkflowId;

use crate::{ExecutorChoice, ExecutorInput, Runs, RunsError};

impl Runs {
    #[cfg(feature = "test-support")]
    pub fn pause_next_save_for_test(
        &self,
    ) -> (
        tokio::sync::oneshot::Receiver<()>,
        tokio::sync::oneshot::Sender<()>,
    ) {
        self.inner.store.pause_next_save()
    }

    #[cfg(feature = "test-support")]
    pub async fn choose_executor_for_test(
        &self,
        workflow: &WorkflowId,
        task: u32,
        harness: &str,
        model: &str,
        effort: Option<&str>,
    ) -> Result<(), RunsError> {
        self.inner
            .store
            .choose_executor_for_test(
                workflow,
                task,
                ExecutorInput {
                    harness: harness.into(),
                    model: model.into(),
                    effort: effort.map(str::to_owned),
                },
            )
            .await
    }

    pub async fn set_executor(
        &self,
        command_id: String,
        workflow: &WorkflowId,
        version: i64,
        task: u32,
        input: ExecutorInput,
    ) -> Result<ExecutorChoice, RunsError> {
        self.plan(workflow, version).await?;
        self.save_executor(command_id, workflow, task, input, None)
            .await
    }

    pub(crate) async fn save_executor(
        &self,
        command_id: String,
        workflow: &WorkflowId,
        task: u32,
        input: ExecutorInput,
        grant: Option<&Grant>,
    ) -> Result<ExecutorChoice, RunsError> {
        let kind = "run.executor_set";
        let params = serde_json::json!({
            "workflow": workflow, "task": task, "harness": input.harness,
            "model": input.model, "effort": input.effort,
        });
        let mut command = CommandContext::user(command_id, kind, params);
        if let Some(grant) = grant {
            let thread = grant.thread_id.as_ref().ok_or(RunsError::NotFound)?;
            command.principal_kind = "Thread".into();
            command.principal_id = thread.as_str().into();
        }
        let store = &self.inner.store;
        if let Some(saved) = store.replay_executor_save(&command).await? {
            return Ok(saved);
        }
        let target = store.executor_target(workflow, task).await?;
        if !shadows_agent::policy::EXECUTOR_HARNESSES.contains(&input.harness.as_str()) {
            return Err(RunsError::ExecutorNotOffered(format!(
                "harness: {}",
                input.harness
            )));
        }
        let catalog = self
            .inner
            .harness
            .catalog(&input.harness)
            .await
            .map_err(|error| RunsError::CatalogUnavailable(error.to_string()))?;
        let model = catalog
            .models
            .iter()
            .find(|model| model.id == input.model)
            .ok_or_else(|| RunsError::ExecutorNotOffered(format!("model: {}", input.model)))?;
        match &input.effort {
            None if !model.efforts.is_empty() => {
                return Err(RunsError::ExecutorNotOffered("effort: required".into()));
            }
            Some(effort)
                if !model
                    .efforts
                    .iter()
                    .any(|item| item.id == *effort && item.enabled) =>
            {
                return Err(RunsError::ExecutorNotOffered(format!("effort: {effort}")));
            }
            _ => {}
        }
        store
            .record_executor_save(
                command,
                workflow.clone(),
                task,
                target,
                input,
                grant.cloned(),
            )
            .await
    }
}
