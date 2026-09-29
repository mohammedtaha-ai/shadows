//! One job: bring every turn this runtime owns to a terminal state at shutdown,
//! and record only the stop kind that is true (spec §8.5).
//!
//! Shutdown reuses the cancellation path; there is no drain mode. What it adds
//! is the part a single `stop` cannot see: that *every* operation this runtime
//! owns is terminal before `Graceful` is written. `stop` returning is not that
//! fact. In three of its outcomes the reader writes the terminal state, not
//! `stop`, and a turn still spawning or still `Pending` is in no snapshot of the
//! registry at all. So the claim is checked where it is recorded — the durable
//! rows — and storage itself refuses `Graceful` over a non-terminal one.
//!
//! Anything short of that records `Escalated`, which claims nothing: a failed
//! termination, a storage error, the confirmation bound running out, or a
//! second stop signal. Operations left non-terminal become `Interrupted` at the
//! next startup (§8.6), never `Cancelled`.

use std::future::Future;
use std::sync::Arc;
use std::time::Duration;

use super::handles::LiveHandles;
use super::turn::{PlannerTurn, StopOutcome};
use crate::db::StorageError;
use crate::events::Actor;
use crate::harness::Sessions;
use crate::runtime::Runtime;
use crate::runtime::StopKind;

/// Stops every live turn, waits — at most `confirm_within`, and never past
/// `escalate` — until every operation this runtime owns is terminal, then
/// records the stop. Answers the kind actually recorded.
///
/// `escalate` is the second stop signal. When it fires the runtime stops
/// waiting for confirmation and records `Escalated` (§8.5); a future that
/// never completes means there is no second signal to wait for.
pub async fn shut_down(
    runtime: Arc<Runtime>,
    handles: Arc<LiveHandles>,
    sessions: Arc<Sessions>,
    confirm_within: Duration,
    escalate: impl Future<Output = ()>,
) -> Result<StopKind, StorageError> {
    let confirmed = tokio::select! {
        confirmed = tokio::time::timeout(confirm_within, terminate_all(&runtime, &handles, &sessions)) => {
            match confirmed {
                Ok(confirmed) => confirmed,
                Err(_elapsed) => {
                    log_unconfirmed(&runtime, "shutdown.unconfirmed: the confirmation bound ran out").await;
                    false
                }
            }
        }
        () = escalate => {
            log_unconfirmed(&runtime, "shutdown.escalated: a second stop signal").await;
            false
        }
    };
    record(&runtime, confirmed).await
}

/// Closes the registry, stops every turn it held, and waits for the durable
/// record to say every owned operation is terminal. `false` as soon as one
/// termination cannot be confirmed: waiting for it would only run out the
/// bound over a tree that is known to be alive.
async fn terminate_all(
    runtime: &Arc<Runtime>,
    handles: &Arc<LiveHandles>,
    sessions: &Arc<Sessions>,
) -> bool {
    let live = handles.close().await;
    tracing::info!(live = live.len(), "shutdown.begin");
    let mut confirmed = true;
    for op_id in live {
        match PlannerTurn::stop(
            runtime.clone(),
            handles.clone(),
            sessions.clone(),
            &op_id,
            Actor::system(),
        )
        .await
        {
            Ok(StopOutcome::TerminationFailed) => {
                tracing::error!(operation_id = %op_id, "shutdown.termination_failed");
                confirmed = false;
            }
            Ok(_) => {}
            Err(error) => {
                tracing::error!(operation_id = %op_id, %error, "shutdown.stop_failed");
                confirmed = false;
            }
        }
    }
    if let Err(error) = sessions.close_all().await {
        tracing::error!(%error, "shutdown.sessions_close_failed");
    }
    if !confirmed {
        return false;
    }
    match all_owned_terminal(runtime).await {
        Ok(()) => true,
        Err(error) => {
            tracing::error!(%error, "shutdown.confirmation_unreadable");
            false
        }
    }
}

/// Returns once no operation this runtime owns is `Pending` or `Running`.
/// Every terminal transition commits a journal event, so the committed-sequence
/// signal is the wake-up. The receiver is taken before the first read and
/// `changed` marks each value seen as it returns, so a transition that commits
/// during a read shows as a change rather than falling between two reads.
async fn all_owned_terminal(runtime: &Runtime) -> Result<(), StorageError> {
    let mut committed = runtime.storage.watch_committed();
    loop {
        let live = runtime
            .storage
            .non_terminal_operations_owned_by(&runtime.instance_id)
            .await?;
        if live.is_empty() {
            return Ok(());
        }
        if committed.changed().await.is_err() {
            return Err(StorageError::Unavailable(
                "the committed-write signal closed while shutdown waited on it".into(),
            ));
        }
    }
}

/// Names the operations a shutdown is leaving non-terminal, so the log says
/// which turns the next startup will record as `Interrupted`.
async fn log_unconfirmed(runtime: &Runtime, message: &'static str) {
    match runtime
        .storage
        .non_terminal_operations_owned_by(&runtime.instance_id)
        .await
    {
        Ok(live) => {
            let live: Vec<&str> = live.iter().map(|op| op.as_str()).collect();
            tracing::warn!(unconfirmed = ?live, "{message}");
        }
        Err(error) => tracing::warn!(%error, "{message}"),
    }
}

/// Writes `Graceful` when every termination was confirmed — and storage checks
/// that claim again inside the write, so a turn that slipped past the registry
/// still cannot be covered by it. Anything else is `Escalated`.
async fn record(runtime: &Runtime, confirmed: bool) -> Result<StopKind, StorageError> {
    if confirmed {
        match runtime.stop(StopKind::Graceful).await {
            Ok(()) => return Ok(StopKind::Graceful),
            Err(error @ StorageError::TransitionConflict { .. }) => {
                tracing::error!(%error, "shutdown.graceful_refused");
            }
            Err(error) => return Err(error),
        }
    }
    runtime.stop(StopKind::Escalated).await?;
    Ok(StopKind::Escalated)
}
