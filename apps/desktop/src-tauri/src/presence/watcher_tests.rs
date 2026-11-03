use super::{
    watcher::{PresenceWatcher, Timing, Update},
    watcher_fixture::*,
};
use crate::{
    core_connection::ReadyConnection,
    presence::binding::{BindingView, ConversationTarget},
};
use std::{sync::Arc, time::Duration};
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver};
fn binding(generation: u64, thread: &str, agent: &str) -> BindingView {
    BindingView {
        revision: generation,
        generation,
        configuration_id: Some(CONFIG.into()),
        agent: Some(agent.into()),
        target: Some(ConversationTarget {
            project_id: PROJECT.into(),
            thread_id: thread.into(),
        }),
        previews: true,
    }
}
fn watch(
    api: &Fixture,
) -> (
    Arc<PresenceWatcher>,
    UnboundedReceiver<Update>,
    ReadyConnection,
) {
    let (tx, rx) = unbounded_channel();
    let watcher = PresenceWatcher::with_timing(
        Arc::new(move |update| {
            if update.is_current() {
                let _ = tx.send(update);
            }
        }),
        Timing::test(),
    );
    let connection = ReadyConnection {
        origin: api.origin.clone(),
        configuration_id: CONFIG.into(),
        runtime_id: RUNTIME.into(),
    };
    (watcher, rx, connection)
}
async fn wait(rx: &mut UnboundedReceiver<Update>, predicate: impl Fn(&Update) -> bool) -> Update {
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let value = rx.recv().await.expect("watcher must publish");
            if predicate(&value) {
                return value;
            }
        }
    })
    .await
    .expect("watcher observation deadline")
}
#[tokio::test]
async fn native_feed_subscribes_before_snapshot_suppresses_replay_and_reconciles_late_final_entry()
{
    let api = Fixture::new().await;
    api.operation(THREAD_A, "old", "Completed");
    api.entry(THREAD_A, "old", "Historical reply");
    api.data
        .sequence
        .store(1, std::sync::atomic::Ordering::SeqCst);
    api.data.replay.lock().unwrap().push(("durable".into(),serde_json::json!({"seq":1,"kind":"OperationCompleted","operation_id":"old","thread_id":THREAD_A,"payload":{}}).to_string()));
    let (watcher, mut rx, connection) = watch(&api);
    watcher.replace(binding(1, THREAD_A, "agent-a"), connection);
    let initial = wait(&mut rx, |update| update.projection.status == "Ready").await;
    assert_eq!(initial.projection.preview, None);
    let paths = api.data.requests.lock().unwrap().clone();
    assert!(
        paths
            .iter()
            .position(|path| path.contains("/api/subscribe"))
            .unwrap()
            < paths
                .iter()
                .position(|path| path.ends_with("/operations"))
                .unwrap()
    );
    api.operation(THREAD_A, "new", "Running");
    api.durable(THREAD_A, "new", "OperationStarted");
    wait(&mut rx, |update| update.projection.expression == "thinking").await;
    api.emit("delta", serde_json::json!({"op":"new","text":"live"}));
    wait(&mut rx, |update| update.projection.expression == "speaking").await;
    api.operation(THREAD_A, "new", "Completed");
    api.durable(THREAD_A, "new", "OperationCompleted");
    let completed = wait(&mut rx, |update| update.projection.expression == "success").await;
    assert_eq!(completed.projection.preview, None);
    api.entry(THREAD_A, "new", "**New** reply");
    let final_entry = wait(&mut rx, |update| update.projection.preview.is_some()).await;
    assert_eq!(final_entry.projection.preview.unwrap().text, "New reply");
    assert!(api
        .data
        .requests
        .lock()
        .unwrap()
        .iter()
        .all(|request| request.starts_with("GET ")));
    watcher.stop();
}
#[tokio::test]
async fn rapid_retarget_ignores_old_frames_and_active_agent_deletion_is_discovered_without_thread_events(
) {
    let api = Fixture::new().await;
    api.operation(THREAD_A, "op-a", "Running");
    api.operation(THREAD_B, "op-b", "Running");
    let (watcher, mut rx, connection) = watch(&api);
    watcher.replace(binding(1, THREAD_A, "agent-a"), connection.clone());
    wait(&mut rx, |update| update.projection.expression == "thinking").await;
    watcher.replace(binding(2, THREAD_B, "agent-b"), connection.clone());
    let second = wait(&mut rx, |update| {
        update
            .scope
            .as_ref()
            .is_some_and(|scope| scope.generation == 2)
            && update.projection.expression == "thinking"
    })
    .await;
    assert!(second.is_current());
    api.operation(THREAD_A, "op-a", "Completed");
    api.entry(THREAD_A, "op-a", "Old agent reply");
    api.durable(THREAD_A, "op-a", "OperationCompleted");
    tokio::time::sleep(Duration::from_millis(80)).await;
    while let Ok(update) = rx.try_recv() {
        assert_ne!(update.projection.expression, "success");
        assert_eq!(update.projection.preview, None);
    }
    assert_eq!(api.data.active.load(std::sync::atomic::Ordering::SeqCst), 1);
    watcher.replace(binding(3, THREAD_A, "agent-a"), connection);
    wait(&mut rx, |update| {
        update
            .scope
            .as_ref()
            .is_some_and(|scope| scope.generation == 3)
            && update.projection.status == "Ready"
    })
    .await;
    api.data
        .deleted
        .store(true, std::sync::atomic::Ordering::SeqCst);
    let deleted = wait(&mut rx, |update| {
        update.projection.status.contains("Agent unavailable")
    })
    .await;
    assert_eq!(deleted.projection.preview, None);
    watcher.stop();
}
#[tokio::test]
async fn lagged_and_fatal_reconnect_without_replaying_completion_or_raw_error_text() {
    let api = Fixture::new().await;
    api.operation(THREAD_A, "op-a", "Running");
    let (watcher, mut rx, connection) = watch(&api);
    watcher.replace(binding(1, THREAD_A, "agent-a"), connection);
    wait(&mut rx, |update| update.projection.expression == "thinking").await;
    api.emit("lagged", serde_json::Value::Null);
    wait(&mut rx, |update| update.projection.status == "Reconnecting").await;
    api.operation(THREAD_A, "op-a", "Completed");
    api.entry(THREAD_A, "op-a", "Missed completion");
    wait(&mut rx, |update| update.projection.status == "Ready").await;
    api.emit("fatal", serde_json::json!("private synthetic detail"));
    let reconnect = wait(&mut rx, |update| update.projection.status == "Reconnecting").await;
    assert!(!reconnect.projection.status.contains("synthetic"));
    assert_eq!(reconnect.projection.preview, None);
    watcher.stop();
}
#[tokio::test]
async fn oversized_history_and_sse_frames_fail_with_fixed_unavailable_state() {
    let api = Fixture::new().await;
    api.data
        .oversized
        .store(true, std::sync::atomic::Ordering::SeqCst);
    let (watcher, mut rx, connection) = watch(&api);
    watcher.replace(binding(1, THREAD_A, "agent-a"), connection.clone());
    let blocked = wait(&mut rx, |update| {
        update.projection.status == "Unavailable. Retry local Core in Settings."
    })
    .await;
    assert_eq!(blocked.projection.preview, None);
    api.data
        .oversized
        .store(false, std::sync::atomic::Ordering::SeqCst);
    watcher.stop();
    watcher.replace(binding(2, THREAD_A, "agent-a"), connection);
    wait(&mut rx, |update| update.projection.status == "Ready").await;
    api.emit(
        "delta",
        serde_json::json!({"op":"op-a","text":"x".repeat(1024*1024+1)}),
    );
    wait(&mut rx, |update| {
        update.projection.status == "Unavailable. Retry local Core in Settings."
    })
    .await;
    watcher.stop();
}

#[tokio::test]
async fn ten_failures_without_caught_up_require_an_explicit_retry() {
    use std::sync::atomic::Ordering;
    let api = Fixture::new().await;
    api.data.no_caught_up.store(true, Ordering::SeqCst);
    api.data
        .replay
        .lock()
        .unwrap()
        .push(("fatal".into(), "private fixture detail".into()));
    let (watcher, mut rx, connection) = watch(&api);
    watcher.replace(binding(1, THREAD_A, "agent-a"), connection.clone());
    wait(&mut rx, |update| {
        update.projection.status == "Unavailable. Retry local Core in Settings."
    })
    .await;
    let count = api
        .data
        .requests
        .lock()
        .unwrap()
        .iter()
        .filter(|path| path.contains("/api/subscribe"))
        .count();
    assert_eq!(count, 10);
    tokio::time::sleep(Duration::from_millis(120)).await;
    assert_eq!(
        api.data
            .requests
            .lock()
            .unwrap()
            .iter()
            .filter(|path| path.contains("/api/subscribe"))
            .count(),
        count
    );
    api.data.no_caught_up.store(false, Ordering::SeqCst);
    api.data.replay.lock().unwrap().clear();
    watcher.stop();
    watcher.replace(binding(1, THREAD_A, "agent-a"), connection);
    wait(&mut rx, |update| update.projection.status == "Ready").await;
    watcher.stop();
}

#[tokio::test]
async fn sse_header_handshake_has_a_deadline_without_a_total_stream_timeout() {
    let api = Fixture::new().await;
    api.data
        .stall_headers
        .store(true, std::sync::atomic::Ordering::SeqCst);
    let (watcher, mut rx, connection) = watch(&api);
    watcher.replace(binding(1, THREAD_A, "agent-a"), connection);
    wait(&mut rx, |update| update.projection.status == "Reconnecting").await;
    watcher.stop();
}

#[tokio::test]
async fn stopped_updates_are_rejected_after_binding_commit_even_before_notifier_runs() {
    let api = Fixture::new().await;
    let (watcher, mut rx, _) = watch(&api);
    let before = binding(1, THREAD_A, "agent-a");
    watcher.sync(before.clone(), None);
    let stopped = wait(&mut rx, |update| {
        update.projection.status.contains("Core unavailable")
    })
    .await;
    assert!(stopped.is_current());
    assert!(stopped.matches_binding(&before));
    assert!(!stopped.matches_binding(&binding(2, THREAD_B, "agent-b")));
    watcher.stop();
}

#[tokio::test]
async fn eof_during_a_turn_clears_transient_reply_and_does_not_celebrate_recovery() {
    let api = Fixture::new().await;
    api.operation(THREAD_A, "op-a", "Running");
    let (watcher, mut rx, connection) = watch(&api);
    watcher.replace(binding(1, THREAD_A, "agent-a"), connection);
    wait(&mut rx, |update| update.projection.expression == "thinking").await;
    api.emit("delta", serde_json::json!({"op":"op-a","text":"partial"}));
    wait(&mut rx, |update| update.projection.expression == "speaking").await;
    let _ = api.data.events.send(("__disconnect".into(), String::new()));
    wait(&mut rx, |update| update.projection.status == "Reconnecting").await;
    api.operation(THREAD_A, "op-a", "Completed");
    api.entry(THREAD_A, "op-a", "Completed while disconnected");
    let recovered = wait(&mut rx, |update| update.projection.status == "Ready").await;
    assert_eq!(recovered.projection.preview, None);
    watcher.stop();
}
#[tokio::test]
async fn missing_key_and_removed_target_are_discovered_by_read_only_liveness() {
    use std::sync::atomic::Ordering;
    let api = Fixture::new().await;
    api.operation(THREAD_A, "op-a", "Running");
    let (watcher, mut rx, connection) = watch(&api);
    watcher.replace(binding(1, THREAD_A, "agent-a"), connection);
    wait(&mut rx, |update| update.projection.expression == "thinking").await;
    api.data.key.store(false, Ordering::SeqCst);
    wait(&mut rx, |update| {
        update.projection.status == "Add a provider key in Settings."
    })
    .await;
    api.data.key.store(true, Ordering::SeqCst);
    wait(&mut rx, |update| update.projection.expression == "thinking").await;
    api.data.removed.store(true, Ordering::SeqCst);
    let removed = wait(&mut rx, |update| {
        update
            .projection
            .status
            .contains("Conversation unavailable")
    })
    .await;
    assert_eq!(removed.projection.preview, None);
    assert!(api
        .data
        .requests
        .lock()
        .unwrap()
        .iter()
        .all(|request| request.starts_with("GET ")));
    watcher.stop();
}

#[tokio::test]
async fn duplicate_sequence_does_not_reconcile_again_and_malformed_frames_fail_closed() {
    let api = Fixture::new().await;
    api.operation(THREAD_A, "op-a", "Running");
    let (watcher, mut rx, connection) = watch(&api);
    watcher.replace(binding(1, THREAD_A, "agent-a"), connection);
    wait(&mut rx, |update| update.projection.expression == "thinking").await;
    api.entry(THREAD_A, "op-a", "Ready");
    api.operation(THREAD_A, "op-a", "Completed");
    api.durable(THREAD_A, "op-a", "OperationCompleted");
    wait(&mut rx, |update| update.projection.preview.is_some()).await;
    let reads = api
        .data
        .requests
        .lock()
        .unwrap()
        .iter()
        .filter(|path| path.ends_with("/operations"))
        .count();
    api.emit("durable",serde_json::json!({"seq":1,"kind":"OperationCompleted","operation_id":"op-a","thread_id":THREAD_A}));
    tokio::time::sleep(Duration::from_millis(40)).await;
    assert_eq!(
        api.data
            .requests
            .lock()
            .unwrap()
            .iter()
            .filter(|path| path.ends_with("/operations"))
            .count(),
        reads
    );
    let _ = api
        .data
        .events
        .send(("delta".into(), "{private malformed fixture".into()));
    let invalid = wait(&mut rx, |update| {
        update.projection.status == "Unavailable. Retry local Core in Settings."
    })
    .await;
    assert_eq!(invalid.projection.preview, None);
    watcher.stop();
}

#[tokio::test]
async fn explicit_send_can_wait_for_its_scoped_feed_before_a_fast_completion() {
    let api = Fixture::new().await;
    let (watcher, mut rx, connection) = watch(&api);
    let selected = binding(1, THREAD_A, "agent-a");
    watcher.replace(selected.clone(), connection);
    watcher
        .wait_ready(&selected, Duration::from_secs(2))
        .await
        .unwrap();
    api.operation(THREAD_A, "fast", "Completed");
    api.entry(THREAD_A, "fast", "Fast reply");
    api.durable(THREAD_A, "fast", "OperationCompleted");
    let reply = wait(&mut rx, |update| update.projection.preview.is_some()).await;
    assert_eq!(reply.projection.preview.unwrap().text, "Fast reply");
    watcher.stop();
}
#[tokio::test]
async fn replayed_history_reconciles_once_at_caught_up() {
    let api = Fixture::new().await;
    api.operation(THREAD_A, "old", "Completed");
    api.data
        .sequence
        .store(40, std::sync::atomic::Ordering::SeqCst);
    for seq in 1..=40 {
        let kind = if seq % 2 == 0 {
            "ThreadEntryAppended"
        } else {
            "OperationStarted"
        };
        api.data.replay.lock().unwrap().push((
            "durable".into(),
            serde_json::json!({"seq":seq,"kind":kind,"operation_id":"old","thread_id":THREAD_A})
                .to_string(),
        ));
    }
    let (watcher, mut rx, connection) = watch(&api);
    watcher.replace(binding(1, THREAD_A, "agent-a"), connection);
    wait(&mut rx, |update| update.projection.status == "Ready").await;
    let reads = api
        .data
        .requests
        .lock()
        .unwrap()
        .iter()
        .filter(|path| path.ends_with("/operations"))
        .count();
    assert!(reads <= 2, "40 replayed frames read {reads} snapshots");
    watcher.stop();
}

fn blocked(update: &Update) -> bool {
    update.projection.expression == "blocked" && update.projection.status == "Waiting for you"
}
#[tokio::test]
async fn waiting_permissions_are_read_at_start_and_reread_on_a_project_frame() {
    let api = Fixture::new().await;
    api.permission(THREAD_B, "other", "Elsewhere");
    let (watcher, mut rx, connection) = watch(&api);
    watcher.replace(binding(1, THREAD_A, "agent-a"), connection);
    // One wait: the count and the Ready status may arrive in the same update.
    let start = wait(&mut rx, |update| {
        update.projection.status == "Ready" && update.waiting.count == 1
    })
    .await;
    assert!(
        start.waiting.bound.is_empty(),
        "another thread never blocks the bound one"
    );
    api.permission(THREAD_A, "bound", "Run curl");
    api.permissions_changed(THREAD_A, "bound", "waiting");
    let waiting = wait(&mut rx, blocked).await;
    assert_eq!(waiting.waiting.count, 2);
    assert_eq!(waiting.waiting.bound.len(), 1);
    assert_eq!(waiting.waiting.bound[0].request_id, "bound");
    assert_eq!(waiting.waiting.bound[0].title, "Run curl");
    assert_eq!(waiting.waiting.bound[0].options[0].option_id, "allow_once");
    api.settle("bound");
    api.permissions_changed(THREAD_A, "bound", "answered");
    let restored = wait(&mut rx, |update| {
        update.waiting.count == 1 && update.projection.status == "Ready"
    })
    .await;
    assert!(restored.waiting.bound.is_empty());
    let requests = api.data.requests.lock().unwrap().clone();
    assert!(requests
        .iter()
        .any(|request| request.contains(&format!("/api/projects/{PROJECT}/events"))));
    assert!(requests.iter().all(|request| request.starts_with("GET ")));
    watcher.stop();
}

const GROUP: &str = "00000000-0000-4000-8000-000000000005";
fn waiting_group(api: &Fixture, since: Option<&str>) {
    *api.data.groups.lock().unwrap() = vec![serde_json::json!({
        "id":GROUP,"needs_you_since":since,"members":[{"thread_id":THREAD_B}]
    })];
}
fn request_at(api: &Fixture, thread: &str, request: &str, asked_at: &str) {
    api.data
        .permissions
        .lock()
        .unwrap()
        .push(serde_json::json!({
            "thread_id": thread, "agent": "agent-b", "request_id": request, "title": request,
            "options": [], "asked_at": asked_at
        }));
}
fn reads(api: &Fixture, request: &str) -> usize {
    let requests = api.data.requests.lock().unwrap();
    requests
        .iter()
        .filter(|item| item.as_str() == request)
        .count()
}
fn history_loaded(api: &Fixture) -> bool {
    let requests = api.data.requests.lock().unwrap();
    requests.iter().any(|request| request.contains("/messages"))
}
/// Waits until the watcher has made `n` reads of `/api/groups`, then lets it publish.
async fn groups_read(api: &Fixture, n: usize) {
    tokio::time::timeout(Duration::from_secs(5), async {
        while reads(api, "GET /api/groups") < n {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("groups read deadline");
    tokio::time::sleep(Duration::from_millis(200)).await;
}
/// The highest alert revision among the updates published so far.
fn rung(rx: &mut UnboundedReceiver<Update>) -> u64 {
    let mut highest = 0;
    while let Ok(update) = rx.try_recv() {
        highest = highest.max(update.waiting.alert_revision);
    }
    highest
}
/// Starts a bound watcher and waits for its first caught-up reconciliation.
async fn started(api: &Fixture) -> (Arc<PresenceWatcher>, UnboundedReceiver<Update>) {
    let (watcher, rx, connection) = watch(api);
    watcher.replace(binding(1, THREAD_A, "agent-a"), connection);
    groups_read(api, 2).await;
    (watcher, rx)
}

#[tokio::test]
async fn startup_is_silent_and_a_call_during_a_drop_rings_once() {
    let api = Fixture::new().await;
    waiting_group(&api, Some("2026-11-01T10:00:00Z"));
    api.group_message(GROUP, 1, true);
    let (watcher, mut rx) = started(&api).await;
    assert_eq!(rung(&mut rx), 0);
    assert!(!history_loaded(&api));
    api.project_emit("lagged");
    wait(&mut rx, |update| update.waiting.count == 0).await;
    api.group_message(GROUP, 2, true);
    api.group_message(GROUP, 3, true);
    let resumed = wait(&mut rx, |update| update.waiting.alert_revision > 0).await;
    assert_eq!(resumed.waiting.count, 1);
    assert_eq!(resumed.waiting.alert_revision, 1);
    assert!(!history_loaded(&api));
    groups_read(&api, 4).await;
    assert!(rung(&mut rx) <= 1);
    watcher.stop();
}

#[tokio::test]
async fn a_reconnect_restores_the_count_without_an_old_tone() {
    let api = Fixture::new().await;
    waiting_group(&api, Some("2026-11-01T10:00:00Z"));
    request_at(&api, THREAD_B, "old", "2026-11-01T09:00:00Z");
    let (watcher, mut rx) = started(&api).await;
    assert_eq!(rung(&mut rx), 0);
    api.group_message(GROUP, 1, false);
    api.project_emit("lagged");
    wait(&mut rx, |update| update.waiting.count == 0).await;
    wait(&mut rx, |update| update.waiting.count == 1).await;
    groups_read(&api, 4).await;
    assert_eq!(rung(&mut rx), 0);
    watcher.stop();
}

#[tokio::test]
async fn the_group_pill_works_before_an_agent_is_bound() {
    let api = Fixture::new().await;
    waiting_group(&api, Some("2026-11-01T10:00:00Z"));
    let (watcher, mut rx, connection) = watch(&api);
    let mut unbound = binding(1, THREAD_A, "agent-a");
    unbound.agent = None;
    unbound.target = None;
    watcher.sync(unbound, Some(connection));
    let update = wait(&mut rx, |update| update.waiting.count == 1).await;
    assert_eq!(update.waiting.alert_revision, 0);
    assert!(update.waiting.bound.is_empty());
    watcher.stop();
}

#[tokio::test]
async fn a_message_to_mohammed_raises_the_waiting_state() {
    let api = Fixture::new().await;
    let (watcher, mut rx) = started(&api).await;
    assert_eq!(rung(&mut rx), 0);
    waiting_group(&api, Some("2026-11-01T10:05:00Z"));
    api.group_message(GROUP, 1, true);
    let update = wait(&mut rx, |update| update.waiting.count == 1).await;
    assert_eq!(update.waiting.alert_revision, 1);
    assert!(!history_loaded(&api));
    watcher.stop();
}

#[tokio::test]
async fn a_message_to_others_does_not() {
    let api = Fixture::new().await;
    waiting_group(&api, None);
    let (watcher, mut rx) = started(&api).await;
    api.group_message(GROUP, 1, false);
    groups_read(&api, 3).await;
    assert_eq!(rung(&mut rx), 0);
    // A later call proves the feed is live: it alone rings.
    waiting_group(&api, Some("2026-11-01T10:05:00Z"));
    api.group_message(GROUP, 2, true);
    let update = wait(&mut rx, |update| update.waiting.count == 1).await;
    assert_eq!(update.waiting.alert_revision, 1);
    watcher.stop();
}

#[tokio::test]
async fn a_group_with_needs_you_and_two_requests_counts_once() {
    let api = Fixture::new().await;
    waiting_group(&api, Some("2026-11-01T10:00:00Z"));
    request_at(&api, THREAD_B, "first", "2026-11-01T10:01:00Z");
    request_at(&api, THREAD_B, "second", "2026-11-01T10:02:00Z");
    request_at(&api, THREAD_A, "conversation", "2026-11-01T10:03:00Z");
    let (watcher, mut rx, connection) = watch(&api);
    watcher.replace(binding(1, THREAD_A, "agent-a"), connection);
    let update = wait(&mut rx, |update| update.waiting.count > 0).await;
    assert_eq!(update.waiting.count, 2);
    assert_eq!(update.waiting.bound.len(), 1);
    watcher.stop();
}

#[tokio::test]
async fn the_pill_counts_a_group_once() {
    let api = Fixture::new().await;
    waiting_group(&api, Some("2026-11-01T10:00:00Z"));
    request_at(&api, THREAD_B, "first", "2026-11-01T10:01:00Z");
    let (watcher, mut rx) = started(&api).await;
    assert_eq!(rung(&mut rx), 0);
    request_at(&api, THREAD_B, "second", "2026-11-01T10:02:00Z");
    api.permissions_changed(THREAD_B, "second", "waiting");
    api.group_message(GROUP, 1, true);
    let update = wait(&mut rx, |update| update.waiting.alert_revision > 0).await;
    assert_eq!(update.waiting.count, 1);
    watcher.stop();
}

#[tokio::test]
async fn a_new_call_rings_while_an_older_request_waits() {
    let api = Fixture::new().await;
    waiting_group(&api, None);
    request_at(&api, THREAD_B, "first", "2026-11-01T10:00:00Z");
    let (watcher, mut rx) = started(&api).await;
    assert_eq!(rung(&mut rx), 0);
    // The group's time stays at 10:00, yet the call is new.
    waiting_group(&api, Some("2026-11-01T10:05:00Z"));
    api.group_message(GROUP, 1, true);
    let update = wait(&mut rx, |update| update.waiting.alert_revision > 0).await;
    assert_eq!(update.waiting.alert_revision, 1);
    assert_eq!(update.waiting.count, 1);
    watcher.stop();
}

#[tokio::test]
async fn an_older_request_closing_rings_nothing() {
    let api = Fixture::new().await;
    waiting_group(&api, None);
    request_at(&api, THREAD_B, "first", "2026-11-01T10:00:00Z");
    request_at(&api, THREAD_B, "second", "2026-11-01T10:03:00Z");
    let (watcher, mut rx) = started(&api).await;
    assert_eq!(rung(&mut rx), 0);
    // The group's time moves forward to 10:03, a request already seen.
    let before = reads(&api, "GET /api/aylul/permissions");
    api.settle("first");
    api.permissions_changed(THREAD_B, "first", "answered");
    tokio::time::timeout(Duration::from_secs(5), async {
        while reads(&api, "GET /api/aylul/permissions") == before {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("permissions re-read deadline");
    tokio::time::sleep(Duration::from_millis(200)).await;
    assert_eq!(rung(&mut rx), 0);
    watcher.stop();
}
#[tokio::test]
async fn lagged_project_feed_recovers_like_the_thread_feed() {
    let api = Fixture::new().await;
    api.permission(THREAD_A, "first", "Run ls");
    let (watcher, mut rx, connection) = watch(&api);
    watcher.replace(binding(1, THREAD_A, "agent-a"), connection);
    wait(&mut rx, blocked).await;
    api.settle("first");
    api.permission(THREAD_A, "second", "Run pwd");
    api.project_emit("lagged");
    let recovered = wait(&mut rx, |update| {
        blocked(update)
            && update
                .waiting
                .bound
                .first()
                .is_some_and(|item| item.request_id == "second")
    })
    .await;
    assert_eq!(recovered.waiting.count, 1);
    watcher.stop();
}
#[tokio::test]
async fn waiting_lists_are_bounded() {
    let api = Fixture::new().await;
    for index in 0..12 {
        api.permission(THREAD_A, &format!("r{index}"), &"t".repeat(400));
    }
    api.data.permissions.lock().unwrap()[0]["options"] = serde_json::json!((0..12)
        .map(|index| serde_json::json!({"option_id":format!("o{index}"),"name":"n".repeat(200),"kind":"allow_once"}))
        .collect::<Vec<_>>());
    let (watcher, mut rx, connection) = watch(&api);
    watcher.replace(binding(1, THREAD_A, "agent-a"), connection);
    let update = wait(&mut rx, blocked).await;
    assert_eq!(update.waiting.count, 12);
    assert_eq!(update.waiting.bound.len(), 8);
    assert_eq!(update.waiting.bound[0].title.chars().count(), 240);
    assert_eq!(update.waiting.bound[0].options.len(), 8);
    assert_eq!(update.waiting.bound[0].options[0].name.chars().count(), 80);
    watcher.stop();
}
