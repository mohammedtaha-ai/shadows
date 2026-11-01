use super::*;
use crate::core_connection::ReadyConnection;
use std::{
    io::{Read, Write},
    net::TcpListener,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
};
struct Api {
    origin: String,
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
                let mut bytes = [0u8; 8192];
                let count = stream.read(&mut bytes).unwrap();
                let request = String::from_utf8_lossy(&bytes[..count]);
                assert!(request.starts_with("GET "));
                let path = request.split_whitespace().nth(1).unwrap();
                let body=if path=="/api/runtime" {serde_json::json!({"runtime_id":"00000000-0000-4000-8000-000000000009","configuration_id":CONFIG})} else if path=="/api/aylul/agents" {
    serde_json::json!({"project_id":PROJECT,"agents":[
     {"slug":"agent-a","deleted":false},{"slug":"agent-b","deleted":false},{"slug":"deleted","deleted":true}
    ]})
   } else {
    let id=path.rsplit('/').next().unwrap();
    serde_json::json!({"id":id,"project_id":PROJECT,"harness":if id==WRONG{"claude"}else{"aylul"},"profile":if id==OTHER{"agent-b"}else{"agent-a"},"removed_at":if id==REMOVED{Some("2026-10-20")}else{None}})
   }.to_string();
                write!(stream,"HTTP/1.1 200 OK\r\nContent-Length: {}\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n{}",body.len(),body).unwrap();
            }
        });
        Self {
            origin,
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
