use axum::body::{Body, to_bytes};
use axum::extract::{Request, State};
use axum::middleware::{self, Next};
use axum::response::Response;
use serde_json::{Value, json};
use shadows_agent::policy::{CODEX_ASK, CODEX_READ_ONLY};
use shadows_core::{
    AppCore, KeySource, Operation, OperationId, ProjectId, Queued, SendTurn, SentNow, StartConfig,
    ThreadId, TurnsError,
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
        Self::start_in(tempfile::tempdir().unwrap(), fail_setup).await
    }

    /// Starts Core over `root`'s `core.db`, which a test may have seeded.
    async fn start_in(root: tempfile::TempDir, fail_setup: bool) -> Self {
        let home = root.path().join("codex-home");
        std::fs::create_dir(&home).unwrap();
        if fail_setup {
            std::fs::write(home.join("fail-codex-setup"), "").unwrap();
        }
        Self::start_with(root, fixture_executable(), home).await
    }

    /// Starts the real Codex program in `root`'s project folder, which a test
    /// may have filled.
    async fn start_live(root: tempfile::TempDir) -> Self {
        let program = std::env::var_os("SHADOWS_CODEX_PATH")
            .expect("SHADOWS_CODEX_PATH must name the signed-in Codex program");
        let home = std::env::var_os("SHADOWS_CODEX_HOME")
            .expect("SHADOWS_CODEX_HOME must name its dedicated signed-in home");
        Self::start_with(root, PathBuf::from(program), PathBuf::from(home)).await
    }

    /// Starts Core with `program` as Codex over `home`, and a Codex thread in
    /// `root`'s project folder.
    async fn start_with(root: tempfile::TempDir, program: PathBuf, home: PathBuf) -> Self {
        let directory = root.path().join("project");
        std::fs::create_dir_all(&directory).unwrap();
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
                codex_path: Some(program),
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
        self.send(&self.thread, command, prompt, "fixture-model", Some("low"))
            .await
            .unwrap()
    }

    async fn send(
        &self,
        thread: &ThreadId,
        command: &str,
        prompt: &str,
        model: &str,
        effort: Option<&str>,
    ) -> Result<OperationId, TurnsError> {
        self.core
            .turns()
            .send(thread.clone(), turn(command, prompt, model, effort))
            .await
    }

    /// A turn in `mode`, on the fixture model.
    async fn send_in(&self, mode: &str, command: &str, prompt: &str) -> OperationId {
        let send = SendTurn {
            mode: mode.into(),
            ..turn(command, prompt, "fixture-model", Some("low"))
        };
        self.core
            .turns()
            .send(self.thread.clone(), send)
            .await
            .unwrap()
    }

    async fn done(&self, operation: &OperationId) -> Operation {
        self.settled(&self.thread, operation).await
    }

    /// The bodies a turn wrote, in entry order.
    async fn written(&self, thread: &ThreadId, operation: &OperationId) -> Vec<String> {
        self.core
            .threads()
            .entries(thread)
            .await
            .unwrap()
            .into_iter()
            .filter(|entry| entry.operation_id.as_deref() == Some(operation.as_str()))
            .map(|entry| entry.body)
            .collect()
    }

    /// The `codex` row of the harness list, as a client reads it.
    async fn codex_listed(&self) -> Value {
        let list = serde_json::to_value(self.core.harness().list().await.unwrap()).unwrap();
        list.as_array()
            .unwrap()
            .iter()
            .find(|harness| harness["kind"] == "codex")
            .unwrap()
            .clone()
    }

    async fn settled(&self, thread: &ThreadId, operation: &OperationId) -> Operation {
        self.settled_within(thread, operation, Duration::from_secs(12))
            .await
    }

    async fn settled_within(
        &self,
        thread: &ThreadId,
        operation: &OperationId,
        limit: Duration,
    ) -> Operation {
        tokio::time::timeout(limit, async {
            loop {
                let settled = self
                    .core
                    .turns()
                    .operations(thread)
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
async fn codex_approval_waits_for_the_person() {
    let w = World::start(false).await;
    let operation = w.send_in(CODEX_ASK, "refusal", "approval").await;
    let mut waiting = Vec::new();
    for _ in 0..200 {
        waiting = w.core.harness().waiting_permissions().await;
        if !waiting.is_empty() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    let [held] = waiting.as_slice() else {
        panic!("one held request: {waiting:?}");
    };
    assert_eq!(held.thread_id, w.thread);
    assert_eq!(held.agent, "Planner");
    let options: Vec<&str> = held.options.iter().map(|o| o.option_id.as_str()).collect();
    assert_eq!(options, ["accept", "decline"]);
    w.core
        .harness()
        .answer_permission("no".into(), &w.thread, &held.request_id, "decline")
        .await
        .unwrap();
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
    // Stop may answer once the turn has claimed its own ending, before that
    // ending commits; the durable status is what proves one Cancelled ending.
    w.core.turns().stop(&operation).await.unwrap();
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

fn turn(command: &str, prompt: &str, model: &str, effort: Option<&str>) -> SendTurn {
    SendTurn {
        command_id: command.into(),
        prompt: prompt.into(),
        model: model.into(),
        mode: CODEX_READ_ONLY.into(),
        effort: effort.map(str::to_owned),
        attachments: Vec::new(),
        client_tab: None,
        focus: None,
        plan: None,
    }
}

fn ids(list: &[shadows_agent::choices::Choice]) -> Vec<&str> {
    list.iter().map(|choice| choice.id.as_str()).collect()
}

#[tokio::test]
async fn codex_choices_follow_the_model_catalog() {
    let w = World::start(false).await;
    let listed = w.codex_listed().await;
    assert_eq!(listed["available"], true);
    assert!(listed["reason"].is_null());
    assert_eq!(listed["modes"][0]["id"], CODEX_ASK);
    assert_eq!(listed["modes"][1]["id"], CODEX_READ_ONLY);
    assert_eq!(listed["initial_mode"], CODEX_ASK);
    let choices = w.core.harness().open_session(&w.thread).await.unwrap();
    assert_eq!(
        ids(&choices.models),
        ["fixture-model", "fixture-small", "fixture-locked"],
        "both catalog pages, in the catalog's order"
    );
    assert_eq!(ids(&choices.efforts), ["low", "high"]);
    assert_eq!(ids(&choices.modes), [CODEX_ASK, CODEX_READ_ONLY]);
    assert!(choices.modes[0].enabled);
    assert_eq!(choices.current.model, "fixture-model");
    assert_eq!(
        choices.current.effort.as_deref(),
        Some("low"),
        "an opening without an effort runs at the model default"
    );
    assert!(choices.capabilities.image);
    assert_eq!(choices.capabilities.image_reason, None);
    w.close().await;
}

#[tokio::test]
async fn codex_efforts_belong_to_the_selected_model() {
    let w = World::start(false).await;
    let small = w
        .core
        .harness()
        .change_model(&w.thread, "fixture-small")
        .await
        .unwrap();
    assert_eq!(ids(&small.efforts), ["minimal"]);
    assert_eq!(small.current.effort.as_deref(), Some("minimal"));
    assert!(
        w.core
            .harness()
            .change_effort(&w.thread, "high")
            .await
            .is_err()
    );
    assert!(
        w.codex_listed().await["remembered"].is_null(),
        "a pick alone remembers nothing"
    );
    let refused = w
        .send(&w.thread, "high", "settings", "fixture-small", Some("high"))
        .await;
    assert!(
        matches!(refused, Err(TurnsError::SettingNotOffered { .. })),
        "{refused:?}"
    );
    let op = w
        .send(
            &w.thread,
            "small",
            "settings",
            "fixture-small",
            Some("minimal"),
        )
        .await
        .unwrap();
    assert_eq!(w.done(&op).await.status_kind, "Completed");
    let written = w.written(&w.thread, &op).await;
    assert!(
        written
            .iter()
            .any(|body| body.contains(r#"model="fixture-small" effort="minimal""#)),
        "{written:?}"
    );
    assert_eq!(
        w.codex_listed().await["remembered"],
        json!({"model": "fixture-small", "effort": "minimal"})
    );
    w.close().await;
}

#[tokio::test]
async fn codex_unavailable_model_reports_its_error() {
    let w = World::start(false).await;
    let op = w
        .send(&w.thread, "locked", "hello", "fixture-locked", Some("low"))
        .await
        .unwrap();
    let failed = w.done(&op).await;
    assert_eq!(failed.status_kind, "Failed");
    let reason = failed.failure_reason.unwrap_or_default();
    assert!(reason.contains("not available on this account"), "{reason}");
    assert!(!reason.contains("fixture-secret-value"), "{reason}");
    w.close().await;
}

#[tokio::test]
async fn codex_project_modes_are_enforced() {
    let w = World::start(false).await;
    w.core
        .projects()
        .set_modes(
            "no-codex".into(),
            &w.project,
            [("codex".into(), Vec::new())].into(),
        )
        .await
        .unwrap();
    let refused = w
        .send(&w.thread, "refused", "hello", "fixture-model", Some("low"))
        .await;
    assert!(
        matches!(&refused, Err(TurnsError::ModeNotAllowed(mode)) if mode == CODEX_READ_ONLY),
        "{refused:?}"
    );
    let outside = w
        .core
        .turns()
        .send(
            w.thread.clone(),
            SendTurn {
                mode: "workspace-write".into(),
                ..turn("outside", "hello", "fixture-model", Some("low"))
            },
        )
        .await;
    assert!(outside.is_err(), "a mode outside the policy is refused");
    let choices = w.core.harness().open_session(&w.thread).await.unwrap();
    assert!(!choices.modes[0].enabled);
    assert!(
        w.core
            .turns()
            .operations(&w.thread)
            .await
            .unwrap()
            .is_empty()
    );
    w.close().await;
}

#[tokio::test]
async fn codex_new_projects_use_the_declared_mode_policy() {
    let w = World::start(false).await;
    let project = w.core.projects().get(&w.project).await.unwrap();
    assert_eq!(project.allowed_modes["codex"], [CODEX_ASK, CODEX_READ_ONLY]);
    let choices = w.core.harness().open_session(&w.thread).await.unwrap();
    assert_eq!(choices.current.mode, CODEX_ASK);
    w.close().await;
}

#[tokio::test]
async fn codex_each_mode_sends_its_sandbox_and_approval() {
    let w = World::start(false).await;
    for (mode, words) in [
        (CODEX_READ_ONLY, r#"sandbox="readOnly" approval="never""#),
        (
            CODEX_ASK,
            r#"sandbox="workspaceWrite" approval="on-request""#,
        ),
    ] {
        let op = w.send_in(mode, mode, "settings").await;
        assert_eq!(w.done(&op).await.status_kind, "Completed");
        let written = w.written(&w.thread, &op).await;
        assert!(
            written.iter().any(|body| body.contains(words)),
            "{mode}: {written:?}"
        );
    }
    w.close().await;
}

#[tokio::test]
async fn codex_read_only_holds_no_approval() {
    let w = World::start(false).await;
    let op = w.send_in(CODEX_READ_ONLY, "quiet", "approval").await;
    assert_eq!(w.done(&op).await.status_kind, "Completed");
    assert!(w.core.harness().waiting_permissions().await.is_empty());
    let written = w.written(&w.thread, &op).await;
    assert!(
        written.iter().any(|body| body == "refused without asking"),
        "{written:?}"
    );
    w.close().await;
}

#[tokio::test]
async fn codex_modes_migrate_without_granting_write_access() {
    let root = tempfile::tempdir().unwrap();
    seed_before_0050(root.path()).await;
    let w = World::start_in(root, false).await;
    let ordinary = w
        .core
        .projects()
        .by_slug("existing")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(ordinary.allowed_modes["codex"], [CODEX_READ_ONLY]);
    assert_eq!(ordinary.allowed_modes["claude-code"], ["acceptEdits"]);
    let assistant = w.core.projects().by_slug("aylul").await.unwrap().unwrap();
    assert!(assistant.allowed_modes["codex"].is_empty());
    w.close().await;
}

/// A project that allows only Codex's `read-only` opens a new conversation
/// there, not on the default `ask` it never allowed.
#[tokio::test]
async fn codex_opens_on_the_projects_mode_when_the_default_is_not_allowed() {
    let w = World::start(false).await;
    w.core
        .projects()
        .set_modes(
            "read-only-only".into(),
            &w.project,
            [("codex".into(), vec![CODEX_READ_ONLY.into()])].into(),
        )
        .await
        .unwrap();
    let choices = w.core.harness().open_session(&w.thread).await.unwrap();
    assert_eq!(choices.current.mode, CODEX_READ_ONLY);
    let op = w.send_in(CODEX_READ_ONLY, "opened", "settings").await;
    assert_eq!(w.done(&op).await.status_kind, "Completed");
    w.close().await;
}

/// Applies the migrations before 0050 from copies of the checked-in files, so
/// their checksums match, with one ordinary and one AYLUL project that allow
/// Claude's `acceptEdits` only.
async fn seed_before_0050(root: &std::path::Path) {
    let old = root.join("pre-0050");
    std::fs::create_dir(&old).unwrap();
    for entry in std::fs::read_dir(shadows_core::testing::migrations_dir()).unwrap() {
        let path = entry.unwrap().path();
        let name = path.file_name().unwrap().to_str().unwrap().to_string();
        if name.as_str() < "0050" {
            std::fs::copy(&path, old.join(&name)).unwrap();
        }
    }
    let url = format!(
        "sqlite://{}?mode=rwc",
        root.join("core.db")
            .display()
            .to_string()
            .replace('\\', "/")
    );
    let pool = sqlx::SqlitePool::connect(&url).await.unwrap();
    sqlx::migrate::Migrator::new(old.as_path())
        .await
        .unwrap()
        .run(&pool)
        .await
        .unwrap();
    for (id, slug, assistant) in [
        ("00000000-0000-4000-8000-000000000001", "existing", 0),
        ("00000000-0000-4000-8000-000000000002", "aylul", 1),
    ] {
        sqlx::query(
            "INSERT INTO project (id, slug, name, directory, created_at, assistant)
             VALUES (?, ?, ?, ?, '2026-11-01T00:00:00Z', ?)",
        )
        .bind(id)
        .bind(slug)
        .bind(slug)
        .bind(root.join(slug).to_str().unwrap())
        .bind(assistant)
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO project_mode (project_id, harness_kind, mode_id)
             VALUES (?, 'claude-code', 'acceptEdits')",
        )
        .bind(id)
        .execute(&pool)
        .await
        .unwrap();
    }
    pool.close().await;
}

#[tokio::test]
async fn codex_resume_does_not_replay_entries() {
    let w = World::start(false).await;
    let first = w.start_turn("first", "hello").await;
    assert_eq!(w.done(&first).await.status_kind, "Completed");
    let before = w.core.threads().entries(&w.thread).await.unwrap().len();
    w.core.harness().terminate(&w.thread).await.unwrap();
    let second = w.start_turn("second", "hello").await;
    assert_eq!(w.done(&second).await.status_kind, "Completed");
    let entries = w.core.threads().entries(&w.thread).await.unwrap();
    assert!(
        entries
            .iter()
            .all(|entry| !entry.body.contains("replayed history"))
    );
    let written = w.written(&w.thread, &second).await;
    assert_eq!(entries.len() - before, written.len());
    assert_eq!(written, w.written(&w.thread, &first).await);
    w.close().await;
}

#[tokio::test]
async fn codex_usage_is_last_request_not_cumulative() {
    let w = World::start(false).await;
    let op = w.start_turn("usage", "usage").await;
    let done = w.done(&op).await;
    assert_eq!(done.status_kind, "Completed");
    let seen = done.invocation.unwrap();
    assert_eq!(
        (seen.context_used, seen.context_window),
        (Some(29130), Some(665000)),
        "the last request, with cached input counted once and never the running total"
    );
    assert_eq!(seen.observed_model.as_deref(), Some("fixture-model"));
    let limits = &w.codex_listed().await["limits"];
    assert_eq!(limits["five_hour"]["utilization"], 0.3);
    assert_eq!(limits["five_hour"]["resets_at"], 1790212200);
    assert_eq!(limits["seven_day"]["utilization"], 0.6);
    w.close().await;
}

#[tokio::test]
async fn codex_usage_is_not_double_charged_on_resume() {
    let w = World::start(false).await;
    let first = w.start_turn("usage", "usage").await;
    assert_eq!(w.done(&first).await.status_kind, "Completed");
    w.core.harness().terminate(&w.thread).await.unwrap();
    let resumed = w.start_turn("resumed", "hello").await;
    let seen = w.done(&resumed).await.invocation.unwrap();
    assert_eq!(
        seen.context_used, None,
        "the replayed report belongs to an earlier turn"
    );
    let small = w.start_turn("small", "usage small").await;
    let seen = w.done(&small).await.invocation.unwrap();
    assert_eq!(
        (seen.context_used, seen.context_window),
        (Some(1000), Some(665000))
    );
    w.close().await;
}

#[tokio::test]
async fn codex_fork_keeps_the_source_unchanged() {
    let w = World::start(false).await;
    let first = w.start_turn("first", "hello").await;
    assert_eq!(w.done(&first).await.status_kind, "Completed");
    let source = w.core.threads().entries(&w.thread).await.unwrap();
    let at = source.last().unwrap().id.clone();
    let fork = w
        .core
        .harness()
        .fork_thread("fork".into(), &w.thread, &at)
        .await
        .unwrap();
    assert_eq!(fork.harness, "codex");
    let op = w
        .send(
            &fork.id,
            "on-fork",
            "settings",
            "fixture-model",
            Some("low"),
        )
        .await
        .unwrap();
    assert_eq!(w.settled(&fork.id, &op).await.status_kind, "Completed");
    assert_eq!(
        w.core.threads().entries(&w.thread).await.unwrap().len(),
        source.len()
    );
    let again = w.start_turn("source-again", "hello").await;
    assert_eq!(w.done(&again).await.status_kind, "Completed");
    w.close().await;
}

/// A Codex fork opens with Shadows' instructions as its developer
/// instructions, as a new thread does, so its first turn sends the person's
/// text first and does not repeat them after it.
#[tokio::test]
async fn codex_forks_first_turn_leads_with_the_persons_text() {
    let w = World::start(false).await;
    let first = w.start_turn("first", "hello").await;
    assert_eq!(w.done(&first).await.status_kind, "Completed");
    let at = w
        .core
        .threads()
        .entries(&w.thread)
        .await
        .unwrap()
        .last()
        .unwrap()
        .id
        .clone();
    let fork = w
        .core
        .harness()
        .fork_thread("fork-blocks".into(), &w.thread, &at)
        .await
        .unwrap();
    let op = w
        .send(
            &fork.id,
            "fork-first",
            "blocks",
            "fixture-model",
            Some("low"),
        )
        .await
        .unwrap();
    assert_eq!(w.settled(&fork.id, &op).await.status_kind, "Completed");
    let written = w.written(&fork.id, &op).await;
    let reply = written.last().expect("the fork's reply");
    let sent: Vec<String> = serde_json::from_str(reply).expect(reply);
    assert_eq!(sent[0], "blocks", "{sent:?}");
    for block in &sent[1..] {
        assert!(
            !block.contains("[Shadows] Shadows' instructions for you:")
                && !block.contains("[Shadows] The project's standards changed"),
            "the opening's instructions were sent again: {sent:?}"
        );
    }
    w.close().await;
}

#[tokio::test]
async fn codex_steer_targets_the_active_turn() {
    let w = World::start(false).await;
    let op = w.start_turn("steer", "steer").await;
    let queued = w
        .core
        .turns()
        .queue(
            w.thread.clone(),
            turn("extra", "add this", "fixture-model", Some("low")),
        )
        .await
        .unwrap();
    let Queued::Waiting { message } = queued else {
        panic!("a running turn queues the message");
    };
    let sent = w
        .core
        .turns()
        .send_now(&w.thread, &message.id, "now".into(), Vec::new())
        .await
        .unwrap();
    assert!(matches!(sent, SentNow::Steered { .. }), "{sent:?}");
    assert_eq!(w.done(&op).await.status_kind, "Completed");
    let written = w.written(&w.thread, &op).await;
    assert!(
        written
            .iter()
            .any(|body| body.contains("steered: add this")),
        "{written:?}"
    );
    assert_eq!(w.core.turns().operations(&w.thread).await.unwrap().len(), 1);
    w.close().await;
}

/// A Codex Planner reads a project file with a shell command, and leaves no file after a
/// request to write.
#[ignore = "live: needs codex (SHADOWS_CODEX_PATH) signed in to a dedicated home \
            (SHADOWS_CODEX_HOME)"]
#[tokio::test]
async fn live_a_codex_planner_reads_a_file_and_writes_none() {
    /// The cards of the tool calls `operation` made, in entry order.
    async fn calls(w: &World, operation: &OperationId) -> Vec<shadows_core::ToolCallCard> {
        w.core
            .threads()
            .entries(&w.thread)
            .await
            .unwrap()
            .into_iter()
            .filter(|entry| {
                entry.kind == shadows_core::ThreadEntryKind::ToolCall
                    && entry.operation_id.as_deref() == Some(operation.as_str())
            })
            .map(|entry| serde_json::from_value(entry.card.expect("a tool card")).unwrap())
            .collect()
    }
    let live = Duration::from_secs(180);

    let root = tempfile::tempdir().unwrap();
    let directory = root.path().join("project");
    std::fs::create_dir_all(directory.join("notes")).unwrap();
    let word = uuid::Uuid::new_v4().simple().to_string()[..8].to_owned();
    std::fs::write(directory.join("notes/word.txt"), &word).unwrap();
    let w = World::start_live(root).await;
    let choices = w.core.harness().open_session(&w.thread).await.unwrap();
    let current = choices.current;
    let operation = w
        .send(
            &w.thread,
            "read",
            "What word is written in notes/word.txt? \
             Read the file, then answer with the word only.",
            &current.model,
            current.effort.as_deref(),
        )
        .await
        .unwrap();
    let done = w.settled_within(&w.thread, &operation, live).await;
    assert_eq!(done.status_kind, "Completed", "{done:?}");
    let entries = w.core.threads().entries(&w.thread).await.unwrap();
    let reply = entries
        .iter()
        .rev()
        .find(|entry| {
            entry.kind == shadows_core::ThreadEntryKind::AgentMessage
                && entry.operation_id.as_deref() == Some(operation.as_str())
        })
        .expect("the read turn's reply");
    assert_eq!(
        reply.body.trim(),
        word,
        "the reply must come from reading the file"
    );
    // The word is in no prompt and no file name, and no Shadows tool reads a
    // file's text: only a shell command run by Codex can have read it.
    let reads = calls(&w, &operation)
        .await
        .into_iter()
        .filter(|card| {
            card.kind == shadows_core::CallKind::Execute
                && card.status == shadows_core::CallStatus::Completed
        })
        .count();
    assert!(reads > 0, "the read must be a completed shell command");

    let operation = w
        .send(
            &w.thread,
            "write",
            "Create the file made.txt in the project folder containing x.",
            &current.model,
            current.effort.as_deref(),
        )
        .await
        .unwrap();
    let done = w.settled_within(&w.thread, &operation, live).await;
    assert!(
        matches!(done.status_kind.as_str(), "Completed" | "Failed"),
        "{done:?}"
    );
    assert!(
        !directory.join("made.txt").exists(),
        "the Planner must never write"
    );
    assert_eq!(
        std::fs::read_to_string(directory.join("notes/word.txt")).unwrap(),
        word
    );
    let tried = calls(&w, &operation).await;
    let failed = tried
        .iter()
        .filter(|card| card.status == shadows_core::CallStatus::Failed)
        .count();
    println!("live codex read: {word} {reads} tool entries");
    println!(
        "live codex write: {} tool entries, {failed} failed, made.txt absent",
        tried.len()
    );
    w.close().await;
}

/// A Codex turn receives an attached image and names its colour.
#[ignore = "live: needs codex (SHADOWS_CODEX_PATH) signed in to a dedicated home \
            (SHADOWS_CODEX_HOME)"]
#[tokio::test]
async fn live_a_codex_turn_sees_an_image() {
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(root.path().join("project")).unwrap();
    let w = World::start_live(root).await;
    let choices = w.core.harness().open_session(&w.thread).await.unwrap();
    assert!(choices.capabilities.image);
    let current = choices.current;
    let red = include_bytes!("fixtures/red-64.png").to_vec();
    let image = w
        .core
        .threads()
        .upload_attachment("red".into(), &w.thread, "image/png", None, red)
        .await
        .unwrap();
    let mut send = turn(
        "see",
        "What colour is this image? Answer with one word.",
        &current.model,
        current.effort.as_deref(),
    );
    send.attachments = vec![image.id];
    let operation = w.core.turns().send(w.thread.clone(), send).await.unwrap();
    let done = w
        .settled_within(&w.thread, &operation, Duration::from_secs(180))
        .await;
    assert_eq!(done.status_kind, "Completed", "{done:?}");
    let reply = w
        .core
        .threads()
        .entries(&w.thread)
        .await
        .unwrap()
        .into_iter()
        .rev()
        .find(|entry| {
            entry.kind == shadows_core::ThreadEntryKind::AgentMessage
                && entry.operation_id.as_deref() == Some(operation.as_str())
        })
        .expect("the image turn's reply");
    assert!(
        reply.body.to_lowercase().contains("red"),
        "{:?}",
        reply.body
    );
    w.close().await;
}
