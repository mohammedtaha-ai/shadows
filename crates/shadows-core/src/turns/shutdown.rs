//! One job: bring every turn this runtime owns to a terminal state at shutdown,
//! and record only the stop kind that is true.
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
//! next startup, never `Cancelled`.

use std::future::Future;
use std::sync::Arc;
use std::time::Duration;

use super::handles::LiveHandles;
use super::turn::{PlannerTurn, StopOutcome};
use crate::db::{Actor, StorageError};
use crate::harness::Sessions;
use crate::runtime::Runtime;
use crate::runtime::StopKind;

/// Stops every live turn, waits — at most `confirm_within`, and never past
/// `escalate` — until every operation this runtime owns is terminal, then
/// records the stop. Answers the kind actually recorded.
///
/// `escalate` is the second stop signal. When it fires the runtime stops
/// waiting for confirmation and records `Escalated`; a future that
/// never completes means there is no second signal to wait for.
impl PlannerTurn {
    pub async fn shut_down(
        runtime: Arc<Runtime>,
        storage: Arc<crate::db::Storage>,
        operations: crate::operations::Operations,
        handles: Arc<LiveHandles>,
        sessions: Arc<Sessions>,
        confirm_within: Duration,
        escalate: impl Future<Output = ()>,
    ) -> Result<StopKind, StorageError> {
        let confirmed = tokio::select! {
            confirmed = tokio::time::timeout(
                confirm_within,
                terminate_all(&runtime, &storage, &operations, &handles, &sessions),
            ) => {
                match confirmed {
                    Ok(confirmed) => confirmed,
                    Err(_elapsed) => {
                        log_unconfirmed(
                            &runtime,
                            &operations,
                            "shutdown.unconfirmed: the confirmation bound ran out",
                        )
                        .await;
                        false
                    }
                }
            }
            () = escalate => {
                log_unconfirmed(
                    &runtime, &operations, "shutdown.escalated: a second stop signal",
                ).await;
                false
            }
        };
        record(&runtime, &operations, confirmed).await
    }
}

/// Closes the registry, stops every turn it held, and waits for the durable
/// record to say every owned operation is terminal. `false` as soon as one
/// termination cannot be confirmed: waiting for it would only run out the
/// bound over a tree that is known to be alive.
async fn terminate_all(
    runtime: &Arc<Runtime>,
    storage: &Arc<crate::db::Storage>,
    operations: &crate::operations::Operations,
    handles: &Arc<LiveHandles>,
    sessions: &Arc<Sessions>,
) -> bool {
    let live = handles.close().await;
    tracing::info!(live = live.len(), "shutdown.begin");
    let mut confirmed = true;
    for op_id in live {
        match PlannerTurn::stop(
            storage.clone(),
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
    match all_owned_terminal(runtime, storage, operations).await {
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
async fn all_owned_terminal(
    runtime: &Runtime,
    storage: &crate::db::Storage,
    operations: &crate::operations::Operations,
) -> Result<(), StorageError> {
    let mut committed = storage.watch_committed();
    loop {
        let live = operations
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
async fn log_unconfirmed(
    runtime: &Runtime,
    operations: &crate::operations::Operations,
    message: &'static str,
) {
    match operations
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
async fn record(
    runtime: &Runtime,
    operations: &crate::operations::Operations,
    confirmed: bool,
) -> Result<StopKind, StorageError> {
    if confirmed {
        match operations
            .stop_runtime(&runtime.instance_id, StopKind::Graceful)
            .await
        {
            Ok(()) => return Ok(StopKind::Graceful),
            Err(error @ StorageError::TransitionConflict { .. }) => {
                tracing::error!(%error, "shutdown.graceful_refused");
            }
            Err(error) => return Err(error),
        }
    }
    operations
        .stop_runtime(&runtime.instance_id, StopKind::Escalated)
        .await?;
    Ok(StopKind::Escalated)
}
