//! One job: starting a turn as one command.
//!
//! The user's entry, the `Pending` operation, its invocation, the remembered
//! settings, the thread's title from its first message and the
//! command record commit together or not at all, so a retried request can
//! never become a second message or a second run.

use sqlx::SqliteConnection;

use crate::db::{Actor, CommandContext, StorageError, append_event, classify, now, record_command};
use crate::events::DurableEvent;
use crate::harness::{Harness, NewInvocation};
use crate::operations::Operations;
use crate::plans::Focus;
use crate::plans::Plans;
use crate::runtime::RuntimeInstanceId;
use crate::threads::{EntryRef, NewThreadEntry, ThreadEntryId, ThreadEntryKind, ThreadId, Threads};
use crate::turns::model::{OperationId, QueuedMessageId};
use shadows_agent::TurnSettings;

/// A waiting message the turn sends: taken in the turn's own
/// transaction, so it is sent once or not at all. `also` is Send now's own
/// command, recorded with the turn so its replay answers the turn.
#[derive(Debug, Clone, Copy)]
pub struct Dequeue<'a> {
    pub id: &'a QueuedMessageId,
    pub also: Option<&'a CommandContext>,
}

/// Everything the turn command records. The paths and versions are the
/// adapter's and the CLI's it runs, frozen on the invocation.
#[derive(Debug, Clone, Copy)]
pub struct NewTurn<'a> {
    pub aylul_usage: Option<&'a crate::AylulUsageSnapshot>,
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
    /// `harness::prompt_version()` when the turn started.
    pub prompt_version: Option<&'a str>,
    /// The project's current `planner_instructions_version` id, if it has one.
    pub instructions_version: Option<&'a str>,
    /// The base version and additions id selected for this turn.
    pub standards_version: Option<i64>,
    pub standards_additions_version: Option<&'a str>,
    /// The task the person points at, kept with their message.
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
    /// checked it; `None` without a focus, and on a replay.
    pub focus_task: Option<(u32, String)>,
}

pub use crate::harness::InvocationVersions;

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
/// version and that version belongs to this thread's project.
async fn focused(
    conn: &mut SqliteConnection,
    thread: &ThreadId,
    focus: &Focus,
) -> Result<(u32, String), crate::CoreError> {
    let project = Threads::project_in(conn, thread).await?;
    let in_project = Plans::workflow_in_project_in(conn, &focus.workflow_id, &project).await?;
    if !in_project {
        return Err(crate::plans::PlansError::TaskNotInPlan(
            "the chosen plan is not in this conversation's project".into(),
        )
        .into());
    }
    Plans::task_of(conn, &focus.workflow_id, &focus.task_id)
        .await?
        .ok_or_else(|| {
            crate::plans::PlansError::TaskNotInPlan("the chosen task is not in that plan".into())
                .into()
        })
}

impl super::TurnsStore {
    /// One transaction. A replay found inside it answers the recorded
    /// ids with `replayed: true`, so two concurrent first requests produce
    /// one turn; a thread with a turn not yet ended is `ThreadBusy`; a stopped
    /// runtime is refused as `create_pending_operation` refuses it.
    pub async fn start_turn(
        &self,
        ctx: &CommandContext,
        turn: NewTurn<'_>,
    ) -> Result<StartedTurn, crate::CoreError> {
        let snapshot = turn.aylul_usage.cloned();
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
            .storage
            .write_txn_as::<_, _, crate::CoreError>(move |conn| {
                Box::pin(async move {
                    if let Some(outcome) = classify(conn, &ctx, SCOPE, thread.as_str()).await? {
                        return Ok((recorded(&outcome)?, None));
                    }
                    crate::threads::Threads::require_live_in(conn, &thread).await?;
                    if Operations::has_open_in(conn, &thread).await? {
                        return Err(crate::turns::error::TurnsError::ThreadBusy.into());
                    }
                    if let Some((id, _)) = &dequeue {
                        Self::take_queued_in(conn, &thread, id).await?;
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
                    let transition =
                        Operations::insert_pending_in(conn, &op, &thread, &runtime, &ts).await?;
                    let entry = Threads::append_entry_in(
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
                    Threads::title_from_first_message_in(
                        conn, &thread, &entry.id, &prompt, author, &ts,
                    )
                    .await?;
                    Harness::record_invocation_in(
                        conn,
                        op.as_str(),
                        NewInvocation {
                            identity: &invocation,
                            settings: &settings,
                            versions: &crate::harness::InvocationVersions {
                                prompt: versions.0,
                                instructions: versions.1,
                                standards: versions.2,
                                additions: versions.3,
                            },
                            snapshot: snapshot.as_ref(),
                        },
                        &ts,
                    )
                    .await?;
                    let profile = Threads::profile_in(conn, &thread).await?;
                    Harness::remember_settings_in(
                        conn,
                        &invocation[1],
                        profile.as_deref(),
                        &settings,
                        &ts,
                    )
                    .await?;
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
    /// request. Asked before any validation: a replay starts nothing.
    pub async fn replayed_turn(
        &self,
        ctx: &CommandContext,
        thread: &ThreadId,
    ) -> Result<Option<StartedTurn>, StorageError> {
        let mut conn = self.storage.reader().acquire().await?;
        match classify(&mut conn, ctx, SCOPE, thread.as_str()).await? {
            Some(outcome) => Ok(Some(recorded(&outcome)?)),
            None => Ok(None),
        }
    }
}

impl super::TurnsStore {
    /// The prompt, instructions and standards versions of the thread's
    /// latest invocation, by its turn's order (the ordinal of the turn's own
    /// entry); `None` when it has none. Only a turn that started counts: one
    /// refused or stopped before its prompt went out delivered nothing to the
    /// session, and a turn being started now is still `Pending`.
    pub async fn latest_invocation_versions(
        &self,
        thread: &ThreadId,
    ) -> Result<Option<InvocationVersions>, StorageError> {
        let mut conn = self.storage.reader().begin().await?;
        let operations = Threads::user_operations_in(&mut conn, thread).await?;
        let started: std::collections::HashSet<_> =
            Operations::started_for_thread_in(&mut conn, thread)
                .await?
                .into_iter()
                .collect();
        for operation in operations {
            if !started.contains(&operation) {
                continue;
            }
            let row = Harness::invocation_versions_in(&mut conn, &operation).await?;
            if row.is_some() {
                return Ok(row);
            }
        }
        Ok(None)
    }
}
