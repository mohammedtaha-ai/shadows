//! One job: a Send now inside the turn's watcher.
//!
//! The watcher sends the steering request itself, and reads no events until it
//! has the answer: a chunk of the steered reply can reach Shadows before the
//! answer does, and it must not be written before the steered message.

use tokio::sync::oneshot;

use super::{
    entries::Collector,
    model::QueuedMessageId,
    prompt::Prompt,
    turn::{PlannerTurn, TurnWatch},
};
use crate::{db::CommandContext, error::CoreError, threads::ThreadEntryId};
use shadows_agent::acp::Steer;

/// A Send now for the running turn: `Ok(Some(entry))` is the message steered
/// in, `Ok(None)` is the adapter saying no prompt runs.
pub(crate) type SteerAnswer = Result<Option<ThreadEntryId>, CoreError>;

pub(crate) struct SteerRequest {
    pub command: CommandContext,
    pub row: QueuedMessageId,
    pub reply: oneshot::Sender<SteerAnswer>,
}

/// Steers, and on `injected` writes the text streamed so far, then the
/// steered message with its row taken. A failed steer writes nothing.
impl TurnWatch {
    pub(super) async fn steer(&self, collector: &mut Collector, r: SteerRequest) {
        let _ = r
            .reply
            .send(attempt(self, collector, &r.command, &r.row).await);
    }
}

async fn attempt(
    w: &TurnWatch,
    collector: &mut Collector,
    command: &CommandContext,
    row: &QueuedMessageId,
) -> SteerAnswer {
    let store = super::store::TurnsStore::with_storage(w.storage.clone());
    // The watcher steers one message at a time, so a row taken since the
    // caller read it is seen here, before a second copy reaches the adapter.
    let Some(queued) = store.queued_message(&w.thread_id, row).await? else {
        return Err(crate::turns::error::TurnsError::QueuedMessageGone.into());
    };
    let connection = w.opened.connection();
    let stage = super::Turns::stage_line(&w.storage, &w.thread_id).await?;
    let prompt = Prompt::steered(&queued.prompt, &stage);
    match connection.steer(&w.opened.session_id, &prompt).await {
        Ok(Steer::Injected) => {
            PlannerTurn::persist(w, collector.cut()).await;
            let entry = store
                .steered_entry(command, &w.thread_id, &w.op_id, row, &queued.prompt)
                .await?;
            Ok(Some(entry))
        }
        Ok(Steer::PromptRequired) => Ok(None),
        Err(error) => {
            let reason = format!("the message did not reach the turn: {error}");
            store.fail_queued(&w.thread_id, row, &reason).await?;
            Err(CoreError::HarnessStartFailed(reason))
        }
    }
}
