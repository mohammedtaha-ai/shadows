//! One job: starting an attempt — checks, the record, the session and its
//! first turn.
use crate::git::Git;
use crate::model::AttemptState;
use crate::scope::Scope;
use crate::store::NewAttempt;
use crate::{Attempt, Runs, RunsError};
use codebase::Asker;
use core_db::CommandContext;
use core_grants::TaskScope;
use core_plans::{FileAction, Plan, TaskContent, WorkflowId};
use core_turns::SendTurn;

/// The mode an executor's turns run in; its project's modes do not apply.
const MODE: &str = shadows_agent::policy::EXECUTOR_MODE;

impl Runs {
    /// Starts an attempt at `task` of a frozen version, or answers the one a
    /// replay of `command` started, whatever the plan has become since. An
    /// attempt whose session or turn could not start is recorded, so it is
    /// answered `Failed` with its reason, not refused. The attempt's write
    /// saves the task context it receives, its brief; a replay saves none.
    pub async fn start(
        &self,
        command: String,
        workflow: &WorkflowId,
        version: i64,
        task: u32,
        model: String,
    ) -> Result<Attempt, RunsError> {
        let fingerprint = CommandContext::fingerprint(
            "run.start",
            &serde_json::json!({
                "workflow": workflow, "version": version, "task": task, "model": model,
            }),
        );
        if let Some(first) = self.inner.store.replay(&command, &fingerprint).await? {
            return Ok(first);
        }
        let (plan, content) = self.checks(workflow, version, task).await?;
        // A reported commit already reachable from here is not this attempt's work.
        let start_head = Git::at(&self.folder(&plan.project_id).await?).head().await;
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
                start_head,
            })
            .await?;
        if begun.replayed {
            return Ok(begun.attempt);
        }
        self.run_turn(begun.attempt, self.first_prompt(task)).await
    }

    /// Opens the attempt's executor session, live, closed or gone after a
    /// restart, sends it `prompt` as one turn and watches the turn. Start and
    /// an addition both end here: one way to run an attempt's turn.
    pub(crate) async fn run_turn(
        &self,
        attempt: Attempt,
        prompt: String,
    ) -> Result<Attempt, RunsError> {
        let model = attempt.model.clone();
        let scope = TaskScope {
            workflow: attempt.workflow_id.as_str().to_string(),
            version: attempt.version,
            task: attempt.task,
        };
        if let Err(error) = self
            .inner
            .harness
            .open_executor(&attempt.thread_id, scope, model.clone())
            .await
        {
            return self
                .abandon(attempt, format!("session did not open: {error}"))
                .await;
        }
        // A Stop while the session opened found no turn to stop: send none,
        // and close the session it may have closed before it existed.
        let now = self.inner.store.get(&attempt.id).await?;
        if now.state != AttemptState::Running {
            self.release(&attempt.id, &attempt.thread_id).await;
            return Ok(now);
        }
        let sent = self
            .inner
            .turns
            .send_with_default_effort(
                attempt.thread_id.clone(),
                SendTurn {
                    command_id: format!("run-{}", attempt.id),
                    prompt,
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
        // A Stop between that check and this record saw no turn: stop it here.
        if self.inner.store.get(&attempt.id).await?.state != AttemptState::Running
            && let Err(error) = self.inner.turns.stop(&operation).await
        {
            tracing::warn!(attempt = %attempt.id, %error, "runs.late_stop_failed");
        }
        let hold = self.inner.code.hold(&attempt.project_id).await;
        self.watch(
            attempt.id.clone(),
            attempt.thread_id.clone(),
            operation,
            hold,
        );
        self.inner.store.get(&attempt.id).await
    }

    /// What Start checks before it writes anything, in order: admission's
    /// durable checks in one snapshot (the version, the task and its patterns,
    /// its `needs` marks), then the folder: a git repository with nothing
    /// uncommitted inside the task's `writes`, holding every file the task
    /// requires. Changes elsewhere in the folder are other tasks' business.
    /// The folder is observed now and not locked; the attempt's write checks
    /// the durable part again.
    pub(crate) async fn checks(
        &self,
        workflow: &WorkflowId,
        version: i64,
        task: u32,
    ) -> Result<(Plan, TaskContent), RunsError> {
        let (plan, content) = self.inner.store.admission(workflow, version, task).await?;
        let matcher = Scope::new(&content.writes).map_err(RunsError::BadScope)?;
        let git = Git::at(&self.folder(&plan.project_id).await?);
        let prefix = git.prefix().await.ok_or(RunsError::NotGit)?;
        let dirty = self.dirty(&git, &prefix, &matcher).await?;
        if !dirty.is_empty() {
            return Err(RunsError::DirtyScope { paths: dirty });
        }
        let required = Self::required_inputs(&content);
        if !required.is_empty() {
            let missing = self
                .inner
                .code
                .missing_inputs(Asker::Grant(&plan.project_id), &required)
                .await
                .map_err(|error| RunsError::InputUnavailable(error.to_string()))?;
            if !missing.is_empty() {
                return Err(RunsError::MissingInput { paths: missing });
            }
        }
        Ok((plan, content))
    }

    /// The exact files a task's contract says already exist: those it reads,
    /// modifies or deletes, and the existing code it uses. A file it creates
    /// need not exist, and `reads` patterns are where it may look, not files it
    /// requires. A task without a contract requires none.
    fn required_inputs(content: &TaskContent) -> Vec<String> {
        let Some(contract) = &content.contract else {
            return Vec::new();
        };
        let files = contract
            .files
            .iter()
            .filter(|file| file.action != FileAction::Create)
            .map(|file| file.path.clone());
        let existing = contract.uses.iter().filter_map(|used| match &used.source {
            core_plans::ContractSource::Existing { path, .. } => Some(path.clone()),
            core_plans::ContractSource::Task { .. } => None,
        });
        files.chain(existing).collect()
    }

    /// Holds the next Start after its preflight and before its admission
    /// write: the receiver hears it arrive, the sender lets it go on.
    #[cfg(feature = "test-support")]
    pub fn gate_admission_for_tests(
        &self,
    ) -> (
        tokio::sync::oneshot::Receiver<()>,
        tokio::sync::oneshot::Sender<()>,
    ) {
        self.inner.store.gate()
    }

    /// Every uncommitted path inside the folder that `matcher` admits: the
    /// one read Start's and a report's dirty checks both make.
    pub(crate) async fn dirty(
        &self,
        git: &Git,
        prefix: &str,
        matcher: &Scope,
    ) -> Result<Vec<String>, RunsError> {
        Ok(git
            .changed(prefix)
            .await
            .map_err(RunsError::Unavailable)?
            .into_iter()
            .filter(|path| matcher.admits(path))
            .collect())
    }

    /// The attempt cannot go on: it fails with `reason` and frees its scope.
    pub(crate) async fn abandon(
        &self,
        attempt: Attempt,
        reason: String,
    ) -> Result<Attempt, RunsError> {
        self.inner
            .store
            .finish(&attempt.id, AttemptState::Failed, Some(reason))
            .await?;
        self.release(&attempt.id, &attempt.thread_id).await;
        self.inner.store.get(&attempt.id).await
    }
}
