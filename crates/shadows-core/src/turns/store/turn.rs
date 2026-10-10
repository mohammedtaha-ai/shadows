//! One job: starting a turn as one command (spec §12.7).
//!
//! The user's entry, the `Pending` operation, its invocation, the remembered
//! settings, the thread's title from its first message (§4.2) and the
//! command record commit together or not at all, so a retried request can
//! never become a second message or a second run.

use sqlx::SqliteConnection;

use super::operation::insert_pending;
use crate::command::CommandContext;
use crate::db::{Storage, StorageError, append_event, classify, now, record_command};
use crate::events::{Actor, DurableEvent};
use crate::harness::remember_settings;
use crate::plans::Focus;
use crate::plans::task_of;
use crate::runtime::RuntimeInstanceId;
use crate::threads::{
    EntryRef, NewThreadEntry, ThreadEntryId, ThreadEntryKind, ThreadId, append_entry_in,
    title_from_first_message_in,
};
use crate::turns::model::{OperationId, QueuedMessageId};
use shadows_agent::TurnSettings;

/// A waiting message the turn sends (§20.3, §20.4): taken in the turn's own
/// transaction, so it is sent once or not at all. `also` is Send now's own
/// command, recorded with the turn so its replay answers the turn.
#[derive(Debug, Clone, Copy)]
pub struct Dequeue<'a> {
    pub id: &'a QueuedMessageId,
    pub also: Option<&'a CommandContext>,
}

/// Everything the turn command records. The paths and versions are the
/// adapter's and the CLI's it runs (§12.2), frozen on the invocation.
#[derive(Debug, Clone, Copy)]
pub struct NewTurn<'a> {
    pub thread_id: &'a ThreadId,
    pub runtime: &'a RuntimeInstanceId,
    pub prompt: &'a str,
    pub role: &'a str,
    pub harness_kind: &'a str,
    pub harness_path: &'a str,
    pub harness_version: &'a str,
    pub agent_path: &'a str,
    pub agent_version: &'a str,
    pub settings: &'a TurnSettings,
    /// `harness::prompt_version()` when the turn started (§13.8).
    pub prompt_version: Option<&'a str>,
    /// The project's current `planner_instructions_version` id, if it has one.
    pub instructions_version: Option<&'a str>,
    /// The base version and additions id selected for this turn (§23.4).
    pub standards_version: Option<i64>,
    pub standards_additions_version: Option<&'a str>,
    /// The task the person points at (§13.9), kept with their message.
    pub focus: Option<&'a Focus>,
    /// The waiting message this turn sends, taken in the same transaction.
    pub dequeue: Option<Dequeue<'a>>,
}

/// What the command recorded; `replayed` when it had already happened.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StartedTurn {
    pub operation_id: OperationId,
    pub entry_id: ThreadEntryId,
    pub replayed: bool,
    /// The focused task's number and title, read in the transaction that
    /// checked it (§13.9); `None` without a focus, and on a replay.
    pub focus_task: Option<(u32, String)>,
}

/// Versions delivered by the thread's most recent started turn (§13.8, §23.4).
#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct InvocationVersions {
    pub prompt: Option<String>,
    pub instructions: Option<String>,
    pub standards: Option<i64>,
    pub additions: Option<String>,
}

const SCOPE: &str = "Thread";

fn recorded(outcome: &str) -> Result<StartedTurn, StorageError> {
    let v: serde_json::Value = serde_json::from_str(outcome)?;
    let text = |key: &str| {
        v.get(key)
            .and_then(serde_json::Value::as_str)
            .map(str::to_owned)
            .ok_or(StorageError::NotFound("recorded turn"))
    };
    Ok(StartedTurn {
        operation_id: OperationId::from_stored(text("operation_id")?),
        entry_id: ThreadEntryId::from_stored(text("entry_id")?),
        replayed: true,
        focus_task: None,
    })
}

/// The focused task's number and title, when it is a task of the focus's
/// version and that version belongs to this thread's project (§13.9, §16.4).
async fn focused(
    conn: &mut SqliteConnection,
    thread: &ThreadId,
    focus: &Focus,
) -> Result<(u32, String), StorageError> {
    let in_project: Option<i64> = sqlx::query_scalar(
        "SELECT 1 FROM workflow w JOIN plan p ON p.id = w.plan_id
            JOIN planning_thread t ON t.project_id = p.project_id
            WHERE w.id = ? AND t.id = ?",
    )
    .bind(focus.workflow_id.as_str())
    .bind(thread.as_str())
    .fetch_optional(&mut *conn)
    .await?;
    if in_project.is_none() {
        return Err(StorageError::TaskNotInPlan(
            "the chosen plan is not in this conversation's project".into(),
        ));
    }
    task_of(conn, &focus.workflow_id, &focus.task_id)
        .await?
        .ok_or_else(|| StorageError::TaskNotInPlan("the chosen task is not in that plan".into()))
}

/// Whether the thread has a turn that has not reached a terminal status.
pub(crate) async fn has_open_operation(
    conn: &mut SqliteConnection,
    thread: &ThreadId,
) -> Result<bool, StorageError> {
    let found: Option<i64> = sqlx::query_scalar(
        "SELECT 1 FROM operation
          WHERE thread_id = ?
            AND status_kind NOT IN ('Completed','Failed','Cancelled','Interrupted')
          LIMIT 1",
    )
    .bind(thread.as_str())
    .fetch_optional(&mut *conn)
    .await?;
    Ok(found.is_some())
}

impl Storage {
    /// §12.7's one transaction. A replay found inside it answers the recorded
    /// ids with `replayed: true`, so two concurrent first requests produce
    /// one turn; a thread with a turn not yet ended is `ThreadBusy`; a stopped
    /// runtime is refused as `create_pending_operation` refuses it.
    pub async fn start_turn(
        &self,
        ctx: &CommandContext,
        turn: NewTurn<'_>,
    ) -> Result<StartedTurn, StorageError> {
        let (ctx, ts) = (ctx.clone(), now());
        let thread = turn.thread_id.clone();
        let runtime = turn.runtime.clone();
        let prompt = turn.prompt.to_string();
        let settings = turn.settings.clone();
        let focus = turn.focus.cloned();
        let dequeue = turn.dequeue.map(|d| (d.id.clone(), d.also.cloned()));
        let versions = (
            turn.prompt_version.map(str::to_owned),
            turn.instructions_version.map(str::to_owned),
            turn.standards_version,
            turn.standards_additions_version.map(str::to_owned),
        );
        let invocation = [
            turn.role,
            turn.harness_kind,
            turn.harness_path,
            turn.harness_version,
            turn.agent_path,
            turn.agent_version,
        ]
        .map(str::to_owned);
        let (started, transition) = self
            .write_txn(move |conn| {
                Box::pin(async move {
                    if let Some(outcome) = classify(conn, &ctx, SCOPE, thread.as_str()).await? {
                        return Ok((recorded(&outcome)?, None));
                    }
                    let live: Option<i64> = sqlx::query_scalar(
                        "SELECT 1 FROM planning_thread WHERE id = ? AND removed_at IS NULL",
                    )
                    .bind(thread.as_str())
                    .fetch_optional(&mut *conn)
                    .await?;
                    live.ok_or(StorageError::NotFound("planning_thread"))?;
                    if has_open_operation(conn, &thread).await? {
                        return Err(StorageError::ThreadBusy);
                    }
                    if let Some((id, _)) = &dequeue {
                        super::queue::take_queued_in(conn, &thread, id).await?;
                    }
                    let focus_task = match &focus {
                        Some(focus) => Some(focused(conn, &thread, focus).await?),
                        None => None,
                    };
                    let refs = focus.iter().flat_map(|f| {
                        [
                            EntryRef::Workflow(f.workflow_id.clone()),
                            EntryRef::Task(f.task_id.clone()),
                        ]
                    });
                    let refs: Vec<EntryRef> = refs.collect();
                    let op = OperationId::generate();
                    let transition = insert_pending(conn, &op, &thread, &runtime, &ts).await?;
                    let entry = append_entry_in(
                        conn,
                        &thread,
                        NewThreadEntry {
                            kind: ThreadEntryKind::UserMessage,
                            author: Actor::user(&ctx.principal_id),
                            body: &prompt,
                            refs: &refs,
                            card: None,
                            operation_id: Some(&op),
                        },
                        &ts,
                    )
                    .await?;
                    let author = Actor::user(&ctx.principal_id);
                    title_from_first_message_in(conn, &thread, &entry.id, &prompt, author, &ts)
                        .await?;
                    let [
                        role,
                        kind,
                        harness_path,
                        harness_version,
                        agent_path,
                        agent_version,
                    ] = &invocation;
                    sqlx::query(
                        "INSERT INTO agent_invocation
                           (id, operation_id, role, harness_kind, harness_path, harness_version,
                            agent_path, agent_version, requested_model, requested_mode,
                            requested_effort, prompt_version,
                            planner_instructions_version_id, standards_version,
                            standards_additions_version_id, created_at)
                         VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)",
                    )
                    .bind(uuid::Uuid::new_v4().to_string())
                    .bind(op.as_str())
                    .bind(role)
                    .bind(kind)
                    .bind(harness_path)
                    .bind(harness_version)
                    .bind(agent_path)
                    .bind(agent_version)
                    .bind(&settings.model)
                    .bind(&settings.mode)
                    .bind(&settings.effort)
                    .bind(&versions.0)
                    .bind(&versions.1)
                    .bind(versions.2)
                    .bind(&versions.3)
                    .bind(&ts)
                    .execute(&mut *conn)
                    .await?;
                    remember_settings(conn, kind, &settings, &ts).await?;
                    let outcome = serde_json::json!({
                        "operation_id": op.as_str(),
                        "entry_id": entry.id.as_str(),
                    });
                    record_command(
                        conn,
                        &ctx,
                        SCOPE,
                        thread.as_str(),
                        "Operation",
                        &outcome.to_string(),
                        &ts,
                    )
                    .await?;
                    if let Some((id, also)) = &dequeue {
                        let payload = serde_json::json!({
                            "queued_id": id.as_str(), "how": "turn", "operation_id": op.as_str(),
                        });
                        let sent =
                            DurableEvent::new("QueuedMessageSent", Actor::user(&ctx.principal_id))
                                .with_thread(&thread)
                                .with_payload(payload);
                        append_event(conn, &sent, &ts).await?;
                        if let Some(also) = also {
                            let answer = outcome.to_string();
                            let (scope, id) = (SCOPE, thread.as_str());
                            record_command(conn, also, scope, id, "Operation", &answer, &ts)
                                .await?;
                        }
                    }
                    let started = StartedTurn {
                        operation_id: op,
                        entry_id: entry.id,
                        replayed: false,
                        focus_task,
                    };
                    Ok((started, Some(transition)))
                })
            })
            .await?;
        if let Some(transition) = transition {
            transition.log();
        }
        Ok(started)
    }

    /// Read-only: the recorded answer to this exact command on `thread`, if it
    /// already happened; `CommandConflict` when its id was used with another
    /// request. Asked before any validation (§12.7): a replay starts nothing.
    pub async fn replayed_turn(
        &self,
        ctx: &CommandContext,
        thread: &ThreadId,
    ) -> Result<Option<StartedTurn>, StorageError> {
        let mut conn = self.reader().acquire().await?;
        match classify(&mut conn, ctx, SCOPE, thread.as_str()).await? {
            Some(outcome) => Ok(Some(recorded(&outcome)?)),
            None => Ok(None),
        }
    }

    /// The prompt, instructions and standards versions of the thread's
    /// latest invocation, by its turn's order (the ordinal of the turn's own
    /// entry); `None` when it has none. Only a turn that started counts: one
    /// refused or stopped before its prompt went out delivered nothing to the
    /// session, and a turn being started now is still `Pending` (§13.8).
    pub async fn latest_invocation_versions(
        &self,
        thread: &ThreadId,
    ) -> Result<Option<InvocationVersions>, StorageError> {
        Ok(sqlx::query_as(
            "SELECT i.prompt_version AS prompt, i.planner_instructions_version_id AS instructions,
                    i.standards_version AS standards, i.standards_additions_version_id AS additions
               FROM agent_invocation i
               JOIN operation o ON o.id = i.operation_id
               JOIN thread_entry e ON e.operation_id = o.id AND e.thread_id = o.thread_id
              WHERE o.thread_id = ? AND o.started_at IS NOT NULL AND e.kind = 'UserMessage'
              ORDER BY e.ordinal DESC
              LIMIT 1",
        )
        .bind(thread.as_str())
        .fetch_optional(self.reader())
        .await?)
    }

    /// Whether the thread has a turn that has not ended: checked before a new
    /// turn touches the thread's session, and again inside `start_turn`.
    pub async fn thread_is_busy(&self, thread: &ThreadId) -> Result<bool, StorageError> {
        let mut conn = self.reader().acquire().await?;
        has_open_operation(&mut conn, thread).await
    }
}
