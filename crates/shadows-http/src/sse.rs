//! One job: the durable-replay-then-live stream (spec §2.10).

use std::convert::Infallible;

use axum::extract::{Query, State};
use axum::response::sse::{Event, Sse};
use tokio::sync::mpsc::Sender;
use tokio_stream::wrappers::ReceiverStream;
use tracing::Instrument;

use super::AppState;
use shadows_agent::choices::Offered;
use shadows_agent::events::HarnessEvent;
use shadows_core::OperationId;
use shadows_core::events::{EventCursor, UiSignal};
use shadows_core::storage::Storage;
use shadows_core::thread::ThreadId;

#[derive(serde::Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub struct SubscribeQuery {
    /// The thread to watch.
    pub thread_id: ThreadId,
    /// The last durable sequence this client has already applied; 0 replays
    /// the thread from its start.
    #[serde(default)]
    pub after: i64,
}

/// Spec §2.10: durable replay, then a no-gap handoff to live, then
/// de-duplication by durable sequence.
///
/// Two live sources, both taken BEFORE a single journal row is read:
///
/// - the storage's committed-sequence signal (spec §2.4: raised after each
///   commit that appended to the journal). A commit that lands while the
///   replay is being read shows up as a pending change, so it cannot fall into
///   the gap between the replay's last read and the live phase.
/// - the transient bus, for what is never stored: deltas and turn ends;
///   and beside it the session's options and `plan_show`'s signals.
///
/// Durable events reach the client only by reading the journal after
/// `last_seq` — in the replay, and again on every signal change in the live
/// phase — so each one is sent once, in sequence order, and de-duplication by
/// `seq` is structural rather than left to the client. The bus's `Entry` items
/// are therefore not forwarded: the same entry arrives as its durable
/// `ThreadEntryAppended` event. Bus items for another thread are dropped.
#[utoipa::path(
    get,
    path = "/api/subscribe",
    tag = "stream",
    params(SubscribeQuery),
    responses((
        status = 200,
        content_type = "text/event-stream",
        body = String,
        description = STREAM_DESCRIPTION,
    ))
)]
pub async fn subscribe(
    State(state): State<AppState>,
    Query(q): Query<SubscribeQuery>,
) -> Sse<ReceiverStream<Result<Event, Infallible>>> {
    let (tx, rx) = tokio::sync::mpsc::channel::<Result<Event, Infallible>>(1024);
    // Both taken before the replay is read; see above.
    let committed = state.core.storage().watch_committed();
    let live = state.core.bus().subscribe();
    let options = state.core.sessions().watch_options();
    let signals = state.core.ui_bus().subscribe();
    let span = tracing::debug_span!("sse", thread_id = %q.thread_id);
    tracing::debug!(parent: &span, after = q.after, "sse.subscribe");

    tokio::spawn(
        async move {
            let live = Live {
                bus: live,
                options,
                signals,
            };
            let why = stream(state, q, committed, live, tx).await;
            tracing::debug!(why, "sse.closed");
        }
        .instrument(span),
    );

    Sse::new(ReceiverStream::new(rx))
}

/// The transient sources, taken before the replay is read.
struct Live {
    bus: tokio::sync::broadcast::Receiver<(ThreadId, OperationId, HarnessEvent)>,
    options: tokio::sync::broadcast::Receiver<(ThreadId, Offered)>,
    signals: tokio::sync::broadcast::Receiver<UiSignal>,
}

/// The replay, the handoff, and the live phase for one subscriber. Returns why
/// the stream ended, for the `sse.closed` line.
async fn stream(
    state: AppState,
    q: SubscribeQuery,
    mut committed: tokio::sync::watch::Receiver<i64>,
    live: Live,
    tx: Sender<Result<Event, Infallible>>,
) -> &'static str {
    let Live {
        bus: mut live,
        mut options,
        mut signals,
    } = live;
    // 1. Durable replay.
    let mut last_seq = q.after;
    if let Err(why) =
        send_journal_after(state.core.storage(), &q.thread_id, &mut last_seq, &tx).await
    {
        return why;
    }

    // 2. Handoff. Tell the client where the durable replay ended.
    tracing::debug!(last_seq, "sse.caught_up");
    let _ = tx
        .send(Ok(Event::default()
            .event("caught-up")
            .data(serde_json::json!({ "seq": last_seq }).to_string())))
        .await;

    // 3. Live. The stream ends when the daemon stops (see
    // `AppState::shutdown`); the client resubscribes with its last seq
    // like after any other disconnect.
    let mut shutdown = state.shutdown.clone();
    loop {
        // The select only decides which source woke; acting on it happens
        // after, so no borrowed `watch::Ref` is held across an await.
        //
        // `biased`, journal before bus: a publisher commits (raising the
        // signal) before it sends the transient item that follows, so
        // polling the signal first keeps a turn's `turn-end` behind the
        // durable entry it ends.
        let mut woke_options = None;
        let mut woke_signal = None;
        let received = tokio::select! {
            biased;
            _ = shutdown.wait_for(|stopping| *stopping) => return "shutdown",
            changed = committed.changed() => {
                if changed.is_err() {
                    return "shutdown: storage closed";
                }
                None
            }
            received = live.recv() => Some(received),
            offered = options.recv() => {
                woke_options = Some(offered);
                None
            }
            signal = signals.recv() => {
                woke_signal = Some(signal);
                None
            }
        };
        if let Some(signal) = woke_signal.take() {
            let frame = match signal {
                // The card's durable event was committed before the signal
                // was sent, so the journal branch above has already sent it.
                Ok(signal) if signal.thread_id == q.thread_id => Event::default()
                    .event("plan-show")
                    .data(serde_json::json!(signal).to_string()),
                Ok(_) => continue,
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                    Event::default().event("lagged").data("")
                }
                Err(_) => return "shutdown: signals closed",
            };
            if tx.send(Ok(frame)).await.is_err() {
                return CLIENT_GONE;
            }
            continue;
        }
        if let Some(offered) = woke_options.take() {
            match offered {
                Ok((thread_id, offered)) if thread_id == q.thread_id => {
                    if let Some(ev) = options_event(&state, &thread_id, &offered).await
                        && tx.send(Ok(ev)).await.is_err()
                    {
                        return CLIENT_GONE;
                    }
                }
                Ok(_) => {}
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                    let _ = tx.send(Ok(Event::default().event("lagged").data(""))).await;
                }
                Err(_) => return "shutdown: options closed",
            }
            continue;
        }
        let Some(received) = received else {
            if let Err(why) =
                send_journal_after(state.core.storage(), &q.thread_id, &mut last_seq, &tx).await
            {
                return why;
            }
            continue;
        };
        match received {
            Ok((thread_id, op_id, item)) => {
                if thread_id != q.thread_id {
                    continue;
                }
                let ev = match item {
                    HarnessEvent::Usage { used, size, .. } => {
                        usage_event(&state, &thread_id, used, size).await
                    }
                    item => transient_event(&op_id, item),
                };
                let Some(ev) = ev else {
                    continue;
                };
                if tx.send(Ok(ev)).await.is_err() {
                    return CLIENT_GONE;
                }
            }
            // Spec §8.4 case 7: a client falling behind or
            // disconnecting never cancels work. It resubscribes with
            // its last seq.
            Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                let _ = tx.send(Ok(Event::default().event("lagged").data(""))).await;
            }
            Err(_) => return "shutdown: bus closed",
        }
    }
}

const CLIENT_GONE: &str = "client gone";

/// OpenAPI cannot type the frames of an event stream, so the document
/// describes them here, next to the code that sends them.
const STREAM_DESCRIPTION: &str = "Server-sent events for one thread (spec §2.10): \
the durable journal after `after`, then `caught-up`, then live. Each frame's \
`event:` names its kind and its `data:` is JSON unless stated.\n\n\
- `durable` — `{seq, kind, operation_id, thread_id, payload}`: one journal event. \
`operation_id` and `thread_id` are the ids it names, `null` where it names none; \
`payload` is the event's JSON object. Sent once each, in `seq` order, in the \
replay and live alike; remember the highest `seq` and resubscribe with it as `after`.\n\
- `caught-up` — `{seq}`: the last replayed `seq`. The replay is over.\n\
- `delta` — `{op, text}`: streamed text of a running turn. Transient: never replayed.\n\
- `turn-end` — `{op, subtype, stop_reason}`: the harness finished a turn. Transient.\n\
- `usage` — `{thread_id, context_used, context_window, limits}`: the session's \
context use and the account's limits as the harness last reported them; each is \
`null` when not reported. Transient.\n\
- `options` — `{thread_id, choices}`: the session's `SessionChoices` changed. Transient.\n\
- `plan-show` — `{thread_id, target_tab, workflow_id, version, task_number, place}`: \
the Planner showed a plan (§13.9). Its card arrives first, as the `durable` \
`PlanShown` event. Only the tab whose id is `target_tab` opens the panel or the \
page for `side` or `page`. Transient: never replayed.\n\
- `lagged` — empty: this client fell behind and transient frames were dropped; \
durable ones were not.\n\
- `fatal` — data is a message as plain text: the journal could not be read and \
the stream ends.\n\n\
The stream also ends when the daemon stops. Reconnect with the last `seq`.\n\n\
A `durable` frame of kind `OperationCompleted` carries `payload.invocation`: the \
turn's `InvocationView`.";

/// Sends every journal event for `thread_id` after `last_seq`, advancing it.
/// `Err` means the stream is over and says why: the client left, or storage
/// failed and a `fatal` event was sent.
async fn send_journal_after(
    storage: &Storage,
    thread_id: &ThreadId,
    last_seq: &mut i64,
    tx: &Sender<Result<Event, Infallible>>,
) -> Result<(), &'static str> {
    loop {
        let batch = match storage
            .read_events_after(EventCursor(*last_seq), thread_id, 500)
            .await
        {
            Ok(b) => b,
            Err(e) => {
                // Spec §3.2: the cause is logged here; the client is told only what
                // it needs to know — that the stream is over.
                tracing::error!(error = %e, "sse.replay_failed");
                let _ = tx
                    .send(Ok(Event::default()
                        .event("fatal")
                        .data("the journal could not be read")))
                    .await;
                return Err("error: journal read failed");
            }
        };
        if batch.is_empty() {
            return Ok(());
        }
        for ev in batch {
            // Every payload was written by `serde_json` in the transaction
            // that recorded its state change; one that no longer parses is a
            // damaged journal, which ends the stream like a failed read.
            let payload: serde_json::Value = match serde_json::from_str(&ev.payload_json) {
                Ok(p) => p,
                Err(e) => {
                    tracing::error!(seq = ev.seq, error = %e, "sse.payload_invalid");
                    let _ = tx
                        .send(Ok(Event::default()
                            .event("fatal")
                            .data(format!("event {} has an invalid payload", ev.seq))))
                        .await;
                    return Err("error: journal payload invalid");
                }
            };
            *last_seq = ev.seq;
            let frame = serde_json::json!({
                "seq": ev.seq,
                "kind": ev.kind,
                "operation_id": ev.operation_id,
                "thread_id": ev.thread_id,
                "payload": payload,
            });
            tx.send(Ok(Event::default()
                .event("durable")
                .data(frame.to_string())))
                .await
                .map_err(|_| CLIENT_GONE)?;
        }
    }
}

/// The `usage` frame: a usage report as it arrives, with the harness's latest
/// limits, which the watcher recorded before publishing it (§12.8). A `size`
/// of 0 is no window.
async fn usage_event(state: &AppState, thread: &ThreadId, used: u64, size: u64) -> Option<Event> {
    let limits = match state.core.storage().turn_context(thread).await {
        Ok(context) => state
            .core
            .storage()
            .latest_limits(&context.harness)
            .await
            .ok()
            .flatten(),
        Err(_) => None,
    };
    let frame = serde_json::json!({
        "thread_id": thread,
        "context_used": used,
        "context_window": (size > 0).then_some(size),
        "limits": limits,
    });
    Some(Event::default().event("usage").data(frame.to_string()))
}

/// The `options` frame: the thread's new offer as a client sees it (§12.4).
/// `None` when the thread's policy cannot be read; the next opening answers.
async fn options_event(state: &AppState, thread: &ThreadId, offered: &Offered) -> Option<Event> {
    let choices = state.core.harness().choices(thread, offered).await.ok()?;
    Some(
        Event::default()
            .event("options")
            .data(serde_json::json!({ "thread_id": thread, "choices": choices }).to_string()),
    )
}

/// The SSE form of a transient bus item, or `None` for one this stream does
/// not forward: entries arrive durable, options through `options_event`.
fn transient_event(op_id: &OperationId, item: HarnessEvent) -> Option<Event> {
    Some(match item {
        HarnessEvent::Chunk { text, .. } => Event::default()
            .event("delta")
            .data(serde_json::json!({ "op": op_id, "text": text }).to_string()),
        HarnessEvent::TurnEnd {
            subtype,
            stop_reason,
        } => Event::default().event("turn-end").data(
            serde_json::json!({ "op": op_id, "subtype": subtype, "stop_reason": stop_reason })
                .to_string(),
        ),
        _ => return None,
    })
}
