//! One bound conversation's read loop: validate, subscribe, reconcile, project.
use super::{Notify, Readiness, RefreshRequest, Timing, Update};
use crate::windows::companion::{SurfaceWaiting, SurfaceWaitingOption, SurfaceWaitingRequest};
use crate::{
    core_connection::ReadyConnection,
    presence::{
        binding::BindingView,
        core_client::{Client, Feed, Problem, WaitingGroup, WaitingPermission},
        projection::{Observation, PresenceInput, PresenceProjection, Projection, Scope},
        waiting_items,
    },
};
use serde::Deserialize;
use std::{
    collections::{HashMap, HashSet},
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
struct Session<'a> {
    waiting: Waiting<'a>,
    appearance: Option<crate::presence::CompanionAppearance>,
    scope: Scope,
    projection: Projection,
    epoch: u64,
    current: Arc<AtomicU64>,
    notify: Notify,
    ready: watch::Sender<Readiness>,
}
impl Session<'_> {
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
                waiting: self.waiting.view(self.scope.thread_id.as_deref()),
                acknowledgement: None,
                epoch: self.epoch,
                current: self.current.clone(),
            });
        }
    }
    async fn metadata(&mut self, client: &Client, binding: &BindingView) -> Result<(), Problem> {
        if self.waiting.feed.is_none() && self.waiting.connect(client, binding).await {
            self.waiting_changed();
        }
        let appearance = if binding.agent.is_some() {
            client.validate_appearance(binding).await?
        } else {
            None
        };
        let changed = self.appearance != appearance;
        self.appearance = appearance;
        self.apply(Observation::Ready);
        if changed {
            self.publish(self.projection.value());
        }
        Ok(())
    }
    /// The bound conversation's waiting request decides the expression; the count rides along.
    fn waiting_changed(&mut self) {
        let blocked = !self
            .waiting
            .view(self.scope.thread_id.as_deref())
            .bound
            .is_empty();
        self.apply(Observation::Waiting(blocked));
        self.publish(self.projection.value());
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
            waiting: self.waiting.view(self.scope.thread_id.as_deref()),
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
                changed = self.waiting.change(client) => {
                    if changed {
                        self.waiting_changed();
                    }
                }
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
/// Alert delivery survives a replacement of the bound conversation's runner.
#[derive(Default)]
pub(super) struct AlertHistory {
    configuration: String,
    cursor: i64,
    initialized: bool,
    permissions: HashSet<(String, String)>,
    calls: HashMap<String, i64>,
    pending_tone: bool,
    revision: u64,
}
impl AlertHistory {
    fn scope(&mut self, configuration: &str) {
        if self.configuration != configuration {
            *self = Self {
                configuration: configuration.into(),
                revision: self.revision,
                ..Self::default()
            };
        }
    }
}
enum RefreshWaiting {
    Permissions,
    Message(GroupMessageEvent),
    CaughtUp,
}
#[derive(Deserialize)]
struct GroupMessageEvent {
    group_id: String,
    seq: i64,
}

/// One project feed reconciles both kinds of waiting items.
struct Waiting<'a> {
    history: &'a mut AlertHistory,
    project: Option<String>,
    feed: Option<Feed>,
    failures: u32,
    requests: Vec<WaitingPermission>,
    groups: Vec<WaitingGroup>,
    replaying: bool,
    // select! can cancel a lookup after its frame was already consumed.
    owed: Option<(i64, RefreshWaiting)>,
}
impl<'a> Waiting<'a> {
    const LIST: usize = 256;
    const FAILURES: u32 = 10;
    fn new(history: &'a mut AlertHistory) -> Self {
        Self {
            history,
            project: None,
            feed: None,
            failures: 0,
            requests: vec![],
            groups: vec![],
            replaying: true,
            owed: None,
        }
    }
    /// Subscribe before the read, so a change between them is not lost. True when the requests changed.
    async fn connect(&mut self, client: &Client, binding: &BindingView) -> bool {
        if self.failures >= Self::FAILURES {
            return false;
        }
        let attempt = async {
            let project = match self.project.clone() {
                Some(project) => project,
                None => client.project_id(binding).await?,
            };
            let feed = client
                .subscribe_project(&project, self.history.cursor)
                .await?;
            let groups = client.groups().await?;
            let requests = client.permissions().await?;
            Ok::<_, Problem>((project, feed, groups, requests))
        }
        .await;
        match attempt {
            Ok((project, feed, groups, requests)) => {
                self.project = Some(project);
                self.feed = Some(feed);
                self.replaying = true;
                self.owed = None;
                self.groups = groups;
                self.replace(requests);
                true
            }
            Err(_) => self.lose(),
        }
    }
    fn replace(&mut self, mut requests: Vec<WaitingPermission>) -> bool {
        requests.truncate(Self::LIST);
        for request in &requests {
            let new = self
                .history
                .permissions
                .insert((request.thread_id.clone(), request.request_id.clone()));
            self.history.pending_tone |= new && self.history.initialized;
        }
        let changed = self.requests != requests;
        self.requests = requests;
        changed
    }
    fn lose(&mut self) -> bool {
        self.feed = None;
        self.failures += 1;
        self.owed = None;
        let had_groups = !self.groups.is_empty();
        self.groups.clear();
        self.replace(vec![]) || had_groups
    }
    /// Waits for the next project frame; true when the requests changed.
    ///
    /// Callers race this against timers in `select!`, so the re-read is owed
    /// across a cancellation: the frame is already consumed and would be lost.
    async fn change(&mut self, client: &Client) -> bool {
        if self.owed.is_none() {
            if let Some(changed) = self.frame().await {
                return changed;
            }
        }
        let result = self.refresh_owed(client).await;
        match result {
            Ok(changed) => {
                if !self.replaying && std::mem::take(&mut self.history.pending_tone) {
                    self.history.revision += 1;
                    true
                } else {
                    changed
                }
            }
            Err(_) => self.lose(),
        }
    }
    async fn refresh_owed(&mut self, client: &Client) -> Result<bool, Problem> {
        let (cursor, action) = self.owed.as_ref().expect("a consumed frame is owed");
        let cursor = *cursor;
        let changed = match action {
            RefreshWaiting::Permissions => self.replace(client.permissions().await?),
            RefreshWaiting::Message(event) => {
                let groups = client.groups().await?;
                if self
                    .history
                    .calls
                    .get(&event.group_id)
                    .is_none_or(|seq| event.seq > *seq)
                    && client
                        .group_message_calls_person(&event.group_id, event.seq)
                        .await?
                {
                    self.history.calls.insert(event.group_id.clone(), event.seq);
                    self.history.pending_tone = true;
                }
                let changed = self.groups != groups;
                self.groups = groups;
                changed
            }
            RefreshWaiting::CaughtUp => {
                let groups = client.groups().await?;
                let requests = client.permissions().await?;
                let changed = self.groups != groups;
                self.groups = groups;
                let changed = self.replace(requests) || changed;
                self.history.initialized = true;
                self.replaying = false;
                self.failures = 0;
                changed
            }
        };
        self.history.cursor = self.history.cursor.max(cursor);
        self.owed = None;
        Ok(changed)
    }
    /// The next frame's effect, or `None` when it asks for a re-read.
    async fn frame(&mut self) -> Option<bool> {
        let Some(feed) = self.feed.as_mut() else {
            return std::future::pending().await;
        };
        let Some(Ok(frame)) = feed.next().await else {
            return Some(self.lose());
        };
        match frame.event.as_str() {
            "caught-up" => match serde_json::from_str::<CaughtUp>(&frame.data) {
                Ok(value) => {
                    self.owed = Some((value.seq, RefreshWaiting::CaughtUp));
                    None
                }
                Err(_) => Some(self.lose()),
            },
            "durable" => {
                let Ok(value) = serde_json::from_str::<Durable>(&frame.data) else {
                    return Some(self.lose());
                };
                if value.seq <= self.history.cursor {
                    return Some(false);
                }
                let action = match value.kind.as_str() {
                    "PermissionsChanged" => RefreshWaiting::Permissions,
                    "MessageAdded" if self.history.initialized => {
                        let Ok(event) = serde_json::from_value(value.payload) else {
                            return Some(self.lose());
                        };
                        RefreshWaiting::Message(event)
                    }
                    // The caught-up snapshot covers the initial replay without loading old messages.
                    _ => {
                        self.history.cursor = value.seq;
                        return Some(false);
                    }
                };
                self.owed = Some((value.seq, action));
                None
            }
            "lagged" | "fatal" => Some(self.lose()),
            _ => Some(false),
        }
    }
    /// What the companion shows: every request counted, the bound thread's listed within bounds.
    fn view(&self, thread: Option<&str>) -> SurfaceWaiting {
        SurfaceWaiting {
            count: waiting_items::items(&self.groups, &self.requests).len() as u32,
            alert_revision: self.history.revision,
            bound: self
                .requests
                .iter()
                .filter(|request| Some(request.thread_id.as_str()) == thread)
                .take(8)
                .map(|request| SurfaceWaitingRequest {
                    thread_id: request.thread_id.clone(),
                    request_id: request.request_id.clone(),
                    title: request.title.chars().take(240).collect(),
                    options: request
                        .options
                        .iter()
                        .take(8)
                        .map(|option| SurfaceWaitingOption {
                            option_id: option.option_id.clone(),
                            name: option.name.chars().take(80).collect(),
                            kind: option.kind.clone(),
                        })
                        .collect(),
                })
                .collect(),
        }
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
    alert_history: &mut AlertHistory,
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
    alert_history.scope(&connection.configuration_id);
    let mut state = Session {
        waiting: Waiting::new(alert_history),
        appearance: None,
        scope,
        projection,
        epoch,
        current,
        notify,
        ready,
    };
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
    state: &mut Session<'_>,
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
            changed = state.waiting.change(client) => {
                if changed {
                    state.waiting_changed();
                }
            }
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
    #[serde(default)]
    payload: serde_json::Value,
}
#[derive(Deserialize)]
struct Progress {
    op: String,
    #[serde(default)]
    text: String,
}
