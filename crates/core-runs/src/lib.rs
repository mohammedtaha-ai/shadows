//! One job: executor attempts at the tasks of an approved plan — their start,
//! stop, reported commit, release of scope and recovery. The `Runs` service.
//!
//! An attempt is one executor session on one task. It holds its scope, the
//! `writes` it may change, from Start until its task is judged (marked done)
//! or it is stopped, and for an attempt without a commit until its turn has
//! ended and its session is closed; Start refuses a task whose `writes` may meet a held scope.
//! `model` holds the types, `store` the SQL, `scope` the pattern matcher,
//! `ready` the rule for links, `ready_group` the tasks the Planner may start
//! together, `git` the git calls, `init` a folder's first
//! repository, `planner` the Planner's calls on its tasks, `notify` the
//! Planner's notice of an ended attempt, `amend` the Planner's addition,
//! `lessons` the record a task leaves when it stops on the cap, `brief` the
//! context each attempt was given, `question` an executor's question about
//! its own attempt, `conflict` an executor's contract conflict, `start`,
//! `stop`, `report`, `watch` and `recover` the transitions, `diff` the
//! reading of a commit.

mod amend;
mod brief;
mod checks;
mod conflict;
mod diff;
mod error;
mod executor_choice;
mod git;
mod init;
mod lessons;
mod message;
mod model;
mod notify;
mod planner;
mod question;
mod ready;
mod ready_group;
mod recheck;
mod recover;
mod report;
mod scope;
mod start;
mod stop;
mod store;
mod watch;

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

pub use brief::AttemptBrief;
pub use error::RunsError;
pub use model::{
    AgentMessage, AgentMessageId, AgentMessageState, AgentReplyNotice, Attempt, AttemptFile,
    AttemptId, AttemptNotice, AttemptOrigin, AttemptState, CheckOutcome, CheckResult,
    ConflictDelivery, ConflictEvidence, ConflictInput, DoneMark, FileDiff, LessonAttempt,
    LessonCommand, LiveAttempt, Need, ReadyGroup, ReadyTask, RecheckFailure, RecheckNotice,
    RecoverReport, ReportedCheck, ReportedOutcome, StartAnswer, TaskConflict, TaskLesson, TaskRun,
};
pub use model::{Chooser, ExecutorChoice, ExecutorInput};
pub use question::{PlannedScope, ProducerStatus, TaskAnswer, TaskQuestion};

use codebase::Codebase;
use core_db::{CommandContext, Storage};
use core_harness::Harness;
use core_plans::{Plans, PlansError, WorkflowId};
use core_projects::{ProjectId, Projects};
use core_threads::{ThreadId, Threads};
use core_turns::Turns;
use store::RunsStore;

/// What every executor is first asked to do. It names only the task number:
/// `task_brief` answers the context saved for the executor's own attempt. It
/// says how the work ends, because the executor makes its own commit.
const FIRST_PROMPT: &str = "Do task T{n}. Read it first: call task_brief, which answers the task \
    and the context saved for this attempt. {commit} If nothing needs to change, commit nothing \
    and end with a last message saying why.";

/// How an executor commits in a folder other executors may share, as its
/// first turn and every addition say it: Git's own `--only` commit of its
/// exact paths, which leaves what others staged in the index and out of the
/// commit. The report's scope check stays the final guard.
const COMMIT: &str = "Other executors may work in the same folder at once. Run only the checks \
    your task says you may run alongside them; a shared build, test or database it does not \
    give you is reported as not run, never as passed. When the work is done, commit only your \
    own files inside the task's writes (never a binary or an installer): first stage each new \
    file by its exact path with `git --literal-pathspecs add -- <path>`, then run \
    `git --literal-pathspecs commit --only -m \"<message>\" -- <path>...` naming every path you \
    changed exactly, both names of a rename and the name of a deleted file; never a pattern, a \
    folder or `.`. Leave what others staged alone. If git refuses because an index or ref lock \
    exists, run the same command again after 1, 2 and 4 seconds; never delete the lock or reset \
    the index. If it is still refused, report that the commit failed. Leave nothing uncommitted \
    inside the task's writes; then call task_report with the commit's sha and an honest report \
    of what you did, ran and found.";

struct Inner {
    /// One lock per executor thread: a question's send, a release and a
    /// return's start never interleave on it.
    messaging: tokio::sync::Mutex<std::collections::HashMap<ThreadId, Arc<tokio::sync::Mutex<()>>>>,
    /// One lock per project: Shadows runs one check at a time there.
    checking: tokio::sync::Mutex<std::collections::HashMap<ProjectId, Arc<tokio::sync::Mutex<()>>>>,
    store: RunsStore,
    plans: Plans,
    harness: Arc<Harness>,
    turns: Turns,
    projects: Projects,
    /// Read for the commands an executor ran, in a lessons record.
    threads: Arc<Threads>,
    /// Lists a project's files, so a first commit leaves out secrets and
    /// dependencies.
    code: Codebase,
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
        threads: Arc<Threads>,
        code: Codebase,
    ) -> Self {
        Self {
            inner: Arc::new(Inner {
                messaging: tokio::sync::Mutex::new(std::collections::HashMap::new()),
                checking: tokio::sync::Mutex::new(std::collections::HashMap::new()),
                store: RunsStore::with_storage(storage),
                plans,
                harness,
                turns,
                projects,
                threads,
                code,
                prompt: Mutex::new(None),
            }),
        }
    }

    /// The task as its page reads it: whether it was marked done, its latest
    /// attempt, the tasks it needs, Start's answer and its saved executor choice.
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
            done_reason: mark.as_ref().and_then(|m| m.2.clone()),
            done_at: mark.and_then(|m| m.1),
            latest,
            needs: self.needs(&plan, task).await?,
            start: self.start_answer(&plan, task).await?,
            executor: self.inner.store.executor_choice(workflow, task).await?,
        })
    }

    /// The Planner invocation that determines where notices are delivered.
    async fn last_invocation(&self, thread: &ThreadId) -> Option<core_turns::InvocationView> {
        match self.inner.turns.operations(thread).await {
            Ok(operations) => operations
                .into_iter()
                .filter_map(|op| op.invocation.map(|i| (op.created_at, i)))
                .max_by(|a, b| a.0.cmp(&b.0))
                .map(|(_, invocation)| invocation),
            Err(error) => {
                tracing::warn!(%thread, %error, "runs.planner_unread");
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
        reason: Option<String>,
    ) -> Result<TaskRun, RunsError> {
        let plan = self.plan(workflow, version).await?;
        if !plan.tasks.iter().any(|t| t.content.number == task) {
            return Err(RunsError::NotFound);
        }
        let reason = reason
            .map(|r| r.trim().to_string())
            .filter(|r| !r.is_empty());
        // A mark without a reason fingerprints with no `reason` key, as marks
        // recorded before reasons existed did, so their replays still answer.
        let mut input = serde_json::json!({
            "workflow": workflow, "version": version, "task": task, "done": done,
        });
        if let Some(reason) = &reason {
            input["reason"] = serde_json::json!(reason);
        }
        let command = CommandContext::user(command, "run.mark_done", input);
        let marked = self
            .inner
            .store
            .put_mark(command, plan, task, done, reason)
            .await?;
        // Only the attempt the mark judged, and only while that mark stands:
        // a replay never frees work admitted after it.
        if let Some((attempt, thread)) = marked.judged {
            self.release_after_mark(&attempt, &thread).await;
        }
        if let Some(batch) = marked.recheck {
            let runs = self.clone();
            tokio::spawn(async move {
                runs.recheck_done(batch).await;
            });
        }
        self.task(workflow, version, task).await
    }

    /// The plan version `workflow`.
    async fn read_plan(&self, workflow: &WorkflowId) -> Result<core_plans::Plan, RunsError> {
        self.inner
            .plans
            .get(workflow)
            .await
            .map_err(|error| match error {
                PlansError::Storage(core_db::StorageError::NotFound(_)) => RunsError::NotFound,
                other => RunsError::Unavailable(other.to_string()),
            })
    }

    /// The plan version `workflow`, which must be version `version`.
    async fn plan(
        &self,
        workflow: &WorkflowId,
        version: i64,
    ) -> Result<core_plans::Plan, RunsError> {
        let plan = self.read_plan(workflow).await?;
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

    fn first_prompt(&self, task: u32, left: &[String], left_by: Option<u32>) -> String {
        self.inner
            .prompt
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
            .unwrap_or_else(|| {
                let prompt = FIRST_PROMPT
                    .replace("{n}", &task.to_string())
                    .replace("{commit}", COMMIT);
                match left_by.filter(|_| !left.is_empty()) {
                    Some(number) => format!(
                        "{prompt}\nAttempt {number} of this task ended without a commit and left \
                         these files uncommitted inside your writes: {}. Read them first; keep \
                         what is right, fix or remove the rest, then commit only this task's work.",
                        left.join(", ")
                    ),
                    None => prompt,
                }
            })
    }
}

#[cfg(test)]
mod tests {
    use super::{COMMIT, FIRST_PROMPT};

    #[test]
    fn the_first_prompt_says_how_an_attempt_ends() {
        let prompt = FIRST_PROMPT.replace("{commit}", COMMIT);
        for text in [
            "Leave nothing uncommitted inside the task's writes",
            "If nothing needs to change, commit nothing",
            "git --literal-pathspecs commit --only -m",
            "both names of a rename and the name of a deleted file",
            "never delete the lock",
            "reported as not run, never as passed",
            "call task_brief",
        ] {
            assert!(prompt.contains(text), "{text}");
        }
    }
}
