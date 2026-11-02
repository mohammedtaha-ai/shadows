use axum::{
    body::Body,
    extract::{Request, State},
    response::{
        sse::{Event, Sse},
        IntoResponse, Response,
    },
    routing::any,
    Router,
};
use std::{
    collections::{HashMap, VecDeque},
    convert::Infallible,
    sync::{
        atomic::{AtomicBool, AtomicI64, AtomicUsize, Ordering},
        Arc, Mutex,
    },
};
use tokio::sync::{broadcast, oneshot};
pub const CONFIG: &str = "00000000-0000-4000-8000-000000000001";
pub const RUNTIME: &str = "00000000-0000-4000-8000-000000000002";
pub const PROJECT: &str = "00000000-0000-4000-8000-000000000003";
pub const THREAD_A: &str = "00000000-0000-4000-8000-000000000004";
pub const THREAD_B: &str = "00000000-0000-4000-8000-000000000005";
pub struct Data {
    pub colour: Mutex<Option<String>>,
    pub metadata_delay: AtomicUsize,
    pub wrong_runtime: AtomicBool,
    pub offline: AtomicBool,
    pub requests: Mutex<Vec<String>>,
    pub operations: Mutex<HashMap<String, Vec<serde_json::Value>>>,
    pub entries: Mutex<HashMap<String, Vec<serde_json::Value>>>,
    pub replay: Mutex<Vec<(String, String)>>,
    pub no_caught_up: AtomicBool,
    pub stall_headers: AtomicBool,
    pub removed: AtomicBool,
    pub deleted: AtomicBool,
    pub key: AtomicBool,
    pub oversized: AtomicBool,
    pub active: AtomicUsize,
    pub sequence: AtomicI64,
    pub events: broadcast::Sender<(String, String)>,
    pub permissions: Mutex<Vec<serde_json::Value>>,
    pub project_events: broadcast::Sender<(String, String)>,
    pub project_sequence: AtomicI64,
}
pub struct Fixture {
    pub origin: String,
    pub data: Arc<Data>,
    stop: Option<oneshot::Sender<()>>,
    task: tokio::task::JoinHandle<()>,
}
impl Fixture {
    pub async fn new() -> Self {
        let socket = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let origin = format!("http://{}", socket.local_addr().unwrap());
        let (events, _) = broadcast::channel(64);
        let (project_events, _) = broadcast::channel(64);
        let data = Arc::new(Data {
            colour: Mutex::new(None),
            metadata_delay: AtomicUsize::new(0),
            wrong_runtime: AtomicBool::new(false),
            offline: AtomicBool::new(false),
            requests: Mutex::new(vec![]),
            operations: Mutex::new(HashMap::new()),
            entries: Mutex::new(HashMap::new()),
            replay: Mutex::new(vec![]),
            no_caught_up: AtomicBool::new(false),
            stall_headers: AtomicBool::new(false),
            removed: AtomicBool::new(false),
            deleted: AtomicBool::new(false),
            key: AtomicBool::new(true),
            oversized: AtomicBool::new(false),
            active: AtomicUsize::new(0),
            sequence: AtomicI64::new(0),
            events,
            permissions: Mutex::new(vec![]),
            project_events,
            project_sequence: AtomicI64::new(0),
        });
        let app = Router::new().fallback(any(handle)).with_state(data.clone());
        let (stop, rx) = oneshot::channel();
        let task = tokio::spawn(async move {
            axum::serve(socket, app)
                .with_graceful_shutdown(async {
                    let _ = rx.await;
                })
                .await
                .unwrap()
        });
        Self {
            origin,
            data,
            stop: Some(stop),
            task,
        }
    }
    pub fn operation(&self, thread: &str, id: &str, status: &str) {
        self.data.operations.lock().unwrap().insert(thread.into(),vec![serde_json::json!({"id":id,"kind":"PlannerTurn","status_kind":status,"thread_id":thread})]);
    }
    pub fn entry(&self, thread: &str, id: &str, body: &str) {
        let mut entries = self.data.entries.lock().unwrap();
        let items = entries.entry(thread.into()).or_default();
        let ordinal = items.len() + 1;
        items.push(serde_json::json!({"id":format!("entry-{ordinal}"),"thread_id":thread,"operation_id":id,"kind":"AgentMessage","ordinal":ordinal,"body":body}));
    }
    pub fn emit(&self, name: &str, data: serde_json::Value) {
        let _ = self.data.events.send((name.into(), data.to_string()));
    }
    pub fn durable(&self, thread: &str, op: &str, kind: &str) {
        let seq = self.data.sequence.fetch_add(1, Ordering::SeqCst) + 1;
        self.emit("durable",serde_json::json!({"seq":seq,"kind":kind,"operation_id":op,"thread_id":thread,"payload":{}}));
    }
}
impl Fixture {
    /// A request Core holds for the person; `options` carry the adapter's own ids.
    pub fn permission(&self, thread: &str, request: &str, title: &str) {
        self.data
            .permissions
            .lock()
            .unwrap()
            .push(serde_json::json!({
                "thread_id": thread, "agent": "agent-a", "request_id": request, "title": title,
                "options": [{"option_id":"allow_once","name":"Allow","kind":"allow_once"}],
                "asked_at": "2026-11-01T10:00:00Z"
            }));
    }
    pub fn settle(&self, request: &str) {
        self.data
            .permissions
            .lock()
            .unwrap()
            .retain(|item| item["request_id"] != request);
    }
    pub fn permissions_changed(&self, thread: &str, request: &str, state: &str) {
        let seq = self.data.project_sequence.fetch_add(1, Ordering::SeqCst) + 1;
        let _ = self.data.project_events.send((
            "durable".into(),
            serde_json::json!({"seq":seq,"kind":"PermissionsChanged","operation_id":null,"thread_id":thread,"payload":{"request_id":request,"state":state}}).to_string(),
        ));
    }
    pub fn project_emit(&self, name: &str) {
        let _ = self.data.project_events.send((name.into(), "{}".into()));
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        if let Some(stop) = self.stop.take() {
            let _ = stop.send(());
        }
        self.task.abort();
    }
}
struct Active(Arc<Data>);
impl Drop for Active {
    fn drop(&mut self) {
        self.0.active.fetch_sub(1, Ordering::SeqCst);
    }
}
async fn handle(State(data): State<Arc<Data>>, request: Request) -> Response {
    let path = request.uri().path().to_owned();
    data.requests
        .lock()
        .unwrap()
        .push(format!("{} {}", request.method(), request.uri()));
    if request.method() != axum::http::Method::GET {
        return axum::http::StatusCode::METHOD_NOT_ALLOWED.into_response();
    }
    if data.offline.load(Ordering::SeqCst) {
        return axum::http::StatusCode::SERVICE_UNAVAILABLE.into_response();
    }
    if path == "/api/aylul/agents" {
        let colour = data.colour.lock().unwrap().clone();
        let delay = data.metadata_delay.swap(0, Ordering::SeqCst);
        tokio::time::sleep(std::time::Duration::from_millis(delay as u64)).await;
        let mut agent = serde_json::json!({"slug":"agent-a","provider":"provider-a","deleted":data.deleted.load(Ordering::SeqCst)});
        if let Some(colour) = colour {
            agent["character_color"] = colour.into();
        }
        return axum::Json(serde_json::json!({"project_id":PROJECT,"agents":[agent,{"slug":"agent-b","provider":"provider-a","deleted":false,"character_color":"pink"}]})).into_response();
    }
    if path == format!("/api/projects/{PROJECT}/events") {
        let initial = VecDeque::from([(
            "caught-up".to_owned(),
            serde_json::json!({"seq":data.project_sequence.load(Ordering::SeqCst)}).to_string(),
        )]);
        let stream = futures_util::stream::unfold(
            (initial, data.project_events.subscribe()),
            |(mut initial, mut events)| async move {
                let frame = if let Some(frame) = initial.pop_front() {
                    Some(frame)
                } else {
                    events.recv().await.ok()
                };
                frame.map(|(name, body)| {
                    (
                        Ok::<_, Infallible>(Event::default().event(name).data(body)),
                        (initial, events),
                    )
                })
            },
        );
        return Sse::new(stream).into_response();
    }
    if path == "/api/aylul/permissions" {
        return axum::Json(data.permissions.lock().unwrap().clone()).into_response();
    }
    if path == "/api/subscribe" {
        if data.stall_headers.load(Ordering::SeqCst) {
            tokio::time::sleep(std::time::Duration::from_secs(60)).await;
        }
        data.active.fetch_add(1, Ordering::SeqCst);
        let active = Active(data.clone());
        let mut initial = VecDeque::from(data.replay.lock().unwrap().clone());
        if !data.no_caught_up.load(Ordering::SeqCst) {
            initial.push_back((
                "caught-up".into(),
                serde_json::json!({"seq":data.sequence.load(Ordering::SeqCst)}).to_string(),
            ));
        }
        let stream = futures_util::stream::unfold(
            (initial, data.events.subscribe(), active),
            |(mut initial, mut events, active)| async move {
                let frame = if let Some(frame) = initial.pop_front() {
                    Some(frame)
                } else {
                    events.recv().await.ok()
                };
                frame
                    .filter(|(name, _)| name != "__disconnect")
                    .map(|(name, body)| {
                        (
                            Ok::<_, Infallible>(Event::default().event(name).data(body)),
                            (initial, events, active),
                        )
                    })
            },
        );
        return Sse::new(stream).into_response();
    }
    if path.ends_with("/entries") && data.oversized.load(Ordering::SeqCst) {
        return Response::builder()
            .header("content-type", "application/json")
            .body(Body::from("x".repeat(16 * 1024 * 1024 + 1)))
            .unwrap();
    }
    let value = match path.as_str() {
        "/api/runtime" => {
            serde_json::json!({"runtime_id":if data.wrong_runtime.load(Ordering::SeqCst){"changed"}else{RUNTIME},"configuration_id":CONFIG})
        }
        "/api/aylul/agents" => {
            serde_json::json!({"project_id":PROJECT,"agents":[{"slug":"agent-a","provider":"provider-a","deleted":data.deleted.load(Ordering::SeqCst)},{"slug":"agent-b","provider":"provider-a","deleted":false}]})
        }
        "/api/aylul/providers" => {
            serde_json::json!([{"slug":"provider-a","key_set":data.key.load(Ordering::SeqCst)}])
        }
        _ => {
            let pieces: Vec<_> = path.split('/').collect();
            let thread = pieces.get(3).copied().unwrap_or("");
            match pieces.get(4).copied() {
                Some("operations") => serde_json::json!(data
                    .operations
                    .lock()
                    .unwrap()
                    .get(thread)
                    .cloned()
                    .unwrap_or_default()),
                Some("entries") => serde_json::json!(data
                    .entries
                    .lock()
                    .unwrap()
                    .get(thread)
                    .cloned()
                    .unwrap_or_default()),
                None if matches!(thread, THREAD_A | THREAD_B) => {
                    serde_json::json!({"id":thread,"project_id":PROJECT,"harness":"aylul","profile":if thread==THREAD_A{"agent-a"}else{"agent-b"},"removed_at":if thread==THREAD_A&&data.removed.load(Ordering::SeqCst){Some("2026-10-20")}else{None}})
                }
                _ => return axum::http::StatusCode::NOT_FOUND.into_response(),
            }
        }
    };
    axum::Json(value).into_response()
}
