//! One job: giving the titles a session's harness sends to its thread
//! (spec §4.2, §12.3).
//!
//! The adapter generates a title after the turn has answered, so it arrives
//! when no turn reads the thread's events. It is taken off the connection
//! here, in the connection's own dispatch, and written by one task per
//! adapter, in the order the harness sent them. Threads' store owns the rule.

use std::sync::Arc;

use tokio::sync::mpsc;

use crate::db::Storage;
use crate::threads::ThreadId;
use shadows_agent::events::HarnessEvent;

/// Hands every event but a title on to `next`. The task ends when the
/// connection drops what this returns.
pub(super) fn keep_titles(
    storage: Arc<Storage>,
    thread: ThreadId,
    next: impl Fn(HarnessEvent) + Clone + Send + Sync + 'static,
) -> impl Fn(HarnessEvent) + Clone + Send + Sync + 'static {
    let (tx, mut rx) = mpsc::unbounded_channel::<String>();
    tokio::spawn(async move {
        while let Some(title) = rx.recv().await {
            match storage.title_from_harness(&thread, &title).await {
                Ok(changed) => {
                    tracing::debug!(thread_id = %thread, changed, "thread.harness_title")
                }
                Err(error) => {
                    tracing::warn!(thread_id = %thread, %error, "thread.harness_title_failed")
                }
            }
        }
    });
    move |event| match event {
        HarnessEvent::SessionTitle { title } => {
            let _ = tx.send(title);
        }
        other => next(other),
    }
}
