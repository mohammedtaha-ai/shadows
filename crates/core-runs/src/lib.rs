//! One job: executor attempts at the tasks of an approved plan — their start,
//! stop, commit, release of scope and recovery. The `Runs` service.
//!
//! An attempt is one executor session on one task. It holds its scope, the
//! `writes` it may change, from Start until its turn has ended and its session
//! is closed; Start refuses a task whose `writes` may meet a held scope.
//! `model` holds the types, `store` the SQL, `scope` the pattern matcher,
//! `ready` the rule for links, `git` the git calls, `start`, `stop`, `commit`,
//! `watch` and `recover` the transitions, `diff` the reading of a commit.

mod commit;
mod diff;
mod error;
mod git;
mod model;
mod ready;
mod recover;
mod scope;
mod start;
mod stop;
mod store;
mod watch;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

pub use error::RunsError;
pub use model::{
    Attempt, AttemptId, AttemptState, CommitResult, FileDiff, Need, RecoverReport, StartAnswer,
    TaskRun,
};

use core_db::{CommandContext, Storage};
use core_harness::Harness;
use core_plans::{Plans, PlansError, WorkflowId};
use core_projects::Projects;
use core_turns::Turns;
use store::RunsStore;

/// What every executor is first asked to do.
const FIRST_PROMPT: &str = "Do task T{n}. Read it with task_context first.";

struct Inner {
    store: RunsStore,
    plans: Plans,
    harness: Arc<Harness>,
    turns: Turns,
    projects: Projects,
    /// One commit at a time per project folder, by project id.
    commits: Mutex<HashMap<String, Arc<tokio::sync::Mutex<()>>>>,
    /// Replaces the first prompt, so a test can script the fake adapter.
    prompt: Mutex<Option<String>>,
    #[cfg(feature = "test-support")]
    hooks: Mutex<Hooks>,
}

/// A test's step, given the project folder.
#[cfg(feature = "test-support")]
pub type FolderHook = Arc<dyn Fn(&std::path::Path) + Send + Sync>;

/// What a test makes happen inside a commit.
#[cfg(feature = "test-support")]
#[derive(Clone, Default)]
pub struct Hooks {
    /// Runs in the project folder just before HEAD is moved.
    pub before_update_ref: Option<FolderHook>,
    /// The SQLite write after git committed fails.
    pub fail_record: bool,
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
                commits: Mutex::new(HashMap::new()),
                prompt: Mutex::new(None),
                #[cfg(feature = "test-support")]
                hooks: Mutex::new(Hooks::default()),
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

    /// The model the plan's Planner conversation last asked for: its newest
    /// turn that recorded one. None for a plan an external agent wrote.
    async fn suggested_model(&self, plan: &core_plans::Plan) -> Option<String> {
        let core_plans::WrittenBy::Planner { thread_id, .. } = &plan.written_by else {
            return None;
        };
        match self.inner.turns.operations(thread_id).await {
            Ok(operations) => operations
                .into_iter()
                .filter_map(|op| op.invocation.map(|i| (op.created_at, i.requested_model)))
                .max_by(|a, b| a.0.cmp(&b.0))
                .map(|(_, model)| model),
            Err(error) => {
                tracing::warn!(%thread_id, %error, "runs.suggested_model_unread");
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

    /// Scripts the first prompt of later attempts.
    #[cfg(feature = "test-support")]
    pub fn set_first_prompt_for_tests(&self, prompt: &str) {
        *self
            .inner
            .prompt
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(prompt.to_string());
    }

    /// Sets what later commits make happen.
    #[cfg(feature = "test-support")]
    pub fn set_hooks_for_tests(&self, hooks: Hooks) {
        *self
            .inner
            .hooks
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = hooks;
    }

    #[cfg(feature = "test-support")]
    fn hooks(&self) -> Hooks {
        self.inner
            .hooks
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
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
