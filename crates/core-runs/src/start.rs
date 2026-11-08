//! One job: starting an attempt — checks, the record, the session and its
//! first turn.
use std::path::Path;

use crate::model::AttemptState;
use crate::scope::Scope;
use crate::store::NewAttempt;
use crate::{Attempt, Runs, RunsError};
use core_db::CommandContext;
use core_grants::TaskScope;
use core_plans::{PlanState, WorkflowId, WorkflowState};
use core_turns::SendTurn;

/// The permission mode in which a request outside the task's rules is refused.
const MODE: &str = shadows_agent::policy::EXECUTOR_MODE;

impl Runs {
    /// Starts an attempt at `task` of a frozen version, or answers the one a
    /// replay of `command` started. An attempt whose session or turn could not
    /// start is answered `Failed`, with its reason.
    pub async fn start(
        &self,
        command: String,
        workflow: &WorkflowId,
        version: i64,
        task: u32,
        model: String,
    ) -> Result<Attempt, RunsError> {
        let plan = self.plan(workflow, version).await?;
        if plan.plan_state == PlanState::Archived {
            return Err(RunsError::Archived);
        }
        if plan.state != WorkflowState::Frozen {
            return Err(RunsError::NotFrozen);
        }
        if plan.next.is_some() {
            return Err(RunsError::Superseded);
        }
        let content = plan
            .tasks
            .iter()
            .find(|t| t.content.number == task)
            .ok_or(RunsError::NotFound)?
            .content
            .clone();
        Scope::new(&content.writes).map_err(RunsError::BadScope)?;
        let waiting = self.waiting_on(&plan, task).await?;
        if !waiting.is_empty() {
            return Err(RunsError::Waiting { tasks: waiting });
        }
        let fingerprint = CommandContext::fingerprint(
            "run.start",
            &serde_json::json!({
                "workflow": workflow, "version": version, "task": task, "model": model,
            }),
        );
        let begun = self
            .inner
            .store
            .begin_attempt(NewAttempt {
                command_id: command,
                fingerprint,
                project: plan.project_id.clone(),
                workflow: workflow.clone(),
                version,
                task,
                model: model.clone(),
                writes: content.writes.clone(),
            })
            .await?;
        if begun.replayed {
            return Ok(begun.attempt);
        }
        let attempt = begun.attempt;
        let root = match self.inner.projects.get(&plan.project_id).await {
            Ok(project) => project.directory,
            Err(error) => return self.abandon(attempt, error.to_string()).await,
        };
        let Some(root) = root else {
            return self
                .abandon(attempt, "the project has no directory".into())
                .await;
        };
        let scope = TaskScope {
            workflow: workflow.as_str().to_string(),
            version,
            task,
        };
        if let Err(error) = self
            .inner
            .harness
            .open_executor(
                &attempt.thread_id,
                Path::new(&root),
                &content.writes,
                scope,
                model.clone(),
            )
            .await
        {
            return self
                .abandon(attempt, format!("session did not open: {error}"))
                .await;
        }
        let sent = self
            .inner
            .turns
            .send_with_default_effort(
                attempt.thread_id.clone(),
                SendTurn {
                    command_id: format!("run-{}", attempt.id),
                    prompt: self.first_prompt(task),
                    attachments: vec![],
                    model,
                    mode: MODE.into(),
                    effort: None,
                    focus: None,
                    plan: None,
                    client_tab: None,
                },
            )
            .await;
        let operation = match sent {
            Ok(operation) => operation,
            Err(error) => {
                return self
                    .abandon(attempt, format!("the first turn did not start: {error}"))
                    .await;
            }
        };
        self.inner
            .store
            .set_operation(&attempt.id, &operation)
            .await?;
        self.watch(attempt.id.clone(), attempt.thread_id.clone(), operation);
        self.inner.store.get(&attempt.id).await
    }

    /// The attempt cannot go on: it fails with `reason` and frees its scope.
    async fn abandon(&self, attempt: Attempt, reason: String) -> Result<Attempt, RunsError> {
        self.inner
            .store
            .finish(&attempt.id, AttemptState::Failed, Some(reason))
            .await?;
        self.release(&attempt.id, &attempt.thread_id).await;
        self.inner.store.get(&attempt.id).await
    }
}
