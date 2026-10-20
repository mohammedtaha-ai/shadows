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
const CONFIG: &str = "00000000-0000-4000-8000-000000000001";
const PROJECT: &str = "00000000-0000-4000-8000-000000000002";
const T1: &str = "00000000-0000-4000-8000-000000000003";
const T2: &str = "00000000-0000-4000-8000-000000000004";
#[derive(Default)]
struct Script {
    calls: Vec<(String, String, Value)>,
    create_delays: usize,
    turn_delays: usize,
    removed: bool,
    error: Option<String>,
    settings: bool,
    turns: std::collections::HashMap<String, Value>,
    gate: Option<(mpsc::Sender<()>, mpsc::Receiver<()>)>,
}
struct Api {
    state: Arc<Mutex<Script>>,
    stop: Arc<AtomicBool>,
    worker: Option<thread::JoinHandle<()>>,
    origin: String,
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
                            json!({"configuration_id":CONFIG,"runtime_id":T2})
                        } else if method == "GET" && path == "/api/aylul/agents" {
                            json!({"project_id":PROJECT,"agents":[{"slug":"agent-a","deleted":false,"default_model":"default-model"},{"slug":"agent-b","deleted":false}]})
                        } else if method == "GET" {
                            let id = path.rsplit('/').next().unwrap();
                            json!({"id":id,"project_id":PROJECT,"harness":"aylul","profile":"agent-a","removed_at":if s.removed{Some("removed")}else{None}})
                        } else if path.ends_with("/threads") {
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
    fn writes(&self) -> Vec<(String, String, Value)> {
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
fn fixture(bound: bool) -> (Api, tempfile::TempDir, Arc<Presence>) {
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
fn submission(p: &Presence, id: &str) -> Submission {
    Submission {
        text: "مرحبا 👋\nثاني سطر".into(),
        command_id: id.into(),
        context: p.context().1.unwrap(),
    }
}
fn id() -> String {
    uuid::Uuid::new_v4().to_string()
}
#[test]
fn first_send_creates_binds_opens_and_starts_in_order() {
    let (api, _root, p) = fixture(false);
    let scope = p.context().0;
    assert_eq!(p.send(submission(&p, &id())).unwrap().thread_id, T1);
    let calls = api.writes();
    assert_eq!(
        calls.iter().map(|x| x.0.as_str()).collect::<Vec<_>>(),
        ["POST", "POST", "PUT", "POST"]
    );
    assert_eq!(calls[0].1, format!("/api/projects/{PROJECT}/threads"));
    assert_eq!(calls[1].1, format!("/api/threads/{T1}/session"));
    assert_eq!(calls[2].1, format!("/api/threads/{T1}/session/model"));
    assert_eq!(calls[3].1, format!("/api/threads/{T1}/turns"));
    assert_eq!(calls[0].2["profile"], "agent-a");
    assert_eq!(calls[0].2["harness"], "aylul");
    assert_eq!(calls[0].2["title"], "New conversation");
    assert_eq!(calls[3].2["prompt"], "مرحبا 👋\nثاني سطر");
    assert_eq!(calls[3].2["mode"], "ask");
    assert_eq!(calls[3].2["effort"], "medium");
    assert_eq!(calls[3].2["model"], "default-model");
    assert_eq!(p.view().target.unwrap().thread_id, T1);
    assert_eq!(p.context().0, scope);
}
#[test]
fn bound_send_reuses_target_and_keeps_session_settings() {
    let (api, _root, p) = fixture(true);
    p.send(submission(&p, &id())).unwrap();
    let c = api.writes();
    assert_eq!(c.len(), 2);
    assert!(c[0].1.ends_with("/session"));
    assert_eq!(c[1].2["model"], "session-model");
    assert_eq!(c[1].2["mode"], "ask");
    assert_eq!(c[1].2["effort"], "medium");
}
#[test]
fn uncertain_create_retry_makes_one_thread() {
    let (api, _root, p) = fixture(false);
    api.state.lock().unwrap().create_delays = 1;
    let id = id();
    assert_eq!(
        p.send(submission(&p, &id)).unwrap_err(),
        "companion_send_uncertain"
    );
    p.send(submission(&p, &id)).unwrap();
    let c = api.writes();
    assert_eq!(c[0].2["command_id"], c[1].2["command_id"]);
    assert_eq!(api.state.lock().unwrap().turns.len(), 1);
    assert_eq!(p.view().target.unwrap().thread_id, T1);
}
#[test]
fn uncertain_turn_retry_sends_the_captured_request() {
    let (api, _root, p) = fixture(true);
    api.state.lock().unwrap().turn_delays = 1;
    let id = id();
    let original = submission(&p, &id);
    let old_context = original.context.clone();
    assert_eq!(p.send(original).unwrap_err(), "companion_send_uncertain");
    let mut changed = submission(&p, &id);
    changed.text = "changed".into();
    let before = api.writes().len();
    assert_eq!(p.send(changed).unwrap_err(), "companion_send_invalid");
    assert_eq!(api.writes().len(), before);
    p.previews(false).unwrap();
    assert_ne!(p.context().1.unwrap(), old_context);
    api.state.lock().unwrap().settings = true;
    assert_eq!(
        p.send(submission(&p, &id)).unwrap().operation_id,
        "operation-one"
    );
    let c = api.writes();
    assert_eq!(c.len(), 3);
    assert_eq!(c[1].2, c[2].2);
    assert_eq!(api.state.lock().unwrap().turns.len(), 1);
}
#[test]
fn retry_after_main_switches_conversation_is_refused() {
    let (api, _root, p) = fixture(true);
    api.state.lock().unwrap().turn_delays = 1;
    let retry = submission(&p, &id());
    assert_eq!(
        p.send(Submission {
            text: retry.text.clone(),
            command_id: retry.command_id.clone(),
            context: retry.context.clone()
        })
        .unwrap_err(),
        "companion_send_uncertain"
    );
    p.retarget(p.request(None).unwrap(), T2.into()).unwrap();
    let before = api.writes().len();
    assert_eq!(p.send(retry).unwrap_err(), "companion_send_stale");
    assert_eq!(api.writes().len(), before);
    assert!(p.inner.lock().unwrap().send.captured.is_none());
}
fn gate(api: &Api) -> (mpsc::Receiver<()>, mpsc::Sender<()>) {
    let (entered, rx) = mpsc::channel();
    let (tx, release) = mpsc::channel();
    api.state.lock().unwrap().gate = Some((entered, release));
    (rx, tx)
}
#[test]
fn late_old_send_cannot_replace_newer_attempt() {
    let (api, _root, p) = fixture(true);
    let (entered, release) = gate(&api);
    let old_id = id();
    let old = submission(&p, &old_id);
    let owner = p.clone();
    let worker = thread::spawn(move || owner.send(old));
    entered.recv_timeout(Duration::from_secs(2)).unwrap();
    p.retarget(p.request(None).unwrap(), T2.into()).unwrap();
    api.state.lock().unwrap().turn_delays = 2;
    let new_id = id();
    assert_eq!(
        p.send(submission(&p, &new_id)).unwrap_err(),
        "companion_send_uncertain"
    );
    release.send(()).unwrap();
    assert_eq!(
        worker.join().unwrap().unwrap_err(),
        "companion_send_uncertain"
    );
    let inner = p.inner.lock().unwrap();
    let captured = inner.send.captured.as_ref().unwrap();
    assert_eq!(captured.command_id, new_id);
    assert_eq!(captured.thread_id, T2);
    let calls = api.writes();
    assert!(calls
        .iter()
        .any(|c| c.1 == format!("/api/threads/{T1}/turns") && c.2["command_id"] == old_id));
}
#[test]
fn main_switching_conversation_changes_scope() {
    let (_api, _root, p) = fixture(true);
    let before = p.context().0;
    p.retarget(p.request(None).unwrap(), T2.into()).unwrap();
    assert_ne!(p.context().0, before);
}
#[test]
fn stale_context_is_refused_before_any_write() {
    let (api, _root, p) = fixture(true);
    let stale = submission(&p, &id());
    p.select(p.request(None).unwrap(), Some("agent-b".into()))
        .unwrap();
    assert_eq!(p.send(stale).unwrap_err(), "companion_send_stale");
    assert!(api.writes().is_empty());
}
#[test]
fn binding_change_mid_send_completes_on_captured_thread() {
    let (api, _root, p) = fixture(true);
    let (entered, release) = gate(&api);
    let s = submission(&p, &id());
    let owner = p.clone();
    let worker = thread::spawn(move || owner.send(s));
    entered.recv_timeout(Duration::from_secs(2)).unwrap();
    p.retarget(p.request(None).unwrap(), T2.into()).unwrap();
    release.send(()).unwrap();
    assert_eq!(worker.join().unwrap().unwrap().thread_id, T1);
    assert_eq!(p.view().target.unwrap().thread_id, T2);
}
#[test]
fn error_codes_map_to_fixed_values() {
    for (code, want) in [
        ("THREAD_BUSY", "companion_send_busy"),
        ("AYLUL_KEY_MISSING", "companion_send_setup"),
        ("MODE_NOT_ALLOWED", "companion_send_settings"),
        ("INTERNAL", "companion_send_failed"),
    ] {
        let (api, _root, p) = fixture(true);
        api.state.lock().unwrap().error = Some(code.into());
        let got = p.send(submission(&p, &id())).unwrap_err();
        assert_eq!(got, want);
        assert!(!got.contains("SECRET"));
    }
}
#[test]
fn removed_target_is_reported_not_replaced() {
    let (api, _root, p) = fixture(true);
    api.state.lock().unwrap().removed = true;
    assert_eq!(
        p.send(submission(&p, &id())).unwrap_err(),
        "companion_thread_invalid"
    );
    assert!(api.writes().is_empty());
    assert_eq!(p.view().target.unwrap().thread_id, T1);
}
#[test]
fn invalid_submissions_send_nothing() {
    let (api, _root, p) = fixture(true);
    for text in ["".to_string(), "   ".into(), "x".repeat(32769)] {
        let mut s = submission(&p, &id());
        s.text = text;
        assert_eq!(p.send(s).unwrap_err(), "companion_send_invalid");
    }
    assert_eq!(
        p.send(submission(&p, "not-uuid")).unwrap_err(),
        "companion_send_invalid"
    );
    assert!(api.state.lock().unwrap().calls.is_empty());
}
