//! One job: the waiting messages of a conversation (spec §20) — queueing,
//! listing and removing them, sending the next, and Send now.

use super::{NewQueued, QueueAnswer, SendTurn, Turns, start_command};
use crate::app::user_command;
use crate::db::StorageError;
use crate::error::CoreError;
use crate::threads::ThreadId;
use crate::turns::model::{Queued, QueuedMessage, QueuedMessageId};

/// The id of the turn a `turn.queue` starts on an idle thread (§20.2).
fn idle_start_id(command_id: &str) -> String {
    format!("queue:{command_id}")
}

impl Turns {
    /// §20.2: a waiting message on a busy thread; on an idle one, the turn
    /// it starts. A replay answers what the first call answered.
    pub async fn queue(&self, thread: ThreadId, turn: SendTurn) -> Result<Queued, CoreError> {
        let derived = start_command(idle_start_id(&turn.command_id), &thread, &turn);
        if let Some(replay) = self.storage.replayed_turn(&derived, &thread).await? {
            return Ok(Queued::Started {
                operation_id: replay.operation_id,
            });
        }
        let params = serde_json::json!({
            "thread_id": thread,
            "prompt": turn.prompt,
            "model": turn.model,
            "mode": turn.mode,
            "effort": turn.effort,
            "focus": turn.focus,
            "plan": turn.plan,
        });
        let command = user_command(turn.command_id.clone(), "turn.queue", params);
        let new = || NewQueued {
            prompt: &turn.prompt,
            model: &turn.model,
            mode: &turn.mode,
            effort: turn.effort.as_deref(),
            focus: turn.focus.as_ref(),
            plan: turn.plan.as_ref(),
        };
        // Twice at most: a turn may take the thread between `Idle` and the start.
        for _ in 0..2 {
            match self.storage.queue_message(&command, &thread, new()).await? {
                QueueAnswer::Waiting(message) => return Ok(Queued::Waiting { message }),
                QueueAnswer::Idle => {
                    let start = SendTurn {
                        command_id: idle_start_id(&turn.command_id),
                        ..turn.clone()
                    };
                    match self.send(thread.clone(), start).await {
                        Err(CoreError::Storage(StorageError::ThreadBusy)) => continue,
                        other => {
                            return other.map(|operation_id| Queued::Started { operation_id });
                        }
                    }
                }
            }
        }
        Err(StorageError::ThreadBusy.into())
    }

    /// §20.5: the thread's waiting messages in `position` order.
    pub async fn queued(&self, thread: &ThreadId) -> Result<Vec<QueuedMessage>, CoreError> {
        Ok(self.storage.queued_messages(thread).await?)
    }

    /// §20.5: removes a waiting message; a replay answers as the first did.
    pub async fn unqueue(
        &self,
        thread: &ThreadId,
        id: &QueuedMessageId,
        command_id: String,
    ) -> Result<(), CoreError> {
        let params = serde_json::json!({ "thread_id": thread, "queued_id": id });
        let command = user_command(command_id, "turn.unqueue", params);
        Ok(self.storage.unqueue_message(&command, thread, id).await?)
    }
}
