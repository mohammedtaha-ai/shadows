// Scripted loopback Core API for companion submission regressions.
use super::send::Submission;
use super::*;
use serde_json::{json, Value};
use std::{
    io::{Read, Write},
    net::TcpListener,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread,
    time::Duration,
};
pub(super) const CONFIG: &str = "00000000-0000-4000-8000-000000000001";
pub(super) const PROJECT: &str = "00000000-0000-4000-8000-000000000002";
pub(super) const T1: &str = "00000000-0000-4000-8000-000000000003";
pub(super) const T2: &str = "00000000-0000-4000-8000-000000000004";
#[derive(Default)]
pub(super) struct Script {
    pub(super) calls: Vec<(String, String, Value)>,
    pub(super) create_delays: usize,
    pub(super) turn_delays: usize,
    pub(super) removed: bool,
    pub(super) error: Option<String>,
    pub(super) settings: bool,
    pub(super) runtime_changed: bool,
    pub(super) gate_create: bool,
    pub(super) turns: std::collections::HashMap<String, Value>,
    pub(super) gate: Option<(mpsc::Sender<()>, mpsc::Receiver<()>)>,
}
pub(super) struct Api {
    pub(super) state: Arc<Mutex<Script>>,
    stop: Arc<AtomicBool>,
    worker: Option<thread::JoinHandle<()>>,
    pub(super) origin: String,
}
impl Api {
    fn new() -> Self {
        let socket = TcpListener::bind("127.0.0.1:0").unwrap();
        socket.set_nonblocking(true).unwrap();
        let origin = format!("http://{}", socket.local_addr().unwrap());
        let state = Arc::new(Mutex::new(Script::default()));
        let stop = Arc::new(AtomicBool::new(false));
        let (shared, cancelled) = (state.clone(), stop.clone());
        let worker = thread::spawn(move || {
            while !cancelled.load(Ordering::Relaxed) {
                let Ok((mut stream, _)) = socket.accept() else {
                    thread::sleep(Duration::from_millis(2));
                    continue;
                };
                let shared = shared.clone();
                thread::spawn(move || {
                    stream.set_nonblocking(false).unwrap();
                    stream
                        .set_read_timeout(Some(Duration::from_secs(2)))
                        .unwrap();
                    let mut bytes = Vec::new();
                    let mut buf = [0; 4096];
                    let header_end = loop {
                        let count = stream.read(&mut buf).unwrap();
                        if count == 0 {
                            return;
                        }
                        bytes.extend_from_slice(&buf[..count]);
                        if let Some(end) = bytes.windows(4).position(|x| x == b"\r\n\r\n") {
                            break end + 4;
                        }
                    };
                    let headers = String::from_utf8_lossy(&bytes[..header_end]).into_owned();
                    let length = headers
                        .lines()
                        .find_map(|l| {
                            l.to_lowercase()
                                .strip_prefix("content-length:")
                                .and_then(|v| v.trim().parse::<usize>().ok())
                        })
                        .unwrap_or(0);
                    while bytes.len() < header_end + length {
                        let count = stream.read(&mut buf).unwrap();
                        if count == 0 {
                            return;
                        }
                        bytes.extend_from_slice(&buf[..count]);
                    }
                    let mut first = headers.split_whitespace();
                    let method = first.next().unwrap().to_string();
                    let path = first.next().unwrap().to_string();
                    let body: Value =
                        serde_json::from_slice(&bytes[header_end..header_end + length])
                            .unwrap_or(Value::Null);
                    let (response, delay, gate, status) = {
                        let mut s = shared.lock().unwrap();
                        s.calls.push((method.clone(), path.clone(), body.clone()));
                        let mut delay = false;
                        let mut gate = None;
                        let mut status = 200;
                        let response = if method == "GET" && path == "/api/runtime" {
                            json!({"configuration_id":CONFIG,"runtime_id":if s.runtime_changed{T1}else{T2}})
                        } else if method == "GET" && path == "/api/aylul/agents" {
                            json!({"project_id":PROJECT,"agents":[{"slug":"agent-a","deleted":false,"default_model":"default-model"},{"slug":"agent-b","deleted":false}]})
                        } else if method == "GET" {
                            let id = path.rsplit('/').next().unwrap();
                            json!({"id":id,"project_id":PROJECT,"harness":"aylul","profile":"agent-a","removed_at":if s.removed{Some("removed")}else{None}})
                        } else if path.ends_with("/threads") {
                            if s.gate_create {
                                gate = s.gate.take();
                            }
                            if s.create_delays > 0 {
                                s.create_delays -= 1;
                                delay = true;
                            }
                            json!({"id":T1,"project_id":PROJECT,"harness":"aylul","profile":"agent-a","removed_at":null})
                        } else if path.ends_with("/turns") {
                            if s.turn_delays > 0 {
                                s.turn_delays -= 1;
                                delay = true;
                            }
                            if let Some(code) = s.error.clone() {
                                status = if code == "INTERNAL" { 500 } else { 409 };
                                json!({"code":code,"message":"SECRET MUST NOT ESCAPE"})
                            } else {
                                let id = body["command_id"].as_str().unwrap().to_string();
                                let prior = s.turns.entry(id).or_insert_with(|| body.clone());
                                if prior != &body {
                                    status = 409;
                                    json!({"code":"COMMAND_CONFLICT"})
                                } else {
                                    json!({"operation_id":"operation-one"})
                                }
                            }
                        } else {
                            if path.ends_with("/session") {
                                gate = s.gate.take();
                            }
                            json!({"current":{"model":if path.ends_with("/model"){"default-model"}else if s.settings{"changed-model"}else{"session-model"},"mode":if s.settings{"changed-mode"}else{"ask"},"effort":"medium"}})
                        };
                        (response, delay, gate, status)
                    };
                    if let Some((entered, release)) = gate {
                        let _ = entered.send(());
                        let _ = release.recv_timeout(Duration::from_secs(3));
                    }
                    if delay {
                        thread::sleep(Duration::from_millis(350));
                    }
                    let response = response.to_string();
                    let _=write!(stream,"HTTP/1.1 {status} OK\r\nContent-Length: {}\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n{response}",response.len());
                });
            }
        });
        Self {
            state,
            stop,
            worker: Some(worker),
            origin,
        }
    }
    pub(super) fn writes(&self) -> Vec<(String, String, Value)> {
        self.state
            .lock()
            .unwrap()
            .calls
            .iter()
            .filter(|x| x.0 != "GET")
            .cloned()
            .collect()
    }
}
impl Drop for Api {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        self.worker.take().unwrap().join().unwrap();
    }
}
pub(super) fn fixture(bound: bool) -> (Api, tempfile::TempDir, Arc<Presence>) {
    let api = Api::new();
    let root = tempfile::tempdir().unwrap();
    let connection = crate::core_connection::Connection::for_binding_test(
        crate::core_connection::ReadyConnection {
            origin: api.origin.clone(),
            configuration_id: CONFIG.into(),
            runtime_id: T2.into(),
        },
    );
    let p = Presence::new(root.path().into(), connection);
    p.select(p.request(None).unwrap(), Some("agent-a".into()))
        .unwrap();
    if bound {
        p.retarget(p.request(None).unwrap(), T1.into()).unwrap();
    }
    p.inner.lock().unwrap().send.create_timeout = Duration::from_millis(200);
    p.inner.lock().unwrap().send.turn_timeout = Duration::from_millis(200);
    api.state.lock().unwrap().calls.clear();
    (api, root, p)
}
pub(super) fn submission(p: &Presence, id: &str) -> Submission {
    Submission {
        text: "مرحبا 👋\nثاني سطر".into(),
        command_id: id.into(),
        context: p.context().1.unwrap(),
    }
}
pub(super) fn id() -> String {
    uuid::Uuid::new_v4().to_string()
}
pub(super) fn gate(api: &Api) -> (mpsc::Receiver<()>, mpsc::Sender<()>) {
    let (entered, rx) = mpsc::channel();
    let (tx, release) = mpsc::channel();
    api.state.lock().unwrap().gate = Some((entered, release));
    (rx, tx)
}
