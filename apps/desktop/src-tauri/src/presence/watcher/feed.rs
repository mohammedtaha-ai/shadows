//! One bound conversation's read loop: validate, subscribe, reconcile, project.
use super::{Notify, Readiness, RefreshRequest, Timing, Update};
use crate::{
    core_connection::ReadyConnection,
    presence::{
        binding::BindingView,
        core_client::{Client, Problem},
        projection::{Observation, PresenceInput, PresenceProjection, Projection, Scope},
    },
};
use serde::Deserialize;
use std::{
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::{SystemTime, UNIX_EPOCH},
};
use tokio::sync::{mpsc, watch};

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .min(u64::MAX as u128) as u64
}
struct Session {
    appearance: Option<crate::presence::CompanionAppearance>,
    scope: Scope,
    projection: Projection,
    epoch: u64,
    current: Arc<AtomicU64>,
    notify: Notify,
    ready: watch::Sender<Readiness>,
}
impl Session {
    fn apply(&mut self, observation: Observation) {
        let before = self.projection.value();
        let cleared = matches!(
            &observation,
            Observation::Unavailable(_) | Observation::Disconnected
        ) && self.appearance.take().is_some();
        let projection = self.projection.apply(
            PresenceInput {
                scope: self.scope.clone(),
                observation,
            },
            now(),
        );
        if self.current.load(Ordering::SeqCst) == self.epoch && !self.projection.reconciled() {
            self.ready.send_replace(None);
        }
        if projection != before || cleared {
            self.publish(projection)
        }
    }
    fn publish(&self, projection: PresenceProjection) {
        if self.current.load(Ordering::SeqCst) == self.epoch {
            self.ready.send_replace(
                self.projection
                    .reconciled()
                    .then(|| (self.epoch, self.scope.clone())),
            );
            (self.notify)(Update {
                guard: None,
                scope: Some(self.scope.clone()),
                projection,
                appearance: self.appearance.clone(),
                republish_only: false,
                acknowledgement: None,
                epoch: self.epoch,
                current: self.current.clone(),
            });
        }
    }
    async fn metadata(&mut self, client: &Client, binding: &BindingView) -> Result<(), Problem> {
        let appearance = client.validate_appearance(binding).await?;
        let changed = self.appearance != appearance;
        self.appearance = appearance;
        self.apply(Observation::Ready);
        if changed {
            self.publish(self.projection.value());
        }
        Ok(())
    }
    async fn explicit(&mut self, request: RefreshRequest, client: &Client, binding: &BindingView) {
        if request.epoch != self.epoch || self.current.load(Ordering::SeqCst) != self.epoch {
            let _ = request.reply.send(Err("companion_binding_changed"));
            return;
        }
        let result = self.metadata(client, binding).await;
        if let Err(problem) = result {
            self.apply(Observation::Unavailable(problem.status()));
        }
        (self.notify)(Update {
            guard: None,
            scope: Some(self.scope.clone()),
            projection: self.projection.value(),
            appearance: self.appearance.clone(),
            republish_only: false,
            acknowledgement: Some((
                request.reply,
                result.map_err(|_| "companion_core_unavailable"),
            )),
            epoch: self.epoch,
            current: self.current.clone(),
        });
    }
    async fn wait(
        &mut self,
        delay: std::time::Duration,
        refreshes: &mut mpsc::Receiver<RefreshRequest>,
        client: &Client,
        binding: &BindingView,
    ) {
        let sleep = tokio::time::sleep(delay);
        tokio::pin!(sleep);
        loop {
            tokio::select! {
                _ = &mut sleep => break,
                Some(request) = refreshes.recv() => self.explicit(request, client, binding).await,
            }
        }
    }
    async fn refresh(
        &mut self,
        client: &Client,
        thread: &str,
        cursor: i64,
        live_completed: Option<String>,
    ) -> Result<(), Problem> {
        let snapshot = client.snapshot(thread).await?;
        self.apply(Observation::Snapshot {
            cursor,
            operations: snapshot.operations,
            entries: snapshot.entries,
            live_completed,
        });
        Ok(())
    }
}
/// Where one run publishes: its epoch and the channels its updates go out on.
pub(super) struct Outlet {
    pub epoch: u64,
    pub current: Arc<AtomicU64>,
    pub notify: Notify,
    pub ready: watch::Sender<Readiness>,
}
/// Where the reconnect loop stands: the durable cursor and consecutive failures.
struct Reconnect {
    cursor: i64,
    failures: u32,
}
pub(super) async fn run(
    binding: BindingView,
    connection: ReadyConnection,
    outlet: Outlet,
    timing: Timing,
    refreshes: &mut mpsc::Receiver<RefreshRequest>,
) {
    let Outlet {
        epoch,
        current,
        notify,
        ready,
    } = outlet;
    let scope = Scope {
        generation: binding.generation,
        configuration_id: connection.configuration_id.clone(),
        runtime_id: connection.runtime_id.clone(),
        thread_id: binding
            .target
            .as_ref()
            .map(|target| target.thread_id.clone()),
    };
    let projection = Projection::new(scope.clone(), binding.agent.is_some(), binding.previews);
    let mut state = Session {
        appearance: None,
        scope,
        projection,
        epoch,
        current,
        notify,
        ready,
    };
    if binding.agent.is_none() {
        state.publish(PresenceProjection::unbound());
        return;
    }
    state.publish(state.projection.value());
    let client = match Client::new(connection, timing.snapshot) {
        Ok(client) => client,
        Err(problem) => {
            state.apply(Observation::Unavailable(problem.status()));
            return;
        }
    };
    let mut reconnect = Reconnect {
        cursor: 0,
        failures: 0,
    };
    loop {
        let outcome = match state.metadata(&client, &binding).await {
            Ok(()) => {
                state.apply(Observation::Ready);
                if let Some(target) = binding.target.as_ref() {
                    session(
                        &client,
                        &binding,
                        &target.thread_id,
                        &mut state,
                        &mut reconnect,
                        &timing,
                        refreshes,
                    )
                    .await
                } else {
                    reconnect.failures = 0;
                    state.apply(Observation::CaughtUp);
                    state
                        .wait(timing.liveness, refreshes, &client, &binding)
                        .await;
                    continue;
                }
            }
            Err(problem) => Err(problem),
        };
        let problem = outcome.err().unwrap_or(Problem::Transport);
        if problem.terminal() {
            state.apply(Observation::Unavailable(problem.status()));
            break;
        }
        if problem != Problem::Transport {
            state.apply(Observation::Unavailable(problem.status()));
            state
                .wait(timing.liveness, refreshes, &client, &binding)
                .await;
            continue;
        }
        state.apply(Observation::Disconnected);
        reconnect.failures += 1;
        if reconnect.failures >= 10 {
            state.apply(Observation::Unavailable(
                "Unavailable. Retry local Core in Settings.",
            ));
            break;
        }
        let delay = timing
            .retry
            .saturating_mul(1u32 << (reconnect.failures - 1).min(10))
            .min(timing.retry_cap);
        state.wait(delay, refreshes, &client, &binding).await;
    }
    // A stopped observer accepts explicit metadata checks without reopening its feed.
    while let Some(request) = refreshes.recv().await {
        state.explicit(request, &client, &binding).await;
    }
}
async fn session(
    client: &Client,
    binding: &BindingView,
    thread: &str,
    state: &mut Session,
    reconnect: &mut Reconnect,
    timing: &Timing,
    refreshes: &mut mpsc::Receiver<RefreshRequest>,
) -> Result<(), Problem> {
    let Reconnect { cursor, failures } = reconnect;
    let mut feed = client.subscribe(thread, *cursor).await?;
    // Subscribe before snapshot. Replay is suppressed until this stream's own caught-up.
    state.refresh(client, thread, *cursor, None).await?;
    let mut caught_up = false;
    let caught_up_deadline = tokio::time::sleep(timing.snapshot);
    tokio::pin!(caught_up_deadline);
    let mut ticks = tokio::time::interval(timing.tick);
    let mut liveness = tokio::time::interval(timing.liveness);
    liveness.tick().await;
    let mut entry_attempts = 0;
    let mut entry_at = 0;
    // Replayed durable frames only advance the cursor; one reconciliation at
    // caught-up replaces a full snapshot read per historical event.
    let mut replayed = false;
    loop {
        tokio::select! {
            Some(request) = refreshes.recv() => state.explicit(request, client, binding).await,
            _ = &mut caught_up_deadline, if !caught_up => return Err(Problem::Transport),
            _ = liveness.tick() => {
                state.metadata(client, binding).await?;
                state.apply(Observation::Ready);
            }
            _ = ticks.tick() => {
                state.apply(Observation::Tick);
                let clock = now();
                if state.projection.needs_entry(clock) && entry_attempts < 5 && clock >= entry_at {
                    state.refresh(client, thread, *cursor, None).await?;
                    entry_attempts += 1;
                    let backoff = timing.entry.saturating_mul(1u32 << entry_attempts);
                    entry_at = clock.saturating_add(backoff.as_millis() as u64);
                }
                if !state.projection.needs_entry(clock) {
                    entry_attempts = 0;
                    entry_at = 0;
                }
            }
            frame = feed.next() => {
                let frame = frame.ok_or(Problem::Transport)??;
                match frame.event.as_str() {
                    "caught-up" => {
                        let value: CaughtUp =
                            serde_json::from_str(&frame.data).map_err(|_| Problem::Invalid)?;
                        if value.seq < *cursor {
                            return Err(Problem::Invalid);
                        }
                        *cursor = value.seq;
                        if replayed {
                            state.refresh(client, thread, *cursor, None).await?;
                            replayed = false;
                        }
                        caught_up = true;
                        *failures = 0;
                        state.apply(Observation::CaughtUp);
                    }
                    "durable" => {
                        let value: Durable =
                            serde_json::from_str(&frame.data).map_err(|_| Problem::Invalid)?;
                        if value.seq <= 0 {
                            return Err(Problem::Invalid);
                        }
                        if value.seq <= *cursor {
                            continue;
                        }
                        *cursor = value.seq;
                        if value.thread_id.as_deref() != Some(thread) {
                            continue;
                        }
                        if value.kind.starts_with("Operation") || value.kind == "ThreadEntryAppended" {
                            if !caught_up {
                                replayed = true;
                                continue;
                            }
                            let completed = if value.kind == "OperationCompleted" {
                                Some(value.operation_id.ok_or(Problem::Invalid)?)
                            } else {
                                None
                            };
                            state.refresh(client, thread, *cursor, completed).await?;
                        }
                    }
                    "delta" | "subagent" => {
                        let value: Progress =
                            serde_json::from_str(&frame.data).map_err(|_| Problem::Invalid)?;
                        if !caught_up {
                            continue;
                        }
                        if state.projection.current_operation() != Some(value.op.as_str()) {
                            state.refresh(client, thread, *cursor, None).await?;
                        }
                        if frame.event == "delta" {
                            if !value.text.is_empty() {
                                state.apply(Observation::Delta(value.op));
                            }
                        } else {
                            state.apply(Observation::Tool(value.op));
                        }
                    }
                    "turn-end" => {
                        if caught_up {
                            state.refresh(client, thread, *cursor, None).await?;
                        }
                    }
                    "lagged" | "fatal" => return Err(Problem::Transport),
                    _ => {}
                }
            }
        }
    }
}
#[derive(Deserialize)]
struct CaughtUp {
    seq: i64,
}
#[derive(Deserialize)]
struct Durable {
    seq: i64,
    kind: String,
    operation_id: Option<String>,
    thread_id: Option<String>,
}
#[derive(Deserialize)]
struct Progress {
    op: String,
    #[serde(default)]
    text: String,
}
