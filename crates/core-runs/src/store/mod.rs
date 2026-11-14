//! Runs' private persistence: attempts, their saved contexts, task marks and
//! lessons records.
//!
//! `RunsStore` is private to the crate, so its methods are `pub` only to
//! the service; no other service's write calls into it, and nothing here is
//! `pub(crate)` or declared under `shared_in_transaction`.
mod brief;
mod lessons;
mod marks;
mod recover;

use std::sync::Arc;

use sqlx::SqliteConnection;

use crate::model::{Attempt, AttemptId, AttemptState, Need};
use crate::scope::Scope;
use crate::{Runs, RunsError};
use core_db::{Storage, StorageError};
use core_operations::OperationId;
use core_plans::{Plan, Plans, TaskContent, WorkflowId};
use core_projects::ProjectId;
use core_threads::ThreadId;

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
    Option<String>,
    Option<String>,
);

pub(super) struct RunsStore {
    storage: Arc<Storage>,
    /// Holds the next Start between its preflight and its admission write.
    #[cfg(feature = "test-support")]
    gate: std::sync::Mutex<Option<Gate>>,
}

/// A test's hold on one Start: it hears the Start arrive, then lets it go on.
#[cfg(feature = "test-support")]
struct Gate {
    reached: tokio::sync::oneshot::Sender<()>,
    release: tokio::sync::oneshot::Receiver<()>,
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
    /// The commit HEAD named at Start; `None` on a branch with no commit.
    pub(crate) start_head: Option<String>,
}

/// What an addition asks the store to record on the attempt it begins.
pub(crate) struct Continued {
    pub(crate) addition: String,
    /// The commit HEAD named when the addition was sent.
    pub(crate) start_head: Option<String>,
}

pub(crate) struct Begun {
    pub(crate) attempt: Attempt,
    /// The same command was already accepted: nothing was written.
    pub(crate) replayed: bool,
}

impl RunsStore {
    pub fn with_storage(storage: Arc<Storage>) -> Self {
        Self {
            storage,
            #[cfg(feature = "test-support")]
            gate: std::sync::Mutex::new(None),
        }
    }

    /// Holds the next `begin_attempt` before its write: the receiver hears it
    /// arrive, and the sender lets it go on.
    #[cfg(feature = "test-support")]
    pub fn gate(
        &self,
    ) -> (
        tokio::sync::oneshot::Receiver<()>,
        tokio::sync::oneshot::Sender<()>,
    ) {
        let (reached, heard) = tokio::sync::oneshot::channel();
        let (go, release) = tokio::sync::oneshot::channel();
        *self
            .gate
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(Gate { reached, release });
        (heard, go)
    }

    /// Admission's durable checks in one read snapshot: Start's preflight.
    pub async fn admission(
        &self,
        workflow: &WorkflowId,
        version: i64,
        task: u32,
    ) -> Result<(Plan, TaskContent), RunsError> {
        let mut snapshot = (self.storage.reader().begin().await).map_err(StorageError::from)?;
        let admitted = Runs::admit_in(&mut snapshot, workflow, version, task).await?;
        snapshot.commit().await.map_err(StorageError::from)?;
        Ok(admitted)
    }

    /// The tasks `task` needs, with their marks as one read sees them.
    pub async fn needs(&self, plan: &Plan, task: u32) -> Result<Vec<Need>, RunsError> {
        let mut conn = self
            .storage
            .reader()
            .acquire()
            .await
            .map_err(StorageError::from)?;
        Runs::needs_in(&mut conn, plan, task).await
    }

    /// Changes after each commit that appended to the journal, a turn's
    /// ending among them.
    pub fn committed(&self) -> tokio::sync::watch::Receiver<i64> {
        self.storage.watch_committed()
    }

    /// The replay check, the plan's state and `needs` marks, the busy check,
    /// the overlap check and the insert are one write, so a Start cannot pass
    /// on what a concurrent Start, Draft or mark changed.
    pub async fn begin_attempt(&self, new: NewAttempt) -> Result<Begun, RunsError> {
        #[cfg(feature = "test-support")]
        self.wait_at_gate().await;
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
                    Runs::admit_in(conn, &new.workflow, new.version, new.task).await?;
                    Self::check_free(conn, &new).await?;
                    let thread = core_threads::Threads::create_executor_in(
                        conn,
                        &new.project,
                        format!("T{}", new.task),
                        crate::EXECUTOR_HARNESS,
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
                            model, state, writes_json, started_at, command_id, fingerprint,
                            start_head)
                         VALUES (?,?,?,?,?,?,?,?,'running',?,?,?,?,?)",
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
                    .bind(&new.start_head)
                    .execute(&mut *conn)
                    .await?;
                    let task = (&new.project, &new.workflow, new.task);
                    Self::capture_in(conn, &id, task, &ts).await?;
                    let attempt = Self::load(conn, &id).await?;
                    Ok(Begun {
                        attempt,
                        replayed: false,
                    })
                })
            })
            .await
    }

    #[cfg(feature = "test-support")]
    async fn wait_at_gate(&self) {
        let gate = self
            .gate
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .take();
        if let Some(gate) = gate {
            let _ = gate.reached.send(());
            let _ = gate.release.await;
        }
    }

    /// The attempt `command` already started, read before any check that the
    /// plan's later state could change.
    pub async fn replay(
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

    /// The busy and overlap checks of `begin_attempt`, read without writing:
    /// what Start would answer now. Only `begin_attempt`'s write decides.
    pub async fn check_held(&self, new: &NewAttempt) -> Result<(), RunsError> {
        let mut conn = self
            .storage
            .reader()
            .acquire()
            .await
            .map_err(StorageError::from)?;
        Self::check_free(&mut conn, new).await
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

    pub async fn get(&self, id: &AttemptId) -> Result<Attempt, RunsError> {
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
                    operation_id, model, state, commit_sha, reason, started_at, ended_at,
                    report, addition
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
            report: row.14,
            addition: row.15,
        })
    }

    pub async fn set_operation(
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
    pub async fn finish(
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
    pub async fn executor_attempt(
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
    pub async fn writes(&self, id: &AttemptId) -> Result<Vec<String>, RunsError> {
        let writes: String =
            sqlx::query_scalar("SELECT writes_json FROM run_attempts WHERE id = ?")
                .bind(id.as_str())
                .fetch_optional(self.storage.reader())
                .await?
                .ok_or(RunsError::NotFound)?;
        Ok(serde_json::from_str(&writes)?)
    }

    /// The commit HEAD named when the attempt started.
    pub async fn start_head(&self, id: &AttemptId) -> Result<Option<String>, RunsError> {
        let head: Option<Option<String>> =
            sqlx::query_scalar("SELECT start_head FROM run_attempts WHERE id = ?")
                .bind(id.as_str())
                .fetch_optional(self.storage.reader())
                .await?;
        head.ok_or(RunsError::NotFound)
    }

    /// Records the accepted commit and report: a `Running` attempt becomes
    /// `Committed`. Answers whether this call moved it; one that left
    /// `Running` meanwhile keeps its state.
    pub async fn record_report(
        &self,
        id: &AttemptId,
        sha: &str,
        report: &str,
    ) -> Result<bool, RunsError> {
        let (id, sha, report) = (id.clone(), sha.to_string(), report.to_string());
        let ts = Storage::now();
        self.storage
            .write_txn_as(move |conn| {
                Box::pin(async move {
                    let moved = sqlx::query(
                        "UPDATE run_attempts
                         SET state = 'committed', commit_sha = ?, report = ?, ended_at = ?
                         WHERE id = ? AND state = 'running'",
                    )
                    .bind(&sha)
                    .bind(&report)
                    .bind(&ts)
                    .bind(id.as_str())
                    .execute(&mut *conn)
                    .await?;
                    Ok(moved.rows_affected() == 1)
                })
            })
            .await
    }

    /// The attempt's session cannot write any more: its scope is free.
    pub async fn release(&self, id: &AttemptId) -> Result<(), RunsError> {
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

    /// The task's latest attempt when it is committed and still holds its
    /// scope: the one waiting for the Planner's judgment.
    pub async fn awaiting(
        &self,
        workflow: &WorkflowId,
        version: i64,
        task: u32,
    ) -> Result<Option<(AttemptId, ThreadId)>, RunsError> {
        let row: Option<(String, String, String, Option<String>)> = sqlx::query_as(
            "SELECT id, state, thread_id, released_at FROM run_attempts
             WHERE workflow_id = ? AND version = ? AND task = ?
             ORDER BY number DESC LIMIT 1",
        )
        .bind(workflow.as_str())
        .bind(version)
        .bind(i64::from(task))
        .fetch_optional(self.storage.reader())
        .await?;
        Ok(match row {
            Some((id, state, thread, None)) if state == AttemptState::Committed.as_str() => Some((
                AttemptId::from_recorded(id),
                ThreadId::from_recorded(thread),
            )),
            _ => None,
        })
    }

    /// How many of the task's attempts, from the newest down, carry an
    /// addition before the first that does not (the one a Start began).
    pub async fn additions_since_start(
        &self,
        workflow: &WorkflowId,
        version: i64,
        task: u32,
    ) -> Result<u32, RunsError> {
        let additions: Vec<Option<String>> = sqlx::query_scalar(
            "SELECT addition FROM run_attempts
             WHERE workflow_id = ? AND version = ? AND task = ? ORDER BY number DESC",
        )
        .bind(workflow.as_str())
        .bind(version)
        .bind(i64::from(task))
        .fetch_all(self.storage.reader())
        .await?;
        let count = additions.iter().take_while(|a| a.is_some()).count();
        Ok(u32::try_from(count).unwrap_or(u32::MAX))
    }

    /// One write: `previous`, committed and still holding its scope, is
    /// released into a new `Running` attempt on the same thread carrying the
    /// addition. Any other `previous` is `NotAwaiting`; a version that may no
    /// longer start work keeps its scope with `previous` and answers why.
    pub async fn continue_attempt(
        &self,
        previous: &AttemptId,
        next: Continued,
    ) -> Result<Attempt, RunsError> {
        let (previous, ts) = (previous.clone(), Storage::now());
        self.storage
            .write_txn_as(move |conn| {
                Box::pin(async move {
                    type Row = (i64, String, Option<String>, String, i64, String);
                    let row: Option<Row> = sqlx::query_as(
                        "SELECT task, state, released_at, workflow_id, version, project_id
                         FROM run_attempts WHERE id = ?",
                    )
                    .bind(previous.as_str())
                    .fetch_optional(&mut *conn)
                    .await?;
                    let (task, state, released, workflow, version, project) =
                        row.ok_or(RunsError::NotFound)?;
                    if state != AttemptState::Committed.as_str() || released.is_some() {
                        return Err(RunsError::NotAwaiting {
                            task: u32::try_from(task).unwrap_or(0),
                        });
                    }
                    let workflow = WorkflowId::from_recorded(workflow);
                    let execution = Plans::execution_plan_in(conn, &workflow, version)
                        .await
                        .map_err(RunsError::from_plans)?;
                    if let Some(refusal) = execution.refusal {
                        return Err(refusal.into());
                    }
                    sqlx::query("UPDATE run_attempts SET released_at = ? WHERE id = ?")
                        .bind(&ts)
                        .bind(previous.as_str())
                        .execute(&mut *conn)
                        .await?;
                    let id = AttemptId::generate();
                    sqlx::query(
                        "INSERT INTO run_attempts
                           (id, project_id, workflow_id, version, task, number, thread_id,
                            model, state, writes_json, started_at, command_id, fingerprint,
                            start_head, addition)
                         SELECT ?, p.project_id, p.workflow_id, p.version, p.task,
                                (SELECT MAX(o.number) + 1 FROM run_attempts o
                                 WHERE o.workflow_id = p.workflow_id
                                   AND o.version = p.version AND o.task = p.task),
                                p.thread_id, p.model, 'running', p.writes_json, ?, ?,
                                'amend', ?, ?
                         FROM run_attempts p WHERE p.id = ?",
                    )
                    .bind(id.as_str())
                    .bind(&ts)
                    .bind(format!("amend-{id}"))
                    .bind(&next.start_head)
                    .bind(&next.addition)
                    .bind(previous.as_str())
                    .execute(&mut *conn)
                    .await?;
                    let project = ProjectId::from_recorded(project);
                    let task = u32::try_from(task).unwrap_or(0);
                    Self::capture_in(conn, &id, (&project, &workflow, task), &ts).await?;
                    Self::load(conn, &id).await
                })
            })
            .await
    }

    /// A task's attempts, newest first.
    pub async fn attempts(
        &self,
        workflow: &WorkflowId,
        version: i64,
        task: u32,
    ) -> Result<Vec<Attempt>, RunsError> {
        let rows: Vec<AttemptRow> = sqlx::query_as(
            "SELECT id, project_id, workflow_id, version, task, number, thread_id,
                    operation_id, model, state, commit_sha, reason, started_at, ended_at,
                    report, addition
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
}
