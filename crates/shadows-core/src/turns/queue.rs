//! One job: the waiting messages of a conversation — queueing,
//! listing and removing them, sending the next, and Send now.

use std::sync::Arc;
use std::sync::atomic::Ordering;

use super::handles::SteerTarget;
use super::spawn::OnCompleted;
use super::steer::SteerRequest;
use super::{NewQueued, OperationId, QueueAnswer, SendTurn, Turns, start_command};
use crate::error::CoreError;
use crate::turns::model::{Queued, QueuedMessage, QueuedMessageId, SentNow};
use core_db::CommandContext;
use core_threads::{ThreadEntryId, ThreadId};

/// The id of the turn a `turn.queue` starts on an idle thread.
fn idle_start_id(command_id: &str) -> String {
    format!("queue:{command_id}")
}

impl Turns {
    /// A waiting message on a busy thread; on an idle one, the turn
    /// it starts. A replay answers what the first call answered.
    pub async fn queue(&self, thread: ThreadId, turn: SendTurn) -> Result<Queued, CoreError> {
        let derived = start_command(idle_start_id(&turn.command_id), &thread, &turn);
        if let Some(replay) = self.store.replayed_turn(&derived, &thread).await? {
            return Ok(Queued::Started {
                operation_id: replay.operation_id,
            });
        }
        let mut params = serde_json::json!({
            "thread_id": thread,
            "prompt": turn.prompt,
            "model": turn.model,
            "mode": turn.mode,
            "effort": turn.effort,
            "focus": turn.focus,
            "plan": turn.plan,
        });
        if !turn.attachments.is_empty() {
            params["attachments"] = serde_json::json!(turn.attachments);
        }
        let command = CommandContext::user(turn.command_id.clone(), "turn.queue", params);
        if let Some(message) = self.store.replayed_queue(&command, &thread).await? {
            return Ok(Queued::Waiting { message });
        }
        if !turn.attachments.is_empty() {
            self.images(&thread, &turn.attachments).await?;
            let opened = self.sessions.open_for_images(&thread, true).await?;
            self.sessions.require_images(&opened, true)?;
        }
        let new = || NewQueued {
            prompt: &turn.prompt,
            attachments: &turn.attachments,
            model: &turn.model,
            mode: &turn.mode,
            effort: turn.effort.as_deref(),
            focus: turn.focus.as_ref(),
            plan: turn.plan.as_ref(),
        };
        // Twice at most: a turn may take the thread between `Idle` and the start.
        for _ in 0..2 {
            match self.store.queue_message(&command, &thread, new()).await? {
                QueueAnswer::Waiting(message) => return Ok(Queued::Waiting { message }),
                QueueAnswer::Idle => {
                    let start = SendTurn {
                        command_id: idle_start_id(&turn.command_id),
                        ..turn.clone()
                    };
                    match self.send(thread.clone(), start).await {
                        Err(CoreError::Refused {
                            code: crate::ErrorCode::ThreadBusy,
                            ..
                        }) => continue,
                        other => {
                            return other.map(|operation_id| Queued::Started { operation_id });
                        }
                    }
                }
            }
        }
        Err(crate::turns::error::TurnsError::ThreadBusy.into())
    }

    /// The thread's waiting messages in `position` order.
    pub async fn queued(&self, thread: &ThreadId) -> Result<Vec<QueuedMessage>, CoreError> {
        Ok(self.store.queued_messages(thread).await?)
    }

    /// Removes a waiting message; a replay answers as the first did.
    pub async fn unqueue(
        &self,
        thread: &ThreadId,
        id: &QueuedMessageId,
        command_id: String,
    ) -> Result<(), CoreError> {
        let params = serde_json::json!({ "thread_id": thread, "queued_id": id });
        let command = CommandContext::user(command_id, "turn.unqueue", params);
        self.store.unqueue_message(&command, thread, id).await
    }
}

/// The id of the turn a waiting message starts: the watcher's start
/// and Send now's share it, so the second is a replay.
fn queued_start_id(id: &QueuedMessageId) -> String {
    format!("queued:{}", id.as_str())
}

impl Turns {
    /// After a `Completed`, the thread's next waiting message is sent
    /// by a task of its own, run to its end as `send` must be.
    pub(super) fn on_completed(&self) -> OnCompleted {
        let turns = self.clone();
        Arc::new(move |thread| {
            let turns = turns.clone();
            tokio::spawn(async move { turns.send_next(&thread).await });
        })
    }

    /// Into the running turn when the adapter takes it; as a turn of
    /// its own when none runs; refused while a Stop is pending.
    pub async fn send_now(
        &self,
        thread: &ThreadId,
        id: &QueuedMessageId,
        command_id: String,
        attachments: Vec<core_threads::AttachmentId>,
    ) -> Result<SentNow, CoreError> {
        let mut params = serde_json::json!({ "thread_id": thread, "queued_id": id });
        if !attachments.is_empty() {
            params["attachments"] = serde_json::json!(attachments);
        }
        let command = CommandContext::user(command_id, "turn.send_now", params);
        if let Some(replay) = self.store.replayed_send_now(&command, thread).await? {
            return Ok(replay);
        }
        let row = self
            .store
            .queued_message(thread, id)
            .await?
            .ok_or(crate::turns::error::TurnsError::QueuedMessageGone)?;
        self.images(thread, &attachments).await?;
        if row.attachments != attachments {
            return Err(CoreError::Refused {
                code: crate::ErrorCode::InvalidCommand,
                message: "The attachments must match the queued message in order".into(),
            });
        }
        if let Some(target) = self.handles.steer_target(thread).await {
            if target.cancel_requested.load(Ordering::SeqCst) {
                return Err(crate::turns::error::TurnsError::ThreadBusy.into());
            }
            if let Some(entry_id) = Self::steer_in(&target, &command, &row.id).await? {
                return Ok(SentNow::Steered { entry_id });
            }
        }
        let operation_id = self.send_queued(thread, &row, Some(command)).await?;
        Ok(SentNow::Started { operation_id })
    }

    /// Asks the turn's watcher to steer `row` in. `None` is "no prompt
    /// runs": the adapter said so, or the watcher is already gone.
    async fn steer_in(
        target: &SteerTarget,
        command: &CommandContext,
        row: &QueuedMessageId,
    ) -> Result<Option<ThreadEntryId>, CoreError> {
        let (reply, answer) = tokio::sync::oneshot::channel();
        let request = SteerRequest {
            command: command.clone(),
            row: row.clone(),
            reply,
        };
        if target.steer.send(request).is_err() {
            return Ok(None);
        }
        answer.await.unwrap_or(Ok(None))
    }

    /// Starts `row` as a turn, taking it from the queue in the turn's own
    /// transaction.
    pub(crate) async fn send_queued(
        &self,
        thread: &ThreadId,
        row: &QueuedMessage,
        also: Option<CommandContext>,
    ) -> Result<OperationId, CoreError> {
        let turn = SendTurn {
            command_id: queued_start_id(&row.id),
            prompt: row.prompt.clone(),
            attachments: row.attachments.clone(),
            model: row.model.clone(),
            mode: row.mode.clone(),
            effort: row.effort.clone(),
            focus: row.focus.clone(),
            plan: row.plan.clone(),
            client_tab: None,
        };
        let dequeue = Some((row.id.clone(), also));
        self.send_with(thread.clone(), turn, dequeue).await
    }

    /// The first waiting message, unless it carries an error and so
    /// waits for the person. `ThreadBusy` is not a failure: the turn that took
    /// the thread sends it when it completes. A gone row was sent by Send now.
    pub(crate) async fn send_next(&self, thread: &ThreadId) {
        let first = match self.store.queued_messages(thread).await {
            Ok(list) => list.into_iter().next(),
            Err(error) => {
                tracing::error!(%error, thread_id = %thread, "queue.read_failed");
                return;
            }
        };
        let Some(row) = first.filter(|row| row.last_error.is_none()) else {
            return;
        };
        match self.send_queued(thread, &row, None).await {
            Ok(_) => {}
            Err(CoreError::Refused {
                code: crate::ErrorCode::ThreadBusy | crate::ErrorCode::QueuedMessageGone,
                ..
            }) => {}
            Err(error) => {
                let message = error.to_string();
                if let Err(e) = self.store.fail_queued(thread, &row.id, &message).await {
                    tracing::error!(error = %e, "queue.fail_record_failed");
                }
            }
        }
    }
}
