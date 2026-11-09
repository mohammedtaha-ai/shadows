//! Runs' private persistence: attempts and task marks.
mod marks;
mod recover;

use std::sync::Arc;

use sqlx::SqliteConnection;

use crate::model::{Attempt, AttemptId, AttemptState};
use crate::scope::Scope;
use crate::{RunsError, model::TaskRun};
use core_db::{Storage, StorageError};
use core_operations::OperationId;
use core_plans::WorkflowId;
use core_projects::ProjectId;
use core_threads::ThreadId;

/// The harness an executor thread runs on.
const HARNESS: &str = "claude-code";

type AttemptRow = (
    String,
    String,
    String,
    i64,
    i64,
    i64,
    String,
    Option<String>,
    String,
    String,
    Option<String>,
    Option<String>,
    String,
    Option<String>,
);

pub(crate) struct RunsStore {
    storage: Arc<Storage>,
}

/// What Start asks the store to record.
pub(crate) struct NewAttempt {
    pub(crate) command_id: String,
    pub(crate) fingerprint: String,
    pub(crate) project: ProjectId,
    pub(crate) workflow: WorkflowId,
    pub(crate) version: i64,
    pub(crate) task: u32,
    pub(crate) model: String,
    pub(crate) writes: Vec<String>,
}

pub(crate) struct Begun {
    pub(crate) attempt: Attempt,
    /// The same command was already accepted: nothing was written.
    pub(crate) replayed: bool,
}

impl RunsStore {
    pub(crate) fn with_storage(storage: Arc<Storage>) -> Self {
        Self { storage }
    }

    /// The replay check, the busy check, the overlap check and the insert are
    /// one write, so two concurrent Starts cannot both pass.
    pub(crate) async fn begin_attempt(&self, new: NewAttempt) -> Result<Begun, RunsError> {
        let ts = Storage::now();
        self.storage
            .write_txn_as(move |conn| {
                Box::pin(async move {
                    if let Some(attempt) =
                        Self::by_command(conn, &new.command_id, &new.fingerprint).await?
                    {
                        return Ok(Begun {
                            attempt,
                            replayed: true,
                        });
                    }
                    Self::check_free(conn, &new).await?;
                    let thread = core_threads::Threads::create_executor_in(
                        conn,
                        &new.project,
                        format!("T{}", new.task),
                        HARNESS,
                    )
                    .await?;
                    let number: i64 = sqlx::query_scalar(
                        "SELECT COALESCE(MAX(number), 0) + 1 FROM run_attempts
                         WHERE workflow_id = ? AND version = ? AND task = ?",
                    )
                    .bind(new.workflow.as_str())
                    .bind(new.version)
                    .bind(i64::from(new.task))
                    .fetch_one(&mut *conn)
                    .await?;
                    let id = AttemptId::generate();
                    sqlx::query(
                        "INSERT INTO run_attempts
                           (id, project_id, workflow_id, version, task, number, thread_id,
                            model, state, writes_json, started_at, command_id, fingerprint)
                         VALUES (?,?,?,?,?,?,?,?,'running',?,?,?,?)",
                    )
                    .bind(id.as_str())
                    .bind(new.project.as_str())
                    .bind(new.workflow.as_str())
                    .bind(new.version)
                    .bind(i64::from(new.task))
                    .bind(number)
                    .bind(thread.as_str())
                    .bind(&new.model)
                    .bind(serde_json::to_string(&new.writes)?)
                    .bind(&ts)
                    .bind(&new.command_id)
                    .bind(&new.fingerprint)
                    .execute(&mut *conn)
                    .await?;
                    let attempt = Self::load(conn, &id).await?;
                    Ok(Begun {
                        attempt,
                        replayed: false,
                    })
                })
            })
            .await
    }

    /// The attempt `command` already started, read before any check that the
    /// plan's later state could change.
    pub(crate) async fn replay(
        &self,
        command: &str,
        fingerprint: &str,
    ) -> Result<Option<Attempt>, RunsError> {
        let mut conn = self
            .storage
            .reader()
            .acquire()
            .await
            .map_err(StorageError::from)?;
        Self::by_command(&mut conn, command, fingerprint).await
    }

    /// The attempt `command` started; the same id with other input is a
    /// conflict.
    async fn by_command(
        conn: &mut SqliteConnection,
        command: &str,
        fingerprint: &str,
    ) -> Result<Option<Attempt>, RunsError> {
        let row: Option<(String, String)> =
            sqlx::query_as("SELECT id, fingerprint FROM run_attempts WHERE command_id = ?")
                .bind(command)
                .fetch_optional(&mut *conn)
                .await?;
        match row {
            Some((_, seen)) if seen != fingerprint => Err(RunsError::CommandConflict),
            Some((id, _)) => Ok(Some(Self::load(conn, &AttemptId::from_recorded(id)).await?)),
            None => Ok(None),
        }
    }

    /// An attempt holds its scope until `released_at`, whatever its state.
    async fn check_free(conn: &mut SqliteConnection, new: &NewAttempt) -> Result<(), RunsError> {
        let held: Vec<(i64, i64, String, String)> = sqlx::query_as(
            "SELECT task, version, workflow_id, writes_json FROM run_attempts
             WHERE project_id = ? AND released_at IS NULL ORDER BY started_at, id",
        )
        .bind(new.project.as_str())
        .fetch_all(&mut *conn)
        .await?;
        let same = |(task, version, workflow, _): &(i64, i64, String, String)| {
            *task == i64::from(new.task)
                && *version == new.version
                && workflow == new.workflow.as_str()
        };
        if held.iter().any(same) {
            return Err(RunsError::TaskBusy);
        }
        for (task, _, _, writes) in &held {
            let writes: Vec<String> = serde_json::from_str(writes)?;
            if Scope::may_overlap(&new.writes, &writes) {
                return Err(RunsError::Overlap {
                    task: u32::try_from(*task).unwrap_or(0),
                });
            }
        }
        Ok(())
    }

    pub(crate) async fn get(&self, id: &AttemptId) -> Result<Attempt, RunsError> {
        let mut conn = self
            .storage
            .reader()
            .acquire()
            .await
            .map_err(StorageError::from)?;
        Self::load(&mut conn, id).await
    }

    async fn load(conn: &mut SqliteConnection, id: &AttemptId) -> Result<Attempt, RunsError> {
        let row: Option<AttemptRow> = sqlx::query_as(
            "SELECT id, project_id, workflow_id, version, task, number, thread_id,
                    operation_id, model, state, commit_sha, reason, started_at, ended_at
                 FROM run_attempts WHERE id = ?",
        )
        .bind(id.as_str())
        .fetch_optional(&mut *conn)
        .await?;
        row.map(Self::into_attempt)
            .transpose()?
            .ok_or(RunsError::NotFound)
    }

    fn into_attempt(row: AttemptRow) -> Result<Attempt, StorageError> {
        let small = |n: i64| {
            u32::try_from(n).map_err(|_| StorageError::Constraint("run_attempts number".into()))
        };
        let state = AttemptState::parse(&row.9)
            .ok_or_else(|| StorageError::Constraint(format!("run_attempts.state {}", row.9)))?;
        Ok(Attempt {
            id: AttemptId::from_recorded(row.0),
            project_id: ProjectId::from_recorded(row.1),
            workflow_id: WorkflowId::from_recorded(row.2),
            version: row.3,
            task: small(row.4)?,
            number: small(row.5)?,
            thread_id: ThreadId::from_recorded(row.6),
            operation_id: row.7.map(OperationId::from_recorded),
            model: row.8,
            state,
            commit: row.10,
            reason: row.11,
            started_at: row.12,
            ended_at: row.13,
        })
    }

    pub(crate) async fn set_operation(
        &self,
        id: &AttemptId,
        operation: &OperationId,
    ) -> Result<(), RunsError> {
        let (id, operation) = (id.clone(), operation.clone());
        self.storage
            .write_txn_as(move |conn| {
                Box::pin(async move {
                    sqlx::query("UPDATE run_attempts SET operation_id = ? WHERE id = ?")
                        .bind(operation.as_str())
                        .bind(id.as_str())
                        .execute(&mut *conn)
                        .await?;
                    Ok(())
                })
            })
            .await
    }

    /// Leaves `Running` for `state`. Answers whether this call moved it: an
    /// attempt that already left `Running` keeps the state it has.
    pub(crate) async fn finish(
        &self,
        id: &AttemptId,
        state: AttemptState,
        reason: Option<String>,
    ) -> Result<bool, RunsError> {
        let (id, ts) = (id.clone(), Storage::now());
        self.storage
            .write_txn_as(move |conn| {
                Box::pin(async move {
                    let moved = sqlx::query(
                        "UPDATE run_attempts SET state = ?, reason = ?, ended_at = ?
                         WHERE id = ? AND state = 'running'",
                    )
                    .bind(state.as_str())
                    .bind(reason)
                    .bind(&ts)
                    .bind(id.as_str())
                    .execute(&mut *conn)
                    .await?;
                    Ok(moved.rows_affected() == 1)
                })
            })
            .await
    }

    /// The latest attempt of the task `scope` names on `thread`: the one an
    /// executor grant belongs to.
    pub(crate) async fn executor_attempt(
        &self,
        thread: &ThreadId,
        scope: &core_grants::TaskScope,
    ) -> Result<Attempt, RunsError> {
        let id: Option<String> = sqlx::query_scalar(
            "SELECT id FROM run_attempts
             WHERE thread_id = ? AND workflow_id = ? AND version = ? AND task = ?
             ORDER BY number DESC LIMIT 1",
        )
        .bind(thread.as_str())
        .bind(&scope.workflow)
        .bind(scope.version)
        .bind(i64::from(scope.task))
        .fetch_optional(self.storage.reader())
        .await?;
        let id = id.ok_or(RunsError::NotFound)?;
        self.get(&AttemptId::from_recorded(id)).await
    }

    /// The `writes` the attempt was started with.
    pub(crate) async fn writes(&self, id: &AttemptId) -> Result<Vec<String>, RunsError> {
        let writes: String =
            sqlx::query_scalar("SELECT writes_json FROM run_attempts WHERE id = ?")
                .bind(id.as_str())
                .fetch_optional(self.storage.reader())
                .await?
                .ok_or(RunsError::NotFound)?;
        Ok(serde_json::from_str(&writes)?)
    }

    /// Records the commit git already holds. A `Running` attempt becomes
    /// `Committed`; one that left `Running` meanwhile keeps its state and
    /// still names its commit.
    pub(crate) async fn record_commit(&self, id: &AttemptId, sha: &str) -> Result<(), RunsError> {
        let (id, sha, ts) = (id.clone(), sha.to_string(), Storage::now());
        self.storage
            .write_txn_as(move |conn| {
                Box::pin(async move {
                    sqlx::query(
                        "UPDATE run_attempts SET commit_sha = ?,
                           state = CASE WHEN state = 'running' THEN 'committed' ELSE state END,
                           ended_at = COALESCE(ended_at, ?)
                         WHERE id = ? AND commit_sha IS NULL",
                    )
                    .bind(&sha)
                    .bind(&ts)
                    .bind(id.as_str())
                    .execute(&mut *conn)
                    .await?;
                    Ok(())
                })
            })
            .await
    }

    /// The attempt's session cannot write any more: its scope is free.
    pub(crate) async fn release(&self, id: &AttemptId) -> Result<(), RunsError> {
        let (id, ts) = (id.clone(), Storage::now());
        self.storage
            .write_txn_as(move |conn| {
                Box::pin(async move {
                    sqlx::query(
                        "UPDATE run_attempts SET released_at = ?
                         WHERE id = ? AND released_at IS NULL",
                    )
                    .bind(&ts)
                    .bind(id.as_str())
                    .execute(&mut *conn)
                    .await?;
                    Ok(())
                })
            })
            .await
    }

    /// A task's attempts, newest first.
    pub(crate) async fn attempts(
        &self,
        workflow: &WorkflowId,
        version: i64,
        task: u32,
    ) -> Result<Vec<Attempt>, RunsError> {
        let rows: Vec<AttemptRow> = sqlx::query_as(
            "SELECT id, project_id, workflow_id, version, task, number, thread_id,
                    operation_id, model, state, commit_sha, reason, started_at, ended_at
             FROM run_attempts
             WHERE workflow_id = ? AND version = ? AND task = ? ORDER BY number DESC",
        )
        .bind(workflow.as_str())
        .bind(version)
        .bind(i64::from(task))
        .fetch_all(self.storage.reader())
        .await?;
        Ok(rows
            .into_iter()
            .map(Self::into_attempt)
            .collect::<Result<_, _>>()?)
    }

    pub(crate) async fn task_run(
        &self,
        workflow: &WorkflowId,
        version: i64,
        task: u32,
    ) -> Result<TaskRun, RunsError> {
        let latest = self
            .attempts(workflow, version, task)
            .await?
            .into_iter()
            .next();
        let mark = self.mark(workflow, version, task).await?;
        Ok(TaskRun {
            task,
            done: mark.as_ref().is_some_and(|m| m.0),
            done_at: mark.and_then(|m| m.1),
            latest,
        })
    }
}
