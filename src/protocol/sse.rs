//! One job: the durable-replay-then-live stream (spec §2.10).

use std::convert::Infallible;

use axum::extract::{Query, State};
use axum::response::sse::{Event, Sse};
use tokio::sync::mpsc::Sender;
use tokio_stream::wrappers::ReceiverStream;
use tracing::Instrument;

use super::AppState;
use crate::agent::StreamItem;
use crate::events::EventCursor;
use crate::operation::OperationId;
use crate::storage::Storage;
use crate::thread::ThreadId;

#[derive(serde::Deserialize)]
pub struct SubscribeQuery {
    pub thread_id: ThreadId,
    /// The last durable sequence this client has already applied.
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
/// - the transient bus, for what is never stored: deltas, turn ends, meta.
///
/// Durable events reach the client only by reading the journal after
/// `last_seq` — in the replay, and again on every signal change in the live
/// phase — so each one is sent once, in sequence order, and de-duplication by
/// `seq` is structural rather than left to the client. The bus's `Entry` items
/// are therefore not forwarded: the same entry arrives as its durable
/// `ThreadEntryAppended` event. Bus items for another thread are dropped.
pub async fn subscribe(
    State(state): State<AppState>,
    Query(q): Query<SubscribeQuery>,
) -> Sse<ReceiverStream<Result<Event, Infallible>>> {
    let (tx, rx) = tokio::sync::mpsc::channel::<Result<Event, Infallible>>(1024);
    // Both taken before the replay is read; see above.
    let committed = state.storage.watch_committed();
    let live = state.bus.subscribe();
    let span = tracing::debug_span!("sse", thread_id = %q.thread_id);
    tracing::debug!(parent: &span, after = q.after, "sse.subscribe");

    tokio::spawn(
        async move {
            let why = stream(state, q, committed, live, tx).await;
            tracing::debug!(why, "sse.closed");
        }
        .instrument(span),
    );

    Sse::new(ReceiverStream::new(rx))
}

/// The replay, the handoff, and the live phase for one subscriber. Returns why
/// the stream ended, for the `sse.closed` line.
async fn stream(
    state: AppState,
    q: SubscribeQuery,
    mut committed: tokio::sync::watch::Receiver<i64>,
    mut live: tokio::sync::broadcast::Receiver<(ThreadId, OperationId, StreamItem)>,
    tx: Sender<Result<Event, Infallible>>,
) -> &'static str {
    // 1. Durable replay.
    let mut last_seq = q.after;
    if let Err(why) = send_journal_after(&state.storage, &q.thread_id, &mut last_seq, &tx).await {
        return why;
    }

    // 2. Handoff. Tell the client where the durable replay ended.
    tracing::debug!(last_seq, "sse.caught_up");
    let _ = tx
        .send(Ok(Event::default()
            .event("caught-up")
            .data(last_seq.to_string())))
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
        };
        let Some(received) = received else {
            if let Err(why) =
                send_journal_after(&state.storage, &q.thread_id, &mut last_seq, &tx).await
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
                let Some(ev) = transient_event(&op_id, item) else {
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
                tracing::error!(error = %e, "sse.replay_failed");
                let _ = tx
                    .send(Ok(Event::default().event("fatal").data(e.to_string())))
                    .await;
                return Err("error: journal read failed");
            }
        };
        if batch.is_empty() {
            return Ok(());
        }
        for ev in batch {
            *last_seq = ev.seq;
            let payload = serde_json::json!({
                "seq": ev.seq, "kind": ev.kind, "payload": ev.payload_json,
            });
            tx.send(Ok(Event::default()
                .event("durable")
                .data(payload.to_string())))
                .await
                .map_err(|_| CLIENT_GONE)?;
        }
    }
}

/// The SSE form of a transient bus item, or `None` for one this stream does
/// not forward. `Entry` is `None` because its durable event carries it.
fn transient_event(op_id: &OperationId, item: StreamItem) -> Option<Event> {
    Some(match item {
        StreamItem::Delta { text } => Event::default()
            .event("delta")
            .data(serde_json::json!({ "op": op_id, "text": text }).to_string()),
        StreamItem::TurnEnd {
            subtype,
            stop_reason,
        } => Event::default().event("turn-end").data(
            serde_json::json!({
                "op": op_id, "subtype": subtype, "stop_reason": stop_reason
            })
            .to_string(),
        ),
        StreamItem::Operational { label, .. } => Event::default()
            .event("meta")
            .data(serde_json::json!({ "op": op_id, "label": label }).to_string()),
        StreamItem::Entry { .. } | StreamItem::Unparsed(_) => return None,
    })
}
