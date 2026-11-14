//! One job: an executor's contract conflict on its own attempt, and reading
//! it back.
//!
//! Accepting a conflict records it and asks the executor to stop work and end
//! its turn; it stops nothing itself. The Planner hears it through `notify`
//! at once, while the attempt may still run; the watcher settles the attempt
//! when the turn's end is confirmed; the Planner stops affected attempts with
//! `stop_for`. Nothing here stops a sibling or follows a link.
use codebase::Asker;
use core_db::CommandContext;
use core_grants::{Grant, GrantKind};

use crate::model::{AttemptId, ConflictEvidence, ConflictInput, TaskConflict};
use crate::{Runs, RunsError};

/// The most Unicode scalars `expected` and `observed` each hold.
const MOST_TEXT: usize = 2_000;
/// The most pieces of evidence.
const MOST_EVIDENCE: usize = 10;
/// The most Unicode scalars one piece's summary holds.
const MOST_SUMMARY: usize = 1_000;
/// The most bytes one evidence path holds.
const MOST_PATH: usize = 1_024;
/// The most bytes the normalized conflict holds.
const MOST_PAYLOAD: usize = 16 * 1024;

/// A conflict as recorded on its attempt: the normalized input without its
/// command id. Its fingerprint is the command's.
#[derive(serde::Serialize, serde::Deserialize)]
struct Recorded {
    expected: String,
    observed: String,
    evidence: Vec<ConflictEvidence>,
}

impl Runs {
    /// Accepts `input` as the contract conflict of the attempt `grant`, an
    /// executor's, belongs to: never an attempt the caller names. A replay of
    /// its command answers the conflict as it stands. The executor must then
    /// stop work and end its turn; the answer's `attempt_state` says only
    /// what is confirmed.
    pub async fn conflict_for(
        &self,
        grant: &Grant,
        input: ConflictInput,
    ) -> Result<TaskConflict, RunsError> {
        let (Some(thread), Some(scope), GrantKind::Executor) =
            (&grant.thread_id, &grant.scope, grant.kind)
        else {
            return Err(RunsError::NotFound);
        };
        let attempt = self.inner.store.executor_attempt(thread, scope).await?;
        let command_id = input.command_id.trim().to_string();
        if command_id.is_empty() {
            return Err(RunsError::InvalidConflict("command_id is empty".into()));
        }
        let recorded = Self::normalized(input)?;
        // A path that does not exist is evidence too; one outside the project
        // or holding a secret stays the code service's refusal.
        let paths: Vec<String> = recorded.evidence.iter().map(|e| e.path.clone()).collect();
        (self.inner.code)
            .missing_inputs(Asker::Grant(&grant.project_id), &paths)
            .await?;
        let payload = serde_json::to_value(&recorded)?;
        let command = CommandContext {
            agent: None,
            principal_kind: "Executor".into(),
            principal_id: thread.as_str().to_string(),
            command_id,
            command_kind: "run.conflict".into(),
            command_schema_ver: 1,
            request_fingerprint: CommandContext::fingerprint("run.conflict", &payload),
        };
        let accepted = (self.inner.store)
            .record_conflict(grant, &attempt.id, command, payload.to_string())
            .await?;
        if accepted {
            tracing::info!(attempt = %attempt.id, "runs.conflict_accepted");
            let now = self.inner.store.get(&attempt.id).await?;
            if let Err(error) = self.notify(&now).await {
                tracing::warn!(attempt = %attempt.id, %error, "runs.conflict_notice_pending");
            }
        }
        (self.conflict(&attempt.id).await?).ok_or(RunsError::NotFound)
    }

    /// The contract conflict of `attempt`, with its current state and held
    /// scope; `None` when it has none. Unknown is `NotFound`.
    pub async fn conflict(&self, attempt: &AttemptId) -> Result<Option<TaskConflict>, RunsError> {
        let Some(row) = self.inner.store.conflict(attempt).await? else {
            return Ok(None);
        };
        let recorded: Recorded = serde_json::from_str(&row.conflict_json)?;
        let attempt = row.attempt;
        Ok(Some(TaskConflict {
            attempt_id: attempt.id,
            project_id: attempt.project_id,
            workflow_id: attempt.workflow_id,
            version: attempt.version,
            task: attempt.task,
            expected: recorded.expected,
            observed: recorded.observed,
            evidence: recorded.evidence,
            attempt_state: attempt.state,
            scope_held: row.held,
            delivery: row.delivery,
            created_at: row.at,
        }))
    }

    /// `input` trimmed, its paths in one spelling, its evidence in its order,
    /// within the bounds.
    fn normalized(input: ConflictInput) -> Result<Recorded, RunsError> {
        let invalid = |message: String| RunsError::InvalidConflict(message);
        let text = |value: String, name: &str, most: usize| {
            let value = value.trim().to_string();
            let count = value.chars().count();
            if count == 0 || count > most {
                return Err(invalid(format!("{name} must hold 1 to {most} characters")));
            }
            Ok(value)
        };
        let expected = text(input.expected, "expected", MOST_TEXT)?;
        let observed = text(input.observed, "observed", MOST_TEXT)?;
        if input.evidence.is_empty() || input.evidence.len() > MOST_EVIDENCE {
            return Err(invalid(format!(
                "evidence must hold 1 to {MOST_EVIDENCE} places"
            )));
        }
        let mut evidence = Vec::with_capacity(input.evidence.len());
        for piece in input.evidence {
            if piece.line == Some(0) {
                return Err(invalid("a line is one-based".into()));
            }
            evidence.push(ConflictEvidence {
                path: Self::evidence_path(&piece.path)?,
                line: piece.line,
                summary: text(piece.summary, "a summary", MOST_SUMMARY)?,
            });
        }
        let recorded = Recorded {
            expected,
            observed,
            evidence,
        };
        if serde_json::to_vec(&recorded)?.len() > MOST_PAYLOAD {
            return Err(invalid("the conflict is larger than 16 KiB".into()));
        }
        Ok(recorded)
    }

    /// One spelling of a project-relative path: forward slashes, no leading
    /// `./`, no doubled slash. Whether it stays inside the project is the
    /// code service's check.
    fn evidence_path(path: &str) -> Result<String, RunsError> {
        let mut path = path.trim().replace('\\', "/");
        while path.contains("//") {
            path = path.replace("//", "/");
        }
        while let Some(rest) = path.strip_prefix("./") {
            path = rest.to_string();
        }
        if path.is_empty() || path.len() > MOST_PATH {
            return Err(RunsError::InvalidConflict(format!(
                "a path must hold 1 to {MOST_PATH} bytes"
            )));
        }
        Ok(path)
    }
}
