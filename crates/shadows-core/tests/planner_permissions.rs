//! A Planner program's permission requests wait for the person, as AYLUL's do:
//! listed by `Harness::waiting_permissions` and answered through
//! `Harness::answer_permission`. Live only: run with `-- --ignored` and
//! `--test-threads=1` once the programs below are signed in.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use shadows_core::{
    AppCore, KeySource, Operation, OperationId, ProjectId, SendTurn, StartConfig, ThreadId,
    WaitingPermission,
};

struct World {
    core: Arc<AppCore>,
    thread: ThreadId,
    directory: PathBuf,
    server: tokio::task::JoinHandle<()>,
    _root: tempfile::TempDir,
}

fn path(var: &str) -> PathBuf {
    PathBuf::from(std::env::var_os(var).unwrap_or_else(|| panic!("{var} must be set")))
}

impl World {
    /// Core with Claude Code (`SHADOWS_NODE_PATH`, `SHADOWS_ADAPTER_PATH`,
    /// `SHADOWS_HARNESS_PATH`) and, when `codex`, Codex (`SHADOWS_CODEX_PATH`,
    /// `SHADOWS_CODEX_HOME`), with a thread on `harness` in an empty project.
    async fn start(harness: &str, codex: bool) -> Self {
        let root = tempfile::tempdir().unwrap();
        let directory = root.path().join("project");
        std::fs::create_dir_all(&directory).unwrap();
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let mcp = format!("http://{}/mcp", listener.local_addr().unwrap());
        let absent = root.path().join("absent");
        let claude = |var| {
            std::env::var_os(var)
                .map(PathBuf::from)
                .unwrap_or_else(|| absent.clone())
        };
        let core = AppCore::start(
            &StartConfig {
                secret_key: KeySource::Given(None),
                db_path: root.path().join("core.db"),
                node_path: claude("SHADOWS_NODE_PATH"),
                adapter_path: claude("SHADOWS_ADAPTER_PATH"),
                harness_path: claude("SHADOWS_HARNESS_PATH"),
                codex_path: codex.then(|| path("SHADOWS_CODEX_PATH")),
                codex_home: codex.then(|| path("SHADOWS_CODEX_HOME")),
                aylul: None,
            },
            mcp,
        )
        .await
        .unwrap();
        let project: ProjectId = core
            .projects()
            .create(
                "create".into(),
                "probe",
                "Permissions",
                directory.to_str().unwrap(),
            )
            .await
            .unwrap()
            .id;
        let thread = core
            .threads()
            .create("thread".into(), &project, "Planner", Some(harness), None)
            .await
            .unwrap()
            .id;
        let router = shadows_mcp::service(core.clone());
        let server = tokio::spawn(async move {
            axum::serve(listener, router).await.unwrap();
        });
        Self {
            core,
            thread,
            directory,
            server,
            _root: root,
        }
    }

    /// The model the program offers first, once its session is open.
    async fn model(&self) -> String {
        self.core.harness().open(&self.thread).await.unwrap();
        let offered = self.core.harness().offered(&self.thread).await.unwrap();
        offered.current.model
    }

    async fn send(&self, prompt: &str, mode: &str) -> OperationId {
        let model = self.model().await;
        let turn = SendTurn {
            command_id: "live".into(),
            prompt: prompt.into(),
            attachments: Vec::new(),
            model,
            mode: mode.into(),
            effort: None,
            focus: None,
            plan: None,
            client_tab: None,
        };
        self.core
            .turns()
            .send_with_default_effort(self.thread.clone(), turn)
            .await
            .unwrap()
    }

    /// The first request the thread's program holds, within three minutes.
    async fn held(&self) -> WaitingPermission {
        for _ in 0..1800 {
            let waiting = self.core.harness().waiting_permissions().await;
            if let Some(held) = waiting.into_iter().find(|w| w.thread_id == self.thread) {
                return held;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        panic!("no permission request was held");
    }

    async fn answer(&self, held: &WaitingPermission, kind: &str) {
        let option = held
            .options
            .iter()
            .find(|o| o.kind.starts_with(kind))
            .unwrap_or_else(|| panic!("no {kind} option: {held:?}"));
        self.core
            .harness()
            .answer_permission(
                "answer".into(),
                &self.thread,
                &held.request_id,
                &option.option_id,
            )
            .await
            .unwrap();
    }

    async fn settled(&self, op: &OperationId) -> Operation {
        for _ in 0..3000 {
            let operations = self.core.turns().operations(&self.thread).await.unwrap();
            if let Some(done) = operations
                .into_iter()
                .find(|o| &o.id == op && o.finished_at.is_some())
            {
                return done;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        panic!("the turn did not settle");
    }

    async fn close(self) {
        self.server.abort();
        self.core
            .shut_down(Duration::from_secs(10), std::future::pending())
            .await
            .unwrap();
    }
}

#[tokio::test]
#[ignore = "live: needs the signed-in Claude Code (SHADOWS_NODE_PATH, SHADOWS_ADAPTER_PATH, \
            SHADOWS_HARNESS_PATH)"]
async fn a_planner_permission_request_waits_for_the_person() {
    let w = World::start(shadows_agent::policy::CLAUDE_CODE, false).await;
    let op = w
        .send(
            "Run exactly this with your Bash tool, once, and then say done: \
             node -e \"require('fs').writeFileSync('asked.txt','yes')\"",
            "acceptEdits",
        )
        .await;
    let held = w.held().await;
    assert_eq!(held.agent, "Planner");
    assert!(
        !w.directory.join("asked.txt").exists(),
        "nothing runs before the person answers"
    );
    w.answer(&held, "allow").await;
    assert_eq!(w.settled(&op).await.status_kind, "Completed");
    assert!(
        w.directory.join("asked.txt").exists(),
        "the allowed command ran"
    );
    w.close().await;
}

#[tokio::test]
#[ignore = "live: needs codex (SHADOWS_CODEX_PATH) signed in to a dedicated home \
            (SHADOWS_CODEX_HOME)"]
async fn codex_approval_is_held_not_declined() {
    let w = World::start(shadows_agent::policy::CODEX, true).await;
    let op = w
        .send(
            "Run this shell command once, asking for escalated permissions since it needs \
             the network: curl -sI https://example.com. Then say what it answered.",
            shadows_agent::policy::CODEX_READ_ONLY,
        )
        .await;
    let held = w.held().await;
    assert_eq!(held.agent, "Planner");
    let ids: Vec<&str> = held.options.iter().map(|o| o.option_id.as_str()).collect();
    assert_eq!(ids, ["accept", "decline"]);
    w.answer(&held, "reject").await;
    assert_eq!(w.settled(&op).await.status_kind, "Completed");
    w.close().await;
}
