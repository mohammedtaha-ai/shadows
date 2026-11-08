use axum::body::{Body, to_bytes};
use axum::extract::{Request, State};
use axum::middleware::{self, Next};
use axum::response::Response;
use serde_json::{Value, json};
use shadows_agent::policy::CODEX_READ_ONLY;
use shadows_core::{
    AppCore, KeySource, Operation, OperationId, ProjectId, SendTurn, StartConfig, ThreadId,
};
use std::path::PathBuf;
use std::sync::{
    Arc, Mutex, OnceLock,
    atomic::{AtomicBool, Ordering},
};
use std::time::Duration;
use tokio::sync::Notify;

#[derive(Default)]
struct Audit {
    bearers: Mutex<Vec<String>>,
    hold_call: AtomicBool,
    held: Notify,
    release: Notify,
}

impl Audit {
    fn last_bearer(&self) -> String {
        self.bearers
            .lock()
            .unwrap()
            .last()
            .expect("MCP contacted")
            .clone()
    }
}

async fn observe(State(audit): State<Arc<Audit>>, request: Request, next: Next) -> Response {
    if let Some(token) = request
        .headers()
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
    {
        audit.bearers.lock().unwrap().push(token.to_owned());
    }
    if !audit.hold_call.load(Ordering::Acquire) {
        return next.run(request).await;
    }
    let (parts, body) = request.into_parts();
    let bytes = to_bytes(body, 64 * 1024).await.unwrap();
    let call = serde_json::from_slice::<Value>(&bytes).is_ok_and(|v| v["method"] == "tools/call");
    if call && audit.hold_call.swap(false, Ordering::AcqRel) {
        audit.held.notify_one();
        audit.release.notified().await;
    }
    next.run(Request::from_parts(parts, Body::from(bytes)))
        .await
}

fn fixture_executable() -> PathBuf {
    static BIN: OnceLock<PathBuf> = OnceLock::new();
    BIN.get_or_init(|| {
        let executable = std::env::current_exe().unwrap();
        let path = executable
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join(format!("fake-codex{}", std::env::consts::EXE_SUFFIX));
        if path.is_file() {
            return path;
        }
        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../fake-codex/Cargo.toml");
        escargot::CargoBuild::new()
            .manifest_path(manifest)
            .bin("fake-codex")
            .features("test-support")
            .run()
            .unwrap()
            .path()
            .to_owned()
    })
    .clone()
}

struct World {
    core: Arc<AppCore>,
    project: ProjectId,
    thread: ThreadId,
    audit: Arc<Audit>,
    server: tokio::task::JoinHandle<()>,
    _root: tempfile::TempDir,
}

impl World {
    async fn start(fail_setup: bool) -> Self {
        let root = tempfile::tempdir().unwrap();
        let home = root.path().join("codex-home");
        std::fs::create_dir(&home).unwrap();
        if fail_setup {
            std::fs::write(home.join("fail-codex-setup"), "").unwrap();
        }
        let directory = root.path().join("project");
        std::fs::create_dir(&directory).unwrap();
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let mcp = format!("http://{}/mcp", listener.local_addr().unwrap());
        let absent = root.path().join("absent");
        let core = AppCore::start(
            &StartConfig {
                secret_key: KeySource::Given(None),
                db_path: root.path().join("core.db"),
                node_path: absent.clone(),
                adapter_path: absent.clone(),
                harness_path: absent,
                codex_path: Some(fixture_executable()),
                codex_home: Some(home),
                aylul: None,
            },
            mcp,
        )
        .await
        .unwrap();
        let project = core
            .projects()
            .create(
                "create".into(),
                "probe",
                "Codex project",
                directory.to_str().unwrap(),
            )
            .await
            .unwrap()
            .id;
        core.projects()
            .set_modes(
                "modes".into(),
                &project,
                [("codex".into(), vec![CODEX_READ_ONLY.into()])].into(),
            )
            .await
            .unwrap();
        core.design()
            .seed_roadmap_for_tests(&project)
            .await
            .unwrap();
        let thread = core
            .threads()
            .create("thread".into(), &project, "Codex", Some("codex"), None)
            .await
            .unwrap()
            .id;
        let audit = Arc::new(Audit::default());
        let router = shadows_mcp::service(core.clone())
            .layer(middleware::from_fn_with_state(audit.clone(), observe));
        let server = tokio::spawn(async move {
            axum::serve(listener, router).await.unwrap();
        });
        Self {
            core,
            project,
            thread,
            audit,
            server,
            _root: root,
        }
    }

    async fn start_turn(&self, command: &str, prompt: &str) -> OperationId {
        self.core
            .turns()
            .send(
                self.thread.clone(),
                SendTurn {
                    command_id: command.into(),
                    prompt: prompt.into(),
                    model: "fixture-model".into(),
                    mode: CODEX_READ_ONLY.into(),
                    effort: Some("low".into()),
                    attachments: Vec::new(),
                    client_tab: None,
                    focus: None,
                    plan: None,
                },
            )
            .await
            .unwrap()
    }

    async fn done(&self, operation: &OperationId) -> Operation {
        tokio::time::timeout(Duration::from_secs(12), async {
            loop {
                let settled = self
                    .core
                    .turns()
                    .operations(&self.thread)
                    .await
                    .unwrap()
                    .into_iter()
                    .find(|op| &op.id == operation && op.finished_at.is_some());
                if let Some(settled) = settled {
                    return settled;
                }
                tokio::time::sleep(Duration::from_millis(25)).await;
            }
        })
        .await
        .expect("operation reached a durable terminal status")
    }

    async fn close(self) {
        self.core
            .shut_down(Duration::from_secs(4), std::future::pending())
            .await
            .unwrap();
        self.server.abort();
    }
}

#[tokio::test]
async fn codex_uses_shadows_mcp() {
    let w = World::start(false).await;
    let page = w.core.design().parts(&w.project, None, None).await.unwrap();
    let input =
        json!({"title":"Native draft","goal":"Persist through MCP","parts":[page.items[0].id]});
    let operation = w
        .start_turn("draft", &format!("mcp draft_start {input}"))
        .await;
    let settled = w.done(&operation).await;
    assert_eq!(settled.status_kind, "Completed");
    let plans = w.core.plans().list(&w.project, false).await.unwrap();
    assert_eq!(plans.len(), 1);
    let entries = w.core.threads().entries(&w.thread).await.unwrap();
    assert!(
        entries
            .iter()
            .any(|entry| entry.body.contains("workflow_id")
                && entry.operation_id.as_deref() == Some(operation.as_str()))
    );
    let bearer = w.audit.last_bearer();
    assert!(w.core.grants().authorize(&bearer).await.unwrap().is_some());
    w.close().await;
}

#[tokio::test]
async fn codex_resume_rotates_the_grant() {
    let w = World::start(false).await;
    let first = w.start_turn("first", "mcp workspace_get {}").await;
    assert_eq!(w.done(&first).await.status_kind, "Completed");
    let old = w.audit.last_bearer();
    w.core.harness().terminate(&w.thread).await.unwrap();
    assert!(w.core.grants().authorize(&old).await.unwrap().is_none());
    let second = w.start_turn("second", "mcp workspace_get {}").await;
    assert_eq!(w.done(&second).await.status_kind, "Completed");
    let fresh = w.audit.last_bearer();
    assert!(old != fresh, "a new opening has a distinct bearer");
    assert!(w.core.grants().authorize(&fresh).await.unwrap().is_some());
    assert!(w.core.grants().authorize(&old).await.unwrap().is_none());
    w.close().await;
}

#[tokio::test]
async fn codex_failed_setup_revokes_the_grant() {
    let w = World::start(true).await;
    let error = w
        .core
        .harness()
        .open(&w.thread)
        .await
        .err()
        .expect("unsafe inventory rejected");
    assert!(error.to_string().contains("unexpected MCP tools"));
    let failed = w.audit.last_bearer();
    assert!(w.core.grants().authorize(&failed).await.unwrap().is_none());
    assert_eq!(w.core.harness().live_count().await, 0);
    w.close().await;
}

#[tokio::test]
async fn codex_refuses_forbidden_authority() {
    let w = World::start(false).await;
    let operation = w.start_turn("refusal", "approval").await;
    assert_eq!(w.done(&operation).await.status_kind, "Completed");
    let entries = w.core.threads().entries(&w.thread).await.unwrap();
    assert!(entries.iter().any(|entry| entry.body == "refused"));
    assert!(
        w.core
            .plans()
            .list(&w.project, false)
            .await
            .unwrap()
            .is_empty()
    );
    w.close().await;
}

#[tokio::test]
async fn codex_stop_during_a_tool_call() {
    let w = World::start(false).await;
    w.audit.hold_call.store(true, Ordering::Release);
    let operation = w.start_turn("pending", "mcp pending").await;
    tokio::time::timeout(Duration::from_secs(8), w.audit.held.notified())
        .await
        .unwrap();
    let stopped = w.core.turns().stop(&operation).await.unwrap();
    assert!(stopped.finished_at.is_some());
    assert_eq!(w.done(&operation).await.status_kind, "Cancelled");
    w.audit.release.notify_one();
    assert!(
        w.core
            .plans()
            .list(&w.project, false)
            .await
            .unwrap()
            .is_empty()
    );
    w.close().await;
}
