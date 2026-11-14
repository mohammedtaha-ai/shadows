//! One job: answering an executor's question about its own running attempt.
//!
//! The answer joins three owners' facts and keeps them apart: what the
//! attempt's Frozen version promises (Plans), what the code shows now
//! (Codebase's language server) and what the producers' marks and commits are
//! now (Runs). A planned consumer is never presented as a caller in source,
//! and a server that could not answer is a `code_error`, never an empty
//! finding.
use codebase::{Asker, CodebaseError, LspAnswer, LspQuestion};
use core_db::Storage;
use core_grants::{Grant, GrantKind};
use core_plans::{ContextPlan, ContractFacts, ContractSelector, ContractTarget, PlansError};

use crate::model::{Attempt, AttemptId, AttemptState};
use crate::{Runs, RunsError};

/// The most bytes one serialized answer holds.
const MOST_ANSWER: usize = 64 * 1024;
/// The most bytes a `code_error` holds.
const MOST_CODE_ERROR: usize = 2 * 1024;

/// What an executor asks: a part of its own task's contract, optionally one
/// native language-server question, and a page of planned consumers.
#[derive(Debug, Clone, serde::Deserialize, schemars::JsonSchema)]
pub struct TaskQuestion {
    /// The part of the contract asked about; leave it out for the whole task.
    #[serde(default)]
    pub selector: ContractSelector,
    /// A question to the project's language server, answered beside the plan.
    #[serde(default)]
    pub code: Option<LspQuestion>,
    /// How many planned consumers to skip: a page's `next`.
    #[serde(default)]
    pub after: Option<u32>,
    /// How many planned consumers one page holds, 1 to 50.
    #[serde(default = "TaskQuestion::default_limit")]
    pub limit: u16,
}

impl TaskQuestion {
    fn default_limit() -> u16 {
        20
    }
}

/// A producer's current state: whether its task is marked done, and the
/// commit its latest committed attempt reported. Neither proves the code
/// holds the promised symbol.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ProducerStatus {
    pub target: ContractTarget,
    pub done: bool,
    pub latest_commit: Option<String>,
}

/// How far the planning facts reach: only the attempt's own Frozen version.
#[derive(Debug, Clone, Copy, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PlannedScope {
    ThisFrozenVersion,
}

/// A task question's answer: the attempt's version's promises, the current
/// observations, and when they were made.
#[derive(Debug, Clone, serde::Serialize)]
pub struct TaskAnswer {
    pub attempt_id: AttemptId,
    pub plan: ContextPlan,
    pub task: u32,
    pub planning: ContractFacts,
    pub producer_status: Vec<ProducerStatus>,
    pub code: Option<LspAnswer>,
    pub code_error: Option<String>,
    pub observed_at: String,
    pub planned_scope: PlannedScope,
}

impl Runs {
    /// Answers `question` for the running attempt `grant`, an executor's,
    /// belongs to: never an attempt the caller names.
    pub async fn question(
        &self,
        grant: &Grant,
        question: TaskQuestion,
    ) -> Result<TaskAnswer, RunsError> {
        let attempt = self.running_attempt(grant).await?;
        let (context, _) =
            (self.inner.store.context(&attempt.id).await?).ok_or(RunsError::ContextNotRecorded)?;
        let planning = self
            .inner
            .plans
            .contract_facts(
                &attempt.workflow_id,
                attempt.task,
                question.selector,
                question.after,
                question.limit,
            )
            .await
            .map_err(|error| match error {
                PlansError::InvalidCommand(message) => RunsError::InvalidQuestion(message),
                other => RunsError::from_plans(other),
            })?;
        let (code, code_error) = match question.code {
            None => (None, None),
            Some(asked) => self.ask_code(grant, asked).await?,
        };
        // The code work awaited a server: the grant and the attempt may have
        // ended meanwhile.
        self.recheck(grant, &attempt.id).await?;
        let mut producer_status = Vec::new();
        for target in &planning.producers {
            let ContractTarget::Local { task } = target;
            let mark = (self.inner.store)
                .mark(&attempt.workflow_id, attempt.version, *task)
                .await?;
            let latest_commit = (self.inner.store)
                .attempts(&attempt.workflow_id, attempt.version, *task)
                .await?
                .into_iter()
                .find_map(|a| a.commit);
            producer_status.push(ProducerStatus {
                target: target.clone(),
                done: mark.is_some_and(|m| m.0),
                latest_commit,
            });
        }
        let answer = TaskAnswer {
            attempt_id: attempt.id,
            plan: context.plan,
            task: attempt.task,
            planning,
            producer_status,
            code,
            code_error,
            observed_at: Storage::now(),
            planned_scope: PlannedScope::ThisFrozenVersion,
        };
        if serde_json::to_vec(&answer)?.len() > MOST_ANSWER {
            return Err(RunsError::QuestionTooLarge);
        }
        Ok(answer)
    }

    /// The executor's own attempt, which must still be running.
    async fn running_attempt(&self, grant: &Grant) -> Result<Attempt, RunsError> {
        let (Some(thread), Some(scope), GrantKind::Executor) =
            (&grant.thread_id, &grant.scope, grant.kind)
        else {
            return Err(RunsError::NotFound);
        };
        let attempt = self.inner.store.executor_attempt(thread, scope).await?;
        if attempt.state != AttemptState::Running {
            return Err(RunsError::NotFound);
        }
        Ok(attempt)
    }

    /// The grant is still live as authenticated, and its attempt is still the
    /// running one.
    async fn recheck(&self, grant: &Grant, attempt: &AttemptId) -> Result<(), RunsError> {
        self.inner.store.grant_live(grant).await?;
        if &self.running_attempt(grant).await?.id != attempt {
            return Err(RunsError::NotFound);
        }
        Ok(())
    }

    /// The language server's answer, or why there is none. A refusal (a path,
    /// project or grant outside reach) stays a refusal.
    async fn ask_code(
        &self,
        grant: &Grant,
        asked: LspQuestion,
    ) -> Result<(Option<LspAnswer>, Option<String>), RunsError> {
        let asker = Asker::Grant(&grant.project_id);
        match self.inner.code.lsp(asker, asked).await {
            Ok(answer @ LspAnswer::Answered { .. }) => Ok((Some(answer), None)),
            Ok(LspAnswer::NoServer {
                extension,
                server,
                state,
                ..
            }) => {
                let server = server.unwrap_or_else(|| "no known server".into());
                let state = state
                    .and_then(|s| serde_json::to_value(s).ok())
                    .and_then(|v| v["state"].as_str().map(str::to_string))
                    .unwrap_or_else(|| "unknown".into());
                let said = format!(
                    "no running language server reads `{extension}` files ({server}: {state}); \
                     the code was not looked at, which says nothing about whether it exists"
                );
                Ok((None, Some(Self::short_code_error(said))))
            }
            Err(CodebaseError::LanguageServer(message)) => {
                let said = format!(
                    "the language server did not answer: {message}; the code was not looked \
                     at, which says nothing about whether it exists"
                );
                Ok((None, Some(Self::short_code_error(said))))
            }
            Err(refused) => Err(refused.into()),
        }
    }

    /// `text`, cut at a character boundary to at most 2 KiB.
    fn short_code_error(mut text: String) -> String {
        if text.len() > MOST_CODE_ERROR {
            let mut end = MOST_CODE_ERROR;
            while !text.is_char_boundary(end) {
                end -= 1;
            }
            text.truncate(end);
        }
        text
    }
}
