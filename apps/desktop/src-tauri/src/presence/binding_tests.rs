use super::*;
use crate::core_connection::ReadyConnection;
use std::{
    io::{Read, Write},
    net::TcpListener,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    thread,
};
struct Api {
    origin: String,
    permissions: Arc<Mutex<serde_json::Value>>,
    posts: Arc<Mutex<Vec<(String, serde_json::Value)>>>,
    stop: Arc<AtomicBool>,
    worker: Option<thread::JoinHandle<()>>,
}
impl Api {
    fn new() -> Self {
        let socket = TcpListener::bind("127.0.0.1:0").unwrap();
        socket.set_nonblocking(true).unwrap();
        let origin = format!("http://{}", socket.local_addr().unwrap());
        let stop = Arc::new(AtomicBool::new(false));
        let cancelled = stop.clone();
        let permissions = Arc::new(Mutex::new(serde_json::json!([])));
        let posts = Arc::new(Mutex::new(vec![]));
        let (listed, recorded) = (permissions.clone(), posts.clone());
        let worker = thread::spawn(move || {
            while !cancelled.load(Ordering::Relaxed) {
                let Ok((mut stream, _)) = socket.accept() else {
                    thread::sleep(std::time::Duration::from_millis(2));
                    continue;
                };
                stream.set_nonblocking(false).unwrap();
                stream
                    .set_read_timeout(Some(std::time::Duration::from_secs(1)))
                    .unwrap();
                let mut raw = Vec::new();
                let mut chunk = [0u8; 8192];
                let (head, body) = loop {
                    let count = stream.read(&mut chunk).unwrap();
                    raw.extend_from_slice(&chunk[..count]);
                    let text = String::from_utf8_lossy(&raw).into_owned();
                    if let Some((head, rest)) = text.split_once("\r\n\r\n") {
                        let length = head
                            .lines()
                            .find_map(|line| {
                                line.to_lowercase()
                                    .strip_prefix("content-length:")
                                    .and_then(|value| value.trim().parse::<usize>().ok())
                            })
                            .unwrap_or(0);
                        if rest.len() >= length {
                            break (head.to_owned(), rest.to_owned());
                        }
                    }
                };
                let method = head.split_whitespace().next().unwrap().to_owned();
                let path = head.split_whitespace().nth(1).unwrap().to_owned();
                let path = path.as_str();
                let mut status = 200;
                if method == "POST" {
                    recorded.lock().unwrap().push((
                        path.to_owned(),
                        serde_json::from_str(&body).unwrap_or_default(),
                    ));
                }
                let body=if method == "POST" {
                    let code = path.rsplit('/').next().unwrap();
                    let (code_status, error) = match code {
                        "settled" => (409, "PERMISSION_SETTLED"),
                        "missing" => (404, "PERMISSION_NOT_FOUND"),
                        "unoffered" => (422, "PERMISSION_OPTION_NOT_OFFERED"),
                        "broken" => (500, "STORAGE_UNAVAILABLE"),
                        _ => (200, ""),
                    };
                    status = code_status;
                    if error.is_empty() { serde_json::json!({"request_id":code}) } else { serde_json::json!({"code":error,"message":"SECRET MUST NOT ESCAPE"}) }
                } else if path=="/api/aylul/permissions" {
                    listed.lock().unwrap().clone()
                } else if path=="/api/runtime" {serde_json::json!({"runtime_id":"00000000-0000-4000-8000-000000000009","configuration_id":CONFIG})} else if path=="/api/aylul/agents" {
    serde_json::json!({"project_id":PROJECT,"agents":[
     {"slug":"agent-a","deleted":false},{"slug":"agent-b","deleted":false},{"slug":"deleted","deleted":true}
    ]})
   } else {
    let id=path.rsplit('/').next().unwrap();
    serde_json::json!({"id":id,"project_id":PROJECT,"harness":if id==WRONG{"claude"}else{"aylul"},"profile":if id==OTHER{"agent-b"}else{"agent-a"},"removed_at":if id==REMOVED{Some("2026-10-20")}else{None}})
   }.to_string();
                write!(stream,"HTTP/1.1 {status} OK\r\nContent-Length: {}\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n{}",body.len(),body).unwrap();
            }
        });
        Self {
            origin,
            permissions,
            posts,
            stop,
            worker: Some(worker),
        }
    }
}
impl Drop for Api {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        let joined = self.worker.take().unwrap().join();
        assert!(
            joined.is_ok() || thread::panicking(),
            "HTTP fixture worker failed"
        );
    }
}
const CONFIG: &str = "00000000-0000-4000-8000-000000000001";
const PROJECT: &str = "00000000-0000-4000-8000-000000000002";
const THREAD: &str = "00000000-0000-4000-8000-000000000003";
const WRONG: &str = "00000000-0000-4000-8000-000000000004";
const OTHER: &str = "00000000-0000-4000-8000-000000000005";
const REMOVED: &str = "00000000-0000-4000-8000-000000000006";
fn fixture() -> (Api, tempfile::TempDir, Arc<Presence>) {
    let api = Api::new();
    let root = tempfile::tempdir().unwrap();
    let connection = crate::core_connection::Connection::for_binding_test(ReadyConnection {
        origin: api.origin.clone(),
        configuration_id: CONFIG.into(),
        runtime_id: "00000000-0000-4000-8000-000000000009".into(),
    });
    let presence = Presence::new(root.path().to_path_buf(), connection);
    (api, root, presence)
}
#[tokio::test]
async fn companion_owns_observer_start_sync_and_stop() {
    let (_api, _root, presence) = fixture();
    let (sender, mut updates) = tokio::sync::mpsc::unbounded_channel();
    presence.start_watcher(Arc::new(move |update| {
        let _ = sender.send(update);
    }));
    let initial = tokio::time::timeout(std::time::Duration::from_secs(2), updates.recv())
        .await
        .unwrap()
        .unwrap();
    assert!(initial.is_current());
    presence.sync_watcher(presence.view(), None);
    assert!(
        !initial.is_current(),
        "sync replaces the owned observer epoch"
    );
    let synchronized = tokio::time::timeout(std::time::Duration::from_secs(2), updates.recv())
        .await
        .unwrap()
        .unwrap();
    assert!(synchronized.is_current());
    presence.stop();
    assert!(
        !synchronized.is_current(),
        "stop invalidates pending observer publications"
    );
    let stopped = tokio::time::timeout(std::time::Duration::from_secs(2), updates.recv())
        .await
        .unwrap()
        .unwrap();
    assert!(stopped.is_current());
    assert_eq!(
        stopped.projection.status,
        "Core unavailable. Retry local Core in Settings."
    );
}
#[test]
fn binding_changes_clear_target_and_persist_without_content() {
    let (_api, root, presence) = fixture();
    let a = presence
        .select(presence.request(None).unwrap(), Some("agent-a".into()))
        .unwrap();
    let target = presence
        .retarget(presence.request(Some(a.generation)).unwrap(), THREAD.into())
        .unwrap();
    assert_eq!(target.target.as_ref().unwrap().thread_id, THREAD);
    let b = presence
        .select(presence.request(None).unwrap(), Some("agent-b".into()))
        .unwrap();
    assert_eq!(b.agent.as_deref(), Some("agent-b"));
    assert_eq!(b.target, None);
    assert!(b.generation > target.generation);
    let restarted = Presence::new(root.path().to_path_buf(), presence.connection.clone());
    assert_eq!(restarted.snapshot().unwrap().agent, b.agent);
    let stored = std::fs::read_to_string(root.path().join("desktop-presence.json")).unwrap();
    for forbidden in ["prompt", "reply", "token", "key", "message"] {
        assert!(!stored.contains(forbidden));
    }
    let same = presence
        .select(presence.request(None).unwrap(), Some("agent-b".into()))
        .unwrap();
    assert_eq!(same, b);
}
#[test]
fn validation_rejects_deleted_unknown_and_wrong_thread() {
    let (_api, _root, presence) = fixture();
    for agent in ["missing", "deleted"] {
        assert_eq!(
            presence
                .select(presence.request(None).unwrap(), Some(agent.into()))
                .unwrap_err(),
            "companion_agent_unavailable"
        );
    }
    let binding = presence
        .select(presence.request(None).unwrap(), Some("agent-a".into()))
        .unwrap();
    for thread in [WRONG, OTHER, REMOVED, "../escape"] {
        assert!(presence
            .retarget(
                presence.request(Some(binding.generation)).unwrap(),
                thread.into()
            )
            .is_err());
    }
    assert!(presence.snapshot().unwrap().target.is_none());
}
#[test]
fn latest_explicit_retarget_wins_and_old_generation_is_rejected() {
    let (_api, _root, presence) = fixture();
    let a = presence
        .select(presence.request(None).unwrap(), Some("agent-a".into()))
        .unwrap();
    let old = presence.request(Some(a.generation)).unwrap();
    let latest = presence.request(Some(a.generation)).unwrap();
    let after = presence.retarget(latest, THREAD.into()).unwrap();
    assert_eq!(
        presence.retarget(old, THREAD.into()).unwrap_err(),
        "companion_binding_changed"
    );
    assert_eq!(presence.snapshot().unwrap(), after);
    assert!(presence.request(Some(a.generation)).is_err());
}
#[test]
fn database_change_invalidates_inflight_work_and_clears_selection() {
    let (_api, _root, presence) = fixture();
    presence
        .select(presence.request(None).unwrap(), Some("agent-a".into()))
        .unwrap();
    let old = presence.request(None).unwrap();
    presence.configuration_changed(Some("00000000-0000-4000-8000-000000000007".into()));
    assert!(presence.select(old, Some("agent-b".into())).is_err());
    let after = presence.snapshot().unwrap();
    assert_eq!(after.agent, None);
    assert_eq!(after.target, None);
}
#[test]
fn corrupt_preferences_fail_without_silently_replacing_them() {
    let (_api, root, presence) = fixture();
    std::fs::write(root.path().join("desktop-presence.json"), "{broken").unwrap();
    let restarted = Presence::new(root.path().to_path_buf(), presence.connection.clone());
    assert_eq!(
        restarted.snapshot().unwrap_err(),
        "companion_preferences_invalid"
    );
    assert_eq!(
        std::fs::read_to_string(root.path().join("desktop-presence.json")).unwrap(),
        "{broken"
    );
    assert_eq!(
        restarted
            .select(restarted.request(None).unwrap(), None)
            .unwrap()
            .agent,
        None
    );
}
#[test]
fn click_destination_is_closed_and_does_not_create_a_thread() {
    let (_api, _root, presence) = fixture();
    assert_eq!(
        presence.destination(),
        crate::navigation::MainDestination::CompanionSettings
    );
    presence
        .select(presence.request(None).unwrap(), Some("agent-a".into()))
        .unwrap();
    assert_eq!(
        presence.destination(),
        crate::navigation::MainDestination::AgentDraft {
            agent: "agent-a".into()
        }
    );
    let view = presence.snapshot().unwrap();
    presence
        .retarget(
            presence.request(Some(view.generation)).unwrap(),
            THREAD.into(),
        )
        .unwrap();
    assert_eq!(
        presence.destination(),
        crate::navigation::MainDestination::Conversation {
            project_id: PROJECT.into(),
            thread_id: THREAD.into()
        }
    );
}
#[test]
fn named_destinations_resolve_with_reads_only_and_closed_fallbacks() {
    use crate::navigation::MainDestination;
    let (_api, _root, presence) = fixture();
    assert_eq!(
        presence.destination_for(MainTarget::NewAgent),
        MainDestination::NewAgent
    );
    assert_eq!(
        presence.destination_for(MainTarget::Settings),
        MainDestination::Settings
    );
    assert_eq!(
        presence.destination_for(MainTarget::AgentSettings),
        MainDestination::CompanionSettings
    );
    presence
        .select(presence.request(None).unwrap(), Some("agent-a".into()))
        .unwrap();
    assert_eq!(
        presence.destination_for(MainTarget::AgentSettings),
        MainDestination::AgentSettings {
            agent: "agent-a".into()
        }
    );
    assert_eq!(
        presence.destination_for(MainTarget::Bound),
        presence.destination()
    );
    assert_eq!(
        presence.destination_for(MainTarget::OldestWaiting),
        MainDestination::CompanionSettings
    );
    presence.configuration_changed(Some("other".into()));
    assert_eq!(
        presence.destination_for(MainTarget::AgentSettings),
        MainDestination::CompanionSettings
    );
}
#[test]
fn oldest_waiting_resolves_the_first_entry_and_falls_back_when_it_does_not_resolve() {
    use crate::navigation::MainDestination;
    let (api, _root, presence) = fixture();
    assert_eq!(
        presence.destination_for(MainTarget::OldestWaiting),
        MainDestination::CompanionSettings
    );
    *api.permissions.lock().unwrap() = serde_json::json!([
        {"thread_id":THREAD,"agent":"agent-a","request_id":"first","title":"Run","options":[]},
        {"thread_id":OTHER,"agent":"agent-b","request_id":"second","title":"Run","options":[]}
    ]);
    assert_eq!(
        presence.destination_for(MainTarget::OldestWaiting),
        MainDestination::Conversation {
            project_id: PROJECT.into(),
            thread_id: THREAD.into()
        }
    );
    *api.permissions.lock().unwrap() = serde_json::json!([
        {"thread_id":WRONG,"agent":"agent-a","request_id":"first","title":"Run","options":[]}
    ]);
    assert_eq!(
        presence.destination_for(MainTarget::OldestWaiting),
        MainDestination::CompanionSettings
    );
}
#[test]
fn permission_answers_post_once_with_the_command_id_and_map_core_errors_to_fixed_codes() {
    let (api, _root, presence) = fixture();
    presence
        .select(presence.request(None).unwrap(), Some("agent-a".into()))
        .unwrap();
    presence
        .retarget(presence.request(None).unwrap(), THREAD.into())
        .unwrap();
    let command = uuid::Uuid::new_v4().to_string();
    let answer = |thread: &str, request: &str, command: &str| {
        tauri::async_runtime::block_on(presence.answer_companion_permission(
            thread.into(),
            request.into(),
            "allow_once".into(),
            command.into(),
        ))
    };
    assert!(answer(THREAD, "granted", &command).unwrap().ok);
    {
        let posts = api.posts.lock().unwrap();
        assert_eq!(posts.len(), 1);
        assert_eq!(
            posts[0].0,
            format!("/api/threads/{THREAD}/session/permissions/granted")
        );
        assert_eq!(
            posts[0].1,
            serde_json::json!({"command_id":command,"option_id":"allow_once"})
        );
    }
    for (request, code) in [
        ("settled", "permission_settled"),
        ("missing", "permission_not_found"),
        ("unoffered", "permission_option_not_offered"),
        ("broken", "permission_failed"),
    ] {
        assert_eq!(answer(THREAD, request, &command).unwrap_err(), code);
    }
    let posts = api.posts.lock().unwrap().len();
    for (thread, request, command) in [
        (OTHER, "granted", command.as_str()),
        (THREAD, "../granted", command.as_str()),
        (THREAD, "granted", "not-a-uuid"),
    ] {
        assert!(answer(thread, request, command).is_err());
    }
    assert_eq!(
        api.posts.lock().unwrap().len(),
        posts,
        "refused answers send nothing"
    );
}
#[tokio::test]
async fn navigation_rejects_unknown_targets_before_window_work() {
    let (_api, _root, presence) = fixture();
    for target in ["", "url", "https://example.com", "../settings"] {
        assert_eq!(
            presence.open_main_from_companion(Some(target.into())).await,
            Err("navigation_invalid".into())
        );
    }
}

#[test]
fn write_presence_fixture() {
    let (_api, _root, presence) = fixture();
    let unbound = presence.snapshot().unwrap();
    let binding = presence
        .select(presence.request(None).unwrap(), Some("agent-a".into()))
        .unwrap();
    let target = presence
        .retarget(
            presence.request(Some(binding.generation)).unwrap(),
            THREAD.into(),
        )
        .unwrap();
    let navigation = crate::navigation::Navigation::default();
    let destinations = [
        navigation.request(crate::navigation::MainDestination::CompanionSettings),
        navigation.request(crate::navigation::MainDestination::AgentDraft {
            agent: "agent-a".into(),
        }),
        navigation.request(crate::navigation::MainDestination::Conversation {
            project_id: PROJECT.into(),
            thread_id: THREAD.into(),
        }),
        navigation.request(crate::navigation::MainDestination::AgentSettings {
            agent: "agent-a".into(),
        }),
        navigation.request(crate::navigation::MainDestination::NewAgent),
        navigation.request(crate::navigation::MainDestination::Settings),
    ];
    let active_presence = super::projection::PresenceProjection {
        expression: "success".into(),
        status: "Reply ready".into(),
        preview: Some(super::projection::Preview {
            id: "preview-1".into(),
            text: "تم تجهيز الرد".into(),
            expires_at_ms: 10_000,
        }),
    };
    let mut companion = crate::windows::companion::state::Model::default();
    companion.present(
        Default::default(),
        Some(crate::windows::companion::SurfaceAppearance {
            color: "blue".into(),
            outfit: Some("opaque outfit".into()),
        }),
    );
    let fixture = serde_json::json!({"unbound":unbound,"binding":binding,"target":target,"destinations":destinations,"companion":companion.view,"activePresence":active_presence});
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../web/src/test/native-presence.json");
    if std::env::var("UPDATE_NATIVE_FIXTURES").as_deref() == Ok("1") {
        std::fs::write(
            &path,
            serde_json::to_string_pretty(&fixture).unwrap() + "\n",
        )
        .unwrap();
    }
    let saved: serde_json::Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    assert_eq!(saved, fixture);
}
