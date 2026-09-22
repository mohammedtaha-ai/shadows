//! One job: the durable-replay-then-live stream (spec §2.10).

use std::convert::Infallible;

use axum::extract::{Query, State};
use axum::response::sse::{Event, Sse};
use tokio_stream::wrappers::ReceiverStream;

use super::AppState;
use crate::agent::StreamItem;
use crate::events::EventCursor;
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
/// The order below is the whole guarantee. The live subscription is taken
/// FIRST, so anything committed while the replay is being read lands in the
/// broadcast buffer instead of falling into the gap between them. Replayed
/// events carry their `seq`; the client discards any live event whose `seq` it
/// has already applied.
pub async fn subscribe(
    State(state): State<AppState>,
    Query(q): Query<SubscribeQuery>,
) -> Sse<ReceiverStream<Result<Event, Infallible>>> {
    let (tx, rx) = tokio::sync::mpsc::channel::<Result<Event, Infallible>>(1024);
    let mut live = state.bus.subscribe(); // taken before the replay is read

    tokio::spawn(async move {
        // 1. Durable replay.
        let mut last_seq = q.after;
        loop {
            let batch = match state
                .storage
                .read_events_after(EventCursor(last_seq), &q.thread_id, 500)
                .await
            {
                Ok(b) => b,
                Err(e) => {
                    let _ = tx
                        .send(Ok(Event::default().event("fatal").data(e.to_string())))
                        .await;
                    return;
                }
            };
            if batch.is_empty() {
                break;
            }
            for ev in batch {
                last_seq = ev.seq;
                let payload = serde_json::json!({
                    "seq": ev.seq, "kind": ev.kind, "payload": ev.payload_json,
                });
                if tx
                    .send(Ok(Event::default()
                        .event("durable")
                        .data(payload.to_string())))
                    .await
                    .is_err()
                {
                    return;
                }
            }
        }

        // 2. Handoff. Tell the client where the durable replay ended so it can
        // de-duplicate anything the live stream repeats.
        let _ = tx
            .send(Ok(Event::default()
                .event("caught-up")
                .data(last_seq.to_string())))
            .await;

        // 3. Live. Transient deltas are forwarded and never stored.
        loop {
            match live.recv().await {
                Ok((op_id, item)) => {
                    let ev = match item {
                        StreamItem::Delta { text } => Event::default()
                            .event("delta")
                            .data(serde_json::json!({ "op": op_id, "text": text }).to_string()),
                        StreamItem::Entry { uuid, role, text } => {
                            Event::default().event("entry").data(
                                serde_json::json!({
                                    "op": op_id, "uuid": uuid, "role": role, "text": text
                                })
                                .to_string(),
                            )
                        }
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
                        StreamItem::Unparsed(_) => continue,
                    };
                    if tx.send(Ok(ev)).await.is_err() {
                        return;
                    }
                }
                // Spec §8.4 case 7: a client falling behind or disconnecting
                // never cancels work. It resubscribes with its last seq.
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                    let _ = tx.send(Ok(Event::default().event("lagged").data(""))).await;
                }
                Err(_) => return,
            }
        }
    });

    Sse::new(ReceiverStream::new(rx))
}
