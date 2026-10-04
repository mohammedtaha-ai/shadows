//! One job: the durable-replay-then-live stream (spec §2.10), framed as SSE.
//!
//! `Events` (`shadows-core`) owns what is delivered and in what order; this
//! file owns the framing: each `Delivery` becomes today's frame, with its
//! event name and JSON body, over a bounded channel, and the `sse.*` lines.

use std::convert::Infallible;

use axum::extract::{Path, Query, State};
use axum::response::sse::{Event, Sse};
use tokio::sync::mpsc::Sender;
use tokio_stream::wrappers::ReceiverStream;
use tracing::Instrument;

use super::failure::ErrorBody;
use super::{AppState, Failure};
use shadows_core::{Delivery, ProjectId, StoredEvent, Subscription, ThreadId};

#[derive(serde::Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub struct ProjectSubscribeQuery {
    /// Resume after the last durable sequence delivered; 0 replays all project plan/design events.
    #[serde(default)]
    pub after: i64,
}

/// A project's plan/design notifications (§16.8, §18.11), replayed then live on the same
/// journal tail as `/api/subscribe`. `durable` carries `{seq, kind,
/// operation_id, thread_id, payload: {plan_id, workflow_id}}`, no plan content.
/// Archive notifications name the plan's latest version to refetch.
/// ProjectDesignChanged carries project_id, revision, changed_parts,
/// changed_outcomes and vision_changed, never the vision content.
#[utoipa::path(
    get, path = "/api/projects/{id}/events", tag = "stream",
    params(("id" = ProjectId, Path, description = "The project"), ProjectSubscribeQuery),
    responses(
        (status = 200, content_type = "text/event-stream", body = String),
        (status = 404, description = "Unknown or removed project", body = ErrorBody),
    )
)]
pub async fn subscribe_project(
    State(state): State<AppState>,
    Path(project): Path<ProjectId>,
    Query(q): Query<ProjectSubscribeQuery>,
) -> Result<Sse<ReceiverStream<Result<Event, Infallible>>>, Failure> {
    let sub = state
        .core
        .events()
        .subscribe_project(project.clone(), q.after)
        .await?;
    let (tx, rx) = tokio::sync::mpsc::channel(1024);
    let span = tracing::debug_span!("sse", project_id = %project);
    tracing::debug!(parent: &span, after = q.after, "sse.subscribe");
    tokio::spawn(
        async move {
            let why = stream(state, sub, tx).await;
            tracing::debug!(why, "sse.closed");
        }
        .instrument(span),
    );
    Ok(Sse::new(ReceiverStream::new(rx)))
}

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
    // Every live source is taken here, before the replay is read; see above.
    let sub = state.core.events().subscribe(q.thread_id.clone(), q.after);
    let span = tracing::debug_span!("sse", thread_id = %q.thread_id);
    tracing::debug!(parent: &span, after = q.after, "sse.subscribe");

    tokio::spawn(
        async move {
            let why = stream(state, sub, tx).await;
            tracing::debug!(why, "sse.closed");
        }
        .instrument(span),
    );

    Sse::new(ReceiverStream::new(rx))
}

/// Sends one subscriber's deliveries as frames until the subscription ends,
/// the client leaves, or the daemon stops (see `AppState::shutdown`); the
/// client resubscribes with its last seq like after any other disconnect.
/// Returns why the stream ended, for the `sse.closed` line.
async fn stream(
    state: AppState,
    mut sub: Subscription,
    tx: Sender<Result<Event, Infallible>>,
) -> &'static str {
    let mut shutdown = state.shutdown.clone();
    loop {
        let delivery = tokio::select! {
            biased;
            _ = shutdown.wait_for(|stopping| *stopping) => return "shutdown",
            d = sub.next() => d,
        };
        let frame = match delivery {
            Err(why) => return why,
            Ok(Delivery::CaughtUp { seq }) => {
                // The handoff: tell the client where the durable replay ended.
                tracing::debug!(last_seq = seq, "sse.caught_up");
                let _ = tx
                    .send(Ok(Event::default()
                        .event("caught-up")
                        .data(serde_json::json!({ "seq": seq }).to_string())))
                    .await;
                continue;
            }
            Ok(Delivery::Fatal(message)) => {
                // The subscription is over: the next `next` answers why.
                let _ = tx
                    .send(Ok(Event::default().event("fatal").data(message)))
                    .await;
                continue;
            }
            Ok(Delivery::Durable(ev)) => match durable_frame(&ev) {
                Ok(frame) => frame,
                Err(fatal) => {
                    let _ = tx.send(Ok(fatal)).await;
                    return "error: journal payload invalid";
                }
            },
            Ok(delivery) => frame(delivery),
        };
        if tx.send(Ok(frame)).await.is_err() {
            return CLIENT_GONE;
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

/// The `durable` frame of one journal event. Every payload was written by
/// `serde_json` in the transaction that recorded its state change; one that no
/// longer parses is a damaged journal, and `Err` is the `fatal` frame that ends
/// the stream like a failed read.
fn durable_frame(ev: &StoredEvent) -> Result<Event, Event> {
    let payload: serde_json::Value = match serde_json::from_str(&ev.payload_json) {
        Ok(p) => p,
        Err(e) => {
            tracing::error!(seq = ev.seq, error = %e, "sse.payload_invalid");
            return Err(Event::default()
                .event("fatal")
                .data(format!("event {} has an invalid payload", ev.seq)));
        }
    };
    let frame = serde_json::json!({
        "seq": ev.seq,
        "kind": ev.kind,
        "operation_id": ev.operation_id,
        "thread_id": ev.thread_id,
        "payload": payload,
    });
    Ok(Event::default().event("durable").data(frame.to_string()))
}

/// The frame of a transient delivery: `delta`, `turn-end`, `usage`,
/// `options`, `plan-show` or `lagged`.
fn frame(delivery: Delivery) -> Event {
    match delivery {
        Delivery::Delta { op, text } => Event::default()
            .event("delta")
            .data(serde_json::json!({ "op": op, "text": text }).to_string()),
        Delivery::TurnEnd {
            op,
            subtype,
            stop_reason,
        } => Event::default().event("turn-end").data(
            serde_json::json!({ "op": op, "subtype": subtype, "stop_reason": stop_reason })
                .to_string(),
        ),
        // A usage report as it arrives, with the harness's latest limits,
        // which the watcher recorded before publishing it (§12.8). A `size`
        // of 0 is no window.
        Delivery::Usage {
            thread,
            used,
            size,
            limits,
        } => Event::default().event("usage").data(
            serde_json::json!({
                "thread_id": thread,
                "context_used": used,
                "context_window": (size > 0).then_some(size),
                "limits": limits,
            })
            .to_string(),
        ),
        // The thread's new offer as a client sees it (§12.4).
        Delivery::Options { thread, choices } => Event::default()
            .event("options")
            .data(serde_json::json!({ "thread_id": thread, "choices": choices }).to_string()),
        Delivery::PlanShow(signal) => Event::default()
            .event("plan-show")
            .data(serde_json::json!(signal).to_string()),
        Delivery::Lagged => Event::default().event("lagged").data(""),
        Delivery::Durable(_) | Delivery::CaughtUp { .. } | Delivery::Fatal(_) => {
            unreachable!("`stream` frames these itself")
        }
    }
}
