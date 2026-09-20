//! THROWAWAY SPIKE. Answers two questions and then gets deleted:
//!   1. Can `shadows serve` hand a browser a page and stream a real Claude
//!      child process to it over SSE, with no database and no framework?
//!   2. What is the concrete shape of the harness output contract, and where
//!      does the line fall between durable ThreadEntry and transient delta?
//!
//! Deliberately absent: persistence, modules, process-tree containment,
//! cancellation, idempotency, error taxonomy. None of that is being probed.

use std::convert::Infallible;
use std::process::Stdio;
use std::sync::{Arc, Mutex};

use axum::Router;
use axum::extract::{Query, State};
use axum::response::Html;
use axum::response::sse::{Event, Sse};
use axum::routing::get;
use serde_json::Value;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio_stream::wrappers::ReceiverStream;

/// The only state: the harness session id, so a later turn can `--resume` the
/// first. Stands in for what a PlanningThread row would hold.
#[derive(Clone, Default)]
struct AppState {
    session: Arc<Mutex<Option<String>>>,
}

#[tokio::main]
async fn main() {
    let state = AppState::default();
    let app = Router::new()
        .route("/", get(index))
        .route("/run", get(run))
        .route("/reset", get(reset))
        .with_state(state);

    let addr = "127.0.0.1:4317";
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    // The daemon prints the URL. It never opens a browser (spec rule).
    println!("spike listening on http://{addr}");
    axum::serve(listener, app).await.unwrap();
}

async fn index() -> Html<&'static str> {
    Html(include_str!("index.html"))
}

async fn reset(State(st): State<AppState>) -> &'static str {
    *st.session.lock().unwrap() = None;
    "reset"
}

#[derive(serde::Deserialize)]
struct RunQuery {
    prompt: String,
}

/// Spawn one real Claude turn and translate its stdout into SSE.
///
/// The translation IS the finding. Every stdout line is classified into
/// exactly one of a few SSE event kinds, and only some of them would ever be
/// written to durable storage.
async fn run(
    State(st): State<AppState>,
    Query(q): Query<RunQuery>,
) -> Sse<ReceiverStream<Result<Event, Infallible>>> {
    let (tx, rx) = tokio::sync::mpsc::channel::<Result<Event, Infallible>>(256);

    let resume = st.session.lock().unwrap().clone();
    let session_slot = st.session.clone();

    tokio::spawn(async move {
        let mut cmd = Command::new("claude");
        cmd.arg("--print")
            .arg("--output-format")
            .arg("stream-json")
            .arg("--verbose")
            .arg("--include-partial-messages")
            .arg("--model")
            .arg("sonnet")
            .arg("--permission-prompts")
            .arg("none")
            .arg("--safe-mode");

        match &resume {
            Some(id) => {
                cmd.arg("--resume").arg(id);
            }
            None => {
                let id = uuid::Uuid::new_v4().to_string();
                cmd.arg("--session-id").arg(&id);
            }
        }

        cmd.arg(&q.prompt)
            // Finding: without this the child waits 3s for stdin that never
            // arrives. A daemon must always close the child stdin.
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let mut child = match cmd.spawn() {
            Ok(c) => c,
            Err(e) => {
                let _ = tx.send(Ok(ev("fatal", &format!("spawn failed: {e}")))).await;
                return;
            }
        };

        let stdout = child.stdout.take().unwrap();
        let stderr = child.stderr.take().unwrap();

        // stderr is diagnostics only; it never carries conversation content.
        let tx_err = tx.clone();
        tokio::spawn(async move {
            let mut lines = BufReader::new(stderr).lines();
            while let Ok(Some(l)) = lines.next_line().await {
                let _ = tx_err.send(Ok(ev("meta", &format!("stderr: {l}")))).await;
            }
        });

        let mut lines = BufReader::new(stdout).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            let Ok(v) = serde_json::from_str::<Value>(&line) else {
                let _ = tx.send(Ok(ev("meta", &format!("unparsed: {line}")))).await;
                continue;
            };

            // Capture the session id the harness actually used, so a later
            // turn resumes instead of starting fresh.
            if v["type"] == "system" && v["subtype"] == "init" {
                if let Some(id) = v["session_id"].as_str() {
                    *session_slot.lock().unwrap() = Some(id.to_string());
                }
            }

            let sse = classify(&v);
            if tx.send(Ok(sse)).await.is_err() {
                // Browser disconnected. A real implementation has to decide
                // whether the turn keeps running; the spike just stops
                // reading. Noted, not probed.
                break;
            }
        }

        let status = child.wait().await;
        let _ = tx.send(Ok(ev("exit", &format!("{status:?}")))).await;
    });

    Sse::new(ReceiverStream::new(rx))
}

/// The durable/transient boundary, in one function.
fn classify(v: &Value) -> Event {
    match v["type"].as_str() {
        // TRANSIENT. Render-only. Hundreds per turn. Never persisted.
        Some("stream_event") => {
            let e = &v["event"];
            if e["type"] == "content_block_delta" && e["delta"]["type"] == "text_delta" {
                return ev("delta", e["delta"]["text"].as_str().unwrap_or(""));
            }
            if e["type"] == "content_block_start" {
                let kind = e["content_block"]["type"].as_str().unwrap_or("?");
                let name = e["content_block"]["name"].as_str().unwrap_or("");
                return ev("meta", &format!("block start: {kind} {name}"));
            }
            ev("meta", e["type"].as_str().unwrap_or("stream_event"))
        }

        // DURABLE. Complete, final, already carries its own stable uuid.
        // Arrives AFTER its own deltas. This is the ThreadEntry.
        Some("assistant") | Some("user") => {
            let role = v["type"].as_str().unwrap_or("?");
            let uuid = v["uuid"].as_str().unwrap_or("");
            let short = &uuid[..uuid.len().min(8)];
            let summary = v["message"]["content"]
                .as_array()
                .map(|blocks| {
                    blocks
                        .iter()
                        .map(|b| match b["type"].as_str() {
                            Some("text") => format!("text: {}", b["text"].as_str().unwrap_or("")),
                            Some("tool_use") => {
                                format!("tool_use: {}", b["name"].as_str().unwrap_or("?"))
                            }
                            Some("tool_result") => "tool_result".to_string(),
                            Some(other) => other.to_string(),
                            None => "?".to_string(),
                        })
                        .collect::<Vec<_>>()
                        .join(" | ")
                })
                .unwrap_or_default();
            ev("entry", &format!("[{role} {short}] {summary}"))
        }

        // TURN END. Exactly one, always last. Carries stop_reason and cost.
        Some("result") => ev(
            "done",
            &format!(
                "subtype={} stop={} cost_usd={}",
                v["subtype"].as_str().unwrap_or("?"),
                v["stop_reason"].as_str().unwrap_or("?"),
                v["total_cost_usd"]
            ),
        ),

        // OPERATIONAL. Useful to surface, not conversation content.
        Some("system") => ev(
            "meta",
            &format!(
                "system/{}: {}",
                v["subtype"].as_str().unwrap_or("?"),
                v["status"]
                    .as_str()
                    .or(v["detail"].as_str())
                    .or(v["status_detail"].as_str())
                    .unwrap_or("")
            ),
        ),
        Some("rate_limit_event") => ev(
            "meta",
            &format!("rate_limit: {}", v["rate_limit_info"]["status"]),
        ),
        other => ev("meta", other.unwrap_or("unknown")),
    }
}

fn ev(kind: &str, data: &str) -> Event {
    Event::default().event(kind).data(data)
}
