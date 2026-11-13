//! The Planner asks a real language server: gopls, through `code_lsp`, on the
//! signed-in Claude Code and Codex. Live only: run with `-- --ignored` and
//! `--test-threads=1` once gopls is on PATH and the programs are signed in.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use shadows_core::{
    AppCore, KeySource, OperationId, ProjectId, SendTurn, StartConfig, ThreadEntryKind, ThreadId,
};

const GO_MOD: &str = "module example.com/demo\n\ngo 1.21\n";
const A_GO: &str = "package demo\n\nfunc Hello() string {\n\treturn \"hi\"\n}\n";
const B_GO: &str = "package demo\n\nfunc Greet() string {\n\treturn Hello()\n}\n";
const C_GO: &str = "package demo\n\nfunc Wave() string {\n\treturn Hello() + \"!\"\n}\n";
const ASK: &str = "Which files call Hello? Use code_lsp. Answer with the file names.";

struct World {
    core: Arc<AppCore>,
    thread: ThreadId,
    server: tokio::task::JoinHandle<()>,
    _root: tempfile::TempDir,
}

fn path(var: &str) -> PathBuf {
    PathBuf::from(std::env::var_os(var).unwrap_or_else(|| panic!("{var} must be set")))
}

impl World {
    /// Core with Claude Code (`SHADOWS_NODE_PATH`, `SHADOWS_ADAPTER_PATH`,
    /// `SHADOWS_HARNESS_PATH`) and, when `codex`, Codex (`SHADOWS_CODEX_PATH`,
    /// `SHADOWS_CODEX_HOME`), with a Planner thread on `harness` in a Go module
    /// whose `b.go` and `c.go` call `Hello` from `a.go`.
    async fn start(harness: &str, codex: bool) -> Self {
        let root = tempfile::tempdir().unwrap();
        let directory = root.path().join("project");
        std::fs::create_dir_all(&directory).unwrap();
        for (name, text) in [
            ("go.mod", GO_MOD),
            ("a.go", A_GO),
            ("b.go", B_GO),
            ("c.go", C_GO),
        ] {
            std::fs::write(directory.join(name), text).unwrap();
        }
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
            .create("create".into(), "go", "Go", directory.to_str().unwrap())
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
            server,
            _root: root,
        }
    }

    async fn send(&self, prompt: &str, mode: &str) -> OperationId {
        self.core.harness().open(&self.thread).await.unwrap();
        let offered = self.core.harness().offered(&self.thread).await.unwrap();
        let turn = SendTurn {
            command_id: "live".into(),
            prompt: prompt.into(),
            attachments: Vec::new(),
            model: offered.current.model,
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

    /// Waits up to five minutes for the turn, allowing any request the
    /// program holds on the way, and returns its status.
    async fn settled(&self, op: &OperationId) -> String {
        for _ in 0..3000 {
            for held in self.core.harness().waiting_permissions().await {
                if held.thread_id != self.thread {
                    continue;
                }
                let allow = held
                    .options
                    .iter()
                    .find(|o| o.kind.starts_with("allow") || o.option_id == "accept");
                if let Some(option) = allow {
                    let _ = self
                        .core
                        .harness()
                        .answer_permission(
                            format!("allow-{}", held.request_id),
                            &self.thread,
                            &held.request_id,
                            &option.option_id,
                        )
                        .await;
                }
            }
            let operations = self.core.turns().operations(&self.thread).await.unwrap();
            if let Some(done) = operations
                .into_iter()
                .find(|o| &o.id == op && o.finished_at.is_some())
            {
                return done.status_kind;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        panic!("the turn did not settle");
    }

    /// The turn called `code_lsp`, and its last reply names both callers.
    async fn assert_answered(&self) {
        let entries = self.core.threads().entries(&self.thread).await.unwrap();
        let called = entries
            .iter()
            .any(|e| e.kind == ThreadEntryKind::ToolCall && format!("{e:?}").contains("code_lsp"));
        assert!(called, "code_lsp was not called: {entries:?}");
        let reply = entries
            .iter()
            .rev()
            .find(|e| e.kind == ThreadEntryKind::AgentMessage)
            .expect("an agent reply")
            .body
            .clone();
        println!("live code_lsp reply: {reply}");
        assert!(reply.contains("b.go") && reply.contains("c.go"), "{reply}");
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
#[ignore = "live: needs gopls on PATH and the signed-in Claude Code (SHADOWS_NODE_PATH, \
            SHADOWS_ADAPTER_PATH, SHADOWS_HARNESS_PATH)"]
async fn live_a_planner_asks_gopls() {
    let w = World::start(shadows_agent::policy::CLAUDE_CODE, false).await;
    let op = w.send(ASK, "acceptEdits").await;
    assert_eq!(w.settled(&op).await, "Completed");
    w.assert_answered().await;
    w.close().await;
}

#[tokio::test]
#[ignore = "live: needs gopls on PATH and codex (SHADOWS_CODEX_PATH) signed in to a dedicated \
            home (SHADOWS_CODEX_HOME)"]
async fn live_a_codex_planner_asks_gopls() {
    let w = World::start(shadows_agent::policy::CODEX, true).await;
    let op = w.send(ASK, shadows_agent::policy::CODEX_ASK).await;
    assert_eq!(w.settled(&op).await, "Completed");
    w.assert_answered().await;
    w.close().await;
}
