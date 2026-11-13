//! The Planner's changes to the project's files: a note records why, and a
//! change it did not explain is recorded and asked about when its turn settles.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use shadows_agent::policy;
use shadows_core::testing::{
    CommandContext, LiveHandles, ProjectDirectory, Storage, acp, create_project,
};
use shadows_core::{
    AppCore, CodebaseConfig, CoreParts, IndexState, OperationId, ProjectId, SendTurn, ThreadEntry,
    ThreadEntryKind, ThreadId, Threads,
};

struct World {
    core: Arc<AppCore>,
    storage: Arc<Storage>,
    project: ProjectId,
    dir: PathBuf,
    thread: ThreadId,
    _tmp: tempfile::TempDir,
}

fn ctx(id: &str, kind: &str) -> CommandContext {
    CommandContext {
        agent: None,
        principal_kind: "User".into(),
        principal_id: "local".into(),
        command_id: id.into(),
        command_kind: kind.into(),
        command_schema_ver: 1,
        request_fingerprint: CommandContext::fingerprint(kind, &serde_json::json!({ "id": id })),
    }
}

/// The database lives beside the project's folder, never in it: its writes
/// would be project changes.
async fn world() -> World {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("project");
    std::fs::create_dir(&dir).unwrap();
    let storage = Arc::new(Storage::open(&tmp.path().join("s.sqlite3")).await.unwrap());
    let (runtime, _report) = AppCore::runtime_for_tests(storage.clone()).await.unwrap();
    let project = create_project(
        &storage,
        &ctx("c1", "project.create"),
        "demo",
        "Demo",
        &ProjectDirectory::resolve(&dir).unwrap(),
        &policy::default_modes(),
    )
    .await
    .unwrap();
    let thread = Threads::create_planning_thread_for_test(
        &storage,
        &ctx("c2", "thread.create"),
        &project.id,
        "T",
        policy::CLAUDE_CODE,
        None,
    )
    .await
    .unwrap()
    .id;
    let secrets = shadows_core::testing::secrets(storage.clone(), None);
    let config = shadows_core::testing::SessionsConfig {
        secrets: Some(secrets.clone()),
        ..acp::test_config()
    };
    let sessions = AppCore::harness_for_tests(acp::fake_adapter(), storage.clone(), config);
    let core = AppCore::assemble(CoreParts {
        operations: AppCore::operations_for_tests(storage.clone()),
        secrets,
        storage: storage.clone(),
        runtime: Arc::new(runtime),
        sessions,
        handles: Arc::new(LiveHandles::default()),
        bus: tokio::sync::broadcast::channel(256).0,
        ui: tokio::sync::broadcast::channel(64).0,
        mcp_url: acp::MCP_URL.into(),
    });
    core.codebase()
        .start(CodebaseConfig {
            rescan_every: Duration::from_secs(60),
            debounce: Duration::from_millis(100),
            name_index: false,
        })
        .await
        .unwrap();
    World {
        core,
        storage,
        project: project.id,
        dir,
        thread,
        _tmp: tmp,
    }
}

impl World {
    /// A turn that runs until it is stopped, once the project's watcher watches.
    async fn running(&self, command: &str) -> OperationId {
        let turn = SendTurn {
            command_id: command.into(),
            prompt: "hang".into(),
            attachments: Vec::new(),
            model: "fake-large".into(),
            mode: "acceptEdits".into(),
            effort: Some("high".into()),
            focus: None,
            plan: None,
            client_tab: None,
        };
        let op = self
            .core
            .turns()
            .send(self.thread.clone(), turn)
            .await
            .unwrap();
        for _ in 0..200 {
            let status = self.core.codebase().status(&self.project).await.unwrap();
            if status.state == IndexState::Ready {
                return op;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        panic!("the project's watcher did not start");
    }

    /// Stops the turn once the watcher had time to see what was written.
    async fn settle(&self, op: &OperationId) {
        tokio::time::sleep(Duration::from_millis(700)).await;
        self.core.turns().stop(op).await.unwrap();
        for _ in 0..200 {
            let loaded = shadows_core::Turns::get_operation_for_test(&self.storage, op)
                .await
                .unwrap();
            if loaded.finished_at.is_some() {
                return;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        panic!("operation did not settle");
    }

    async fn changes(&self) -> Vec<ThreadEntry> {
        let entries = self.core.threads().entries(&self.thread).await.unwrap();
        entries
            .into_iter()
            .filter(|e| e.kind == ThreadEntryKind::ProjectChange)
            .collect()
    }

    async fn close(self) {
        self.core
            .shut_down(Duration::from_secs(5), std::future::pending())
            .await
            .unwrap();
    }
}

#[tokio::test]
async fn a_noted_change_is_recorded_with_its_reason() {
    let w = world().await;
    let op = w.running("t1").await;
    std::fs::write(w.dir.join("notes.txt"), "installed gopls\n").unwrap();
    w.core
        .turns()
        .note_change(
            &w.thread,
            vec!["./notes.txt".into()],
            "a record of the install".into(),
        )
        .await
        .unwrap();
    let outside = w
        .core
        .turns()
        .note_change(&w.thread, vec!["../s.sqlite3".into()], "no".into())
        .await;
    assert!(outside.is_err(), "a path outside the project is refused");
    w.settle(&op).await;
    let changes = w.changes().await;
    let [entry] = changes.as_slice() else {
        panic!("one noted change: {changes:?}");
    };
    assert_eq!(entry.body, "a record of the install");
    assert_eq!(entry.author.kind, "Agent");
    assert_eq!(entry.operation_id.as_deref(), Some(op.as_str()));
    assert_eq!(
        entry.card.clone().unwrap(),
        serde_json::json!({
            "paths": ["notes.txt"], "reason": "a record of the install", "noted": true
        })
    );
    assert!(w.core.turns().queued(&w.thread).await.unwrap().is_empty());
    w.close().await;
}

#[tokio::test]
async fn an_unnoted_shell_change_is_recorded_and_asked_about() {
    let w = world().await;
    let op = w.running("t1").await;
    std::fs::write(w.dir.join("setup.sh"), "echo hi\n").unwrap();
    std::fs::create_dir(w.dir.join(".git")).unwrap();
    std::fs::write(w.dir.join(".git").join("HEAD"), "ref: refs/heads/main\n").unwrap();
    w.settle(&op).await;
    let changes = w.changes().await;
    let [entry] = changes.as_slice() else {
        panic!("one unnoted change: {changes:?}");
    };
    assert_eq!(entry.author.kind, "Shadows");
    assert_eq!(entry.operation_id.as_deref(), Some(op.as_str()));
    let card = entry.card.clone().unwrap();
    assert_eq!(card["paths"], serde_json::json!(["setup.sh"]), "{card}");
    assert!(card["reason"].is_null());
    assert_eq!(card["noted"], false);
    let queued = w.core.turns().queued(&w.thread).await.unwrap();
    let [notice] = queued.as_slice() else {
        panic!("one notice: {queued:?}");
    };
    let sent = notice.notice.clone().expect("a Shadows notice");
    assert_eq!(sent.executor, None);
    assert_eq!(
        sent.card,
        serde_json::json!({ "kind": "project_change", "paths": ["setup.sh"] })
    );
    assert_eq!(
        notice.prompt,
        "You changed these project files in your last turn without project_change_note: \
         setup.sh. Say why with project_change_note."
    );
    assert_eq!(notice.model, "fake-large");
    w.close().await;
}

#[tokio::test]
async fn a_turn_with_no_change_sends_nothing() {
    let w = world().await;
    let op = w.running("t1").await;
    w.settle(&op).await;
    assert!(w.changes().await.is_empty());
    assert!(w.core.turns().queued(&w.thread).await.unwrap().is_empty());
    w.close().await;
}
