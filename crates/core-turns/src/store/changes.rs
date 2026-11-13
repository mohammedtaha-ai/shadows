//! One job: the Planner's project changes as stored — a noted change's entry,
//! and the unnoted ones' entry with the notice that asks about them.

use core_db::{Actor, CommandContext, Storage};
use core_operations::Operations;
use core_threads::{NewThreadEntry, ThreadEntryKind, ThreadId, ThreadKind, Threads};

use super::queue::QueuedFields;
use crate::TurnsError;
use crate::model::{Notice, OperationId, QueuedMessageId};

/// The settings the notice's turn runs under: model, mode and effort.
pub(crate) type NoticeSettings = (String, String, Option<String>);

/// A change's card: what the conversation draws.
fn card(paths: &[String], reason: Option<&str>, noted: bool) -> serde_json::Value {
    serde_json::json!({ "paths": paths, "reason": reason, "noted": noted })
}

fn planner() -> Actor {
    Actor {
        kind: "Agent".into(),
        id: "Planner".into(),
    }
}

impl super::TurnsStore {
    /// The project `thread` belongs to.
    pub async fn thread_project(
        &self,
        thread: &ThreadId,
    ) -> Result<core_projects::ProjectId, TurnsError> {
        let mut conn = self.storage.reader().acquire().await?;
        Ok(Threads::project_in(&mut conn, thread).await?)
    }

    /// Records the Planner's note in its thread, on its running turn; an
    /// executor's thread notes nothing.
    pub async fn note_change(
        &self,
        thread: &ThreadId,
        paths: &[String],
        reason: &str,
    ) -> Result<(), TurnsError> {
        let (thread, paths, reason) = (thread.clone(), paths.to_vec(), reason.to_owned());
        let ts = Storage::now();
        self.storage
            .write_txn_as(move |conn| {
                Box::pin(async move {
                    let context = Threads::turn_context_in(conn, &thread).await?;
                    if context.kind == ThreadKind::Executor {
                        return Err(TurnsError::InvalidCommand(
                            "only the Planner notes its changes".into(),
                        ));
                    }
                    let operation = Operations::pending_or_running_in(conn, &thread).await?;
                    let card = card(&paths, Some(&reason), true);
                    Threads::append_entry_in(
                        conn,
                        &thread,
                        NewThreadEntry {
                            kind: ThreadEntryKind::ProjectChange,
                            author: planner(),
                            body: &reason,
                            refs: &[],
                            card: Some(&card),
                            operation_id: operation.as_ref().map(|op| op.as_str()),
                        },
                        &ts,
                    )
                    .await?;
                    Ok(())
                })
            })
            .await
    }

    /// In one transaction: the unnoted paths' entry on `operation`, and the
    /// notice that asks why, waiting in the thread's queue. Nothing for an
    /// executor's or AYLUL's thread, or a removed one. Answers whether it wrote.
    pub async fn record_unnoted(
        &self,
        thread: &ThreadId,
        operation: &OperationId,
        paths: &[String],
        settings: NoticeSettings,
    ) -> Result<bool, TurnsError> {
        let (thread, operation, paths) = (thread.clone(), operation.clone(), paths.to_vec());
        let ts = Storage::now();
        self.storage
            .write_txn_as(move |conn| {
                Box::pin(async move {
                    if !Threads::is_live_in(conn, &thread).await? {
                        return Ok(false);
                    }
                    let context = Threads::turn_context_in(conn, &thread).await?;
                    if context.kind == ThreadKind::Executor
                        || context.harness == shadows_agent::policy::AYLUL
                    {
                        return Ok(false);
                    }
                    let notice = Notice {
                        executor: None,
                        card: serde_json::json!({ "kind": "project_change", "paths": paths }),
                    };
                    let card = card(&paths, None, false);
                    let listed = paths.join(", ");
                    Threads::append_entry_in(
                        conn,
                        &thread,
                        NewThreadEntry {
                            kind: ThreadEntryKind::ProjectChange,
                            author: notice.actor(),
                            body: &format!("Changed with no reason given: {listed}"),
                            refs: &[],
                            card: Some(&card),
                            operation_id: Some(operation.as_str()),
                        },
                        &ts,
                    )
                    .await?;
                    let (model, mode, effort) = settings;
                    let prompt = format!(
                        "You changed these project files in your last turn without \
                         project_change_note: {listed}. Say why with project_change_note."
                    );
                    let params = serde_json::json!({
                        "thread_id": thread,
                        "operation_id": operation,
                        "paths": paths,
                    });
                    let command = crate::in_name_of(
                        CommandContext::user(
                            format!("project_change:{}", operation.as_str()),
                            "turn.queue",
                            params,
                        ),
                        Some(&notice),
                    );
                    let fields: QueuedFields = (prompt, model, mode, effort, None, None);
                    let notice = serde_json::to_string(&notice)?;
                    Self::insert_queued_in(
                        conn,
                        &command,
                        &thread,
                        &QueuedMessageId::generate(),
                        &fields,
                        &[],
                        Some(&notice),
                        &ts,
                    )
                    .await?;
                    Ok(true)
                })
            })
            .await
    }
}
