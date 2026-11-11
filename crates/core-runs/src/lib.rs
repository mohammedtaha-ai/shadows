//! One job: executor attempts at the tasks of an approved plan — their start,
//! stop, reported commit, release of scope and recovery. The `Runs` service.
//!
//! An attempt is one executor session on one task. It holds its scope, the
//! `writes` it may change, from Start until its turn has ended and its session
//! is closed; Start refuses a task whose `writes` may meet a held scope.
//! `model` holds the types, `store` the SQL, `scope` the pattern matcher,
//! `ready` the rule for links, `git` the git calls, `init` a folder's first
//! repository, `start`, `stop`, `report`,
//! `watch` and `recover` the transitions, `diff` the reading of a commit.

mod diff;
mod error;
mod git;
mod init;
mod model;
mod ready;
mod recover;
mod report;
mod scope;
mod start;
mod stop;
mod store;
mod watch;

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

pub use error::RunsError;
pub use model::{
    Attempt, AttemptId, AttemptState, FileDiff, Need, RecoverReport, StartAnswer, TaskRun,
};

use core_db::{CommandContext, Storage};
use core_harness::Harness;
use core_plans::{Plans, PlansError, WorkflowId};
use core_projects::{ProjectId, Projects};
use core_turns::Turns;
use store::RunsStore;

/// What every executor is first asked to do. It names only the task number:
/// `task_context` reads the plan version from the executor's own grant. It
/// says how the work ends, because the executor makes its own commit.
const FIRST_PROMPT: &str = "Do task T{n}. Read it first: call task_context with task_number {n} \
    and no workflow_id. When the work is done, stage only your own files inside the task's \
    writes (never a binary or an installer) and run git commit yourself, then call task_report \
    with the commit's sha and an honest report of what you did, ran and found.";

/// The harness an executor thread runs on, so the only one whose models an
/// executor can be offered.
const EXECUTOR_HARNESS: &str = shadows_agent::policy::CLAUDE_CODE;

struct Inner {
    store: RunsStore,
    plans: Plans,
    harness: Arc<Harness>,
    turns: Turns,
    projects: Projects,
    /// Replaces the first prompt, so a test can script the fake adapter.
    prompt: Mutex<Option<String>>,
}

/// Attempts at tasks. Cheap to clone: the clones share one state.
#[derive(Clone)]
pub struct Runs {
    inner: Arc<Inner>,
}

impl Runs {
    pub fn new(
        storage: Arc<Storage>,
        plans: Plans,
        harness: Arc<Harness>,
        turns: Turns,
        projects: Projects,
    ) -> Self {
        Self {
            inner: Arc::new(Inner {
                store: RunsStore::with_storage(storage),
                plans,
                harness,
                turns,
                projects,
                prompt: Mutex::new(None),
            }),
        }
    }

    /// The task as its page reads it: whether it was marked done, its latest
    /// attempt, the tasks it needs, Start's answer and the model to offer.
    pub async fn task(
        &self,
        workflow: &WorkflowId,
        version: i64,
        task: u32,
    ) -> Result<TaskRun, RunsError> {
        let plan = self.plan(workflow, version).await?;
        if !plan.tasks.iter().any(|t| t.content.number == task) {
            return Err(RunsError::NotFound);
        }
        let latest = self
            .attempts(workflow, version, task)
            .await?
            .into_iter()
            .next();
        let mark = self.inner.store.mark(workflow, version, task).await?;
        Ok(TaskRun {
            task,
            done: mark.as_ref().is_some_and(|m| m.0),
            done_at: mark.and_then(|m| m.1),
            latest,
            needs: self.needs(&plan, task).await?,
            start: self.start_answer(&plan, task).await?,
            suggested_model: self.suggested_model(&plan).await,
        })
    }

    /// The model to offer the executor: the one the plan's Planner
    /// conversation last asked for, when that turn ran on the executor's
    /// harness. A model of another harness (a Codex Planner's) is not one the
    /// executor is offered, so then the model last used on the executor's
    /// harness is suggested, or none. None for a plan an external agent wrote.
    async fn suggested_model(&self, plan: &core_plans::Plan) -> Option<String> {
        let core_plans::WrittenBy::Planner { thread_id, .. } = &plan.written_by else {
            return None;
        };
        let last = match self.inner.turns.operations(thread_id).await {
            Ok(operations) => operations
                .into_iter()
                .filter_map(|op| op.invocation.map(|i| (op.created_at, i)))
                .max_by(|a, b| a.0.cmp(&b.0))
                .map(|(_, invocation)| invocation),
            Err(error) => {
                tracing::warn!(%thread_id, %error, "runs.suggested_model_unread");
                None
            }
        };
        match last {
            Some(invocation) if invocation.harness_kind == EXECUTOR_HARNESS => {
                Some(invocation.requested_model)
            }
            _ => self.executor_harness_model().await,
        }
    }

    /// The model last used on the executor's harness, which it offered then.
    async fn executor_harness_model(&self) -> Option<String> {
        match self.inner.harness.remembered_model(EXECUTOR_HARNESS).await {
            Ok(model) => model,
            Err(error) => {
                tracing::warn!(%error, "runs.executor_model_unread");
                None
            }
        }
    }

    /// A task's attempts, newest first.
    pub async fn attempts(
        &self,
        workflow: &WorkflowId,
        version: i64,
        task: u32,
    ) -> Result<Vec<Attempt>, RunsError> {
        self.inner.store.attempts(workflow, version, task).await
    }

    /// One attempt as it stands.
    pub async fn attempt(&self, id: &AttemptId) -> Result<Attempt, RunsError> {
        self.inner.store.get(id).await
    }

    /// A person's mark that the task is done (or not), which `needs` links
    /// wait for.
    pub async fn mark_done(
        &self,
        command: String,
        workflow: &WorkflowId,
        version: i64,
        task: u32,
        done: bool,
    ) -> Result<TaskRun, RunsError> {
        let plan = self.plan(workflow, version).await?;
        if !plan.tasks.iter().any(|t| t.content.number == task) {
            return Err(RunsError::NotFound);
        }
        let command = CommandContext::user(
            command,
            "run.mark_done",
            serde_json::json!({
                "workflow": workflow, "version": version, "task": task, "done": done,
            }),
        );
        self.inner
            .store
            .put_mark(command, (workflow, version, task), done)
            .await?;
        self.task(workflow, version, task).await
    }

    /// The plan version `workflow`, which must be version `version`.
    async fn plan(
        &self,
        workflow: &WorkflowId,
        version: i64,
    ) -> Result<core_plans::Plan, RunsError> {
        let plan = self
            .inner
            .plans
            .get(workflow)
            .await
            .map_err(|error| match error {
                PlansError::Storage(core_db::StorageError::NotFound(_)) => RunsError::NotFound,
                other => RunsError::Unavailable(other.to_string()),
            })?;
        if plan.version != version {
            return Err(RunsError::NotFound);
        }
        Ok(plan)
    }

    /// The folder `project` works in.
    pub(crate) async fn folder(&self, project: &ProjectId) -> Result<PathBuf, RunsError> {
        let project = self
            .inner
            .projects
            .get(project)
            .await
            .map_err(|error| RunsError::Unavailable(error.to_string()))?;
        project
            .directory
            .map(PathBuf::from)
            .ok_or(RunsError::NotGit)
    }

    /// Scripts the first prompt of later attempts.
    #[cfg(feature = "test-support")]
    pub fn set_first_prompt_for_tests(&self, prompt: &str) {
        *self
            .inner
            .prompt
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(prompt.to_string());
    }

    fn first_prompt(&self, task: u32) -> String {
        self.inner
            .prompt
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
            .unwrap_or_else(|| FIRST_PROMPT.replace("{n}", &task.to_string()))
    }
}
