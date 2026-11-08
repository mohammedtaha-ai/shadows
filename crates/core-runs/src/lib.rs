//! One job: executor attempts at the tasks of an approved plan — their start,
//! stop, release of scope and recovery. The `Runs` service.
//!
//! An attempt is one executor session on one task. It holds its scope, the
//! `writes` it may change, from Start until its turn has ended and its session
//! is closed; Start refuses a task whose `writes` may meet a held scope.
//! `model` holds the types, `store` the SQL, `scope` the pattern matcher,
//! `ready` the rule for links, `start`, `stop`, `watch` and `recover` the
//! transitions.

mod error;
mod model;
mod ready;
mod recover;
mod scope;
mod start;
mod stop;
mod store;
mod watch;

use std::sync::{Arc, Mutex};

pub use error::RunsError;
pub use model::{Attempt, AttemptId, AttemptState, RecoverReport, TaskRun};
pub use scope::Scope;

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
    projects: Projects,
    harness: Arc<Harness>,
    turns: Turns,
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
                projects,
                harness,
                turns,
                prompt: Mutex::new(None),
            }),
        }
    }

    /// The task, whether it was marked done, and its latest attempt.
    pub async fn task(
        &self,
        workflow: &WorkflowId,
        version: i64,
        task: u32,
    ) -> Result<TaskRun, RunsError> {
        self.inner.store.task_run(workflow, version, task).await
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
        let fingerprint = CommandContext::fingerprint(
            "run.mark_done",
            &serde_json::json!({
                "workflow": workflow, "version": version, "task": task, "done": done,
            }),
        );
        self.inner
            .store
            .put_mark(command, fingerprint, (workflow, version, task), done)
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

    fn first_prompt(&self, task: u32) -> String {
        self.inner
            .prompt
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
            .unwrap_or_else(|| FIRST_PROMPT.replace("{n}", &task.to_string()))
    }
}
