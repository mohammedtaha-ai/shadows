//! `person_ask`: the Planner's question is a stored card, and the person's
//! click is their message, sent as a typed one is.

use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use shadows_agent::policy;
use shadows_core::testing::{
    CommandContext, LiveHandles, ProjectDirectory, Storage, acp, create_project,
};
use shadows_core::{
    AppCore, CoreParts, GrantKind, Grants, Operation, OperationId, QuestionId, QuestionOption,
    Queued, SendTurn, StorageError, ThreadEntry, ThreadEntryKind, ThreadId, Threads, TurnsError,
};

struct World {
    core: Arc<AppCore>,
    storage: Arc<Storage>,
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

async fn world() -> World {
    let tmp = tempfile::tempdir().unwrap();
    let storage = Arc::new(Storage::open(&tmp.path().join("s.sqlite3")).await.unwrap());
    let (runtime, _report) = AppCore::runtime_for_tests(storage.clone()).await.unwrap();
    let project = create_project(
        &storage,
        &ctx("c1", "project.create"),
        "demo",
        "Demo",
        &ProjectDirectory::resolve(tmp.path()).unwrap(),
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
    World {
        core,
        storage,
        thread,
        _tmp: tmp,
    }
}

fn options(labels: &[&str]) -> Vec<QuestionOption> {
    labels
        .iter()
        .map(|label| QuestionOption {
            label: (*label).into(),
        })
        .collect()
}

impl World {
    async fn send(&self, thread: &ThreadId, command: &str, prompt: &str) -> OperationId {
        let turn = SendTurn {
            command_id: command.into(),
            prompt: prompt.into(),
            attachments: Vec::new(),
            model: "fake-large".into(),
            mode: "acceptEdits".into(),
            effort: Some("high".into()),
            focus: None,
            plan: None,
            client_tab: None,
        };
        self.core.turns().send(thread.clone(), turn).await.unwrap()
    }

    async fn settled(&self, op: &OperationId) -> Operation {
        for _ in 0..200 {
            let loaded = shadows_core::Turns::get_operation_for_test(&self.storage, op)
                .await
                .unwrap();
            if loaded.finished_at.is_some() {
                return loaded;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        panic!("operation did not settle");
    }

    async fn ask(&self, thread: &ThreadId) -> QuestionId {
        self.core
            .turns()
            .ask(
                thread,
                "Which database?".into(),
                options(&["SQLite", "Postgres"]),
            )
            .await
            .unwrap()
    }

    async fn entries(&self, thread: &ThreadId) -> Vec<ThreadEntry> {
        self.core.threads().entries(thread).await.unwrap()
    }

    async fn close(self) {
        self.core
            .shut_down(Duration::from_secs(5), std::future::pending())
            .await
            .unwrap();
    }
}

fn question_entry(entries: &[ThreadEntry]) -> &ThreadEntry {
    entries
        .iter()
        .find(|e| e.kind == ThreadEntryKind::PersonQuestion)
        .expect("a question entry")
}

async fn reread(dir: &Path, thread: &ThreadId) -> Vec<ThreadEntry> {
    let storage = Arc::new(Storage::open(&dir.join("s.sqlite3")).await.unwrap());
    Threads::list_thread_entries_for_test(&storage, thread)
        .await
        .unwrap()
}

#[tokio::test]
async fn a_question_is_a_stored_entry() {
    let w = world().await;
    let id = w.ask(&w.thread).await;
    let entries = reread(w._tmp.path(), &w.thread).await;
    let entry = question_entry(&entries);
    assert_eq!(entry.body, "Which database?");
    let card = entry.card.as_ref().unwrap();
    assert_eq!(card["question_id"], id.as_str());
    assert_eq!(card["question"], "Which database?");
    assert_eq!(
        card["options"],
        serde_json::json!([{ "label": "SQLite" }, { "label": "Postgres" }])
    );
    assert!(card["chosen"].is_null());
    let refused = w
        .core
        .turns()
        .ask(&w.thread, "Which?".into(), options(&["only one"]))
        .await;
    assert!(matches!(refused, Err(TurnsError::InvalidCommand(_))));
    let long = "x".repeat(81);
    let refused = w
        .core
        .turns()
        .ask(&w.thread, "Which?".into(), options(&["a", &long]))
        .await;
    assert!(matches!(refused, Err(TurnsError::InvalidCommand(_))));
    w.close().await;
}

#[tokio::test]
async fn an_answer_reaches_the_planner_as_the_persons_message() {
    let w = world().await;
    let first = w.send(&w.thread, "t1", "hello").await;
    w.settled(&first).await;
    let id = w.ask(&w.thread).await;
    let answered = w.core.turns().answer("a1".into(), &w.thread, &id, 1).await;
    let Ok(Queued::Started { operation_id }) = answered else {
        panic!("an idle thread starts the answer: {answered:?}");
    };
    assert_eq!(w.settled(&operation_id).await.status_kind, "Completed");
    let entries = w.entries(&w.thread).await;
    let message = entries
        .iter()
        .find(|e| e.operation_id.as_deref() == Some(operation_id.as_str()))
        .unwrap();
    assert_eq!(message.kind, ThreadEntryKind::UserMessage);
    assert_eq!(message.author.kind, "User");
    assert_eq!(message.body, "Answer to \"Which database?\": Postgres");
    assert!(entries.iter().any(|e| {
        e.kind == ThreadEntryKind::AgentMessage
            && e.operation_id.as_deref() == Some(operation_id.as_str())
    }));

    // While a turn runs, the answer waits as a typed message does.
    let id = w.ask(&w.thread).await;
    let running = w.send(&w.thread, "t2", "hang").await;
    let answered = w.core.turns().answer("a2".into(), &w.thread, &id, 0).await;
    let Ok(Queued::Waiting { message }) = answered else {
        panic!("a busy thread keeps the answer waiting: {answered:?}");
    };
    assert_eq!(message.prompt, "Answer to \"Which database?\": SQLite");
    assert_eq!(message.model, "fake-large");
    w.core.turns().stop(&running).await.unwrap();
    w.close().await;
}

#[tokio::test]
async fn a_question_is_answered_once() {
    let w = world().await;
    let first = w.send(&w.thread, "t1", "hello").await;
    w.settled(&first).await;
    let id = w.ask(&w.thread).await;
    let running = w.send(&w.thread, "t2", "hang").await;
    let one = w.core.turns().answer("a1".into(), &w.thread, &id, 0).await;
    let Ok(Queued::Waiting { message }) = one else {
        panic!("{one:?}");
    };
    let replay = w.core.turns().answer("a1".into(), &w.thread, &id, 0).await;
    let Ok(Queued::Waiting { message: again }) = replay else {
        panic!("{replay:?}");
    };
    assert_eq!(again.id, message.id, "a replay answers the first result");
    let second = w.core.turns().answer("a2".into(), &w.thread, &id, 1).await;
    assert!(
        matches!(
            second,
            Err(TurnsError::Storage(StorageError::CommandConflict))
        ),
        "{second:?}"
    );
    assert_eq!(w.core.turns().queued(&w.thread).await.unwrap().len(), 1);
    w.core.turns().stop(&running).await.unwrap();
    w.close().await;
}

#[tokio::test]
async fn an_answered_card_reads_chosen_after_a_reload() {
    let w = world().await;
    let first = w.send(&w.thread, "t1", "hello").await;
    w.settled(&first).await;
    let id = w.ask(&w.thread).await;
    let running = w.send(&w.thread, "t2", "hang").await;
    w.core
        .turns()
        .answer("a1".into(), &w.thread, &id, 1)
        .await
        .unwrap();
    w.core.turns().stop(&running).await.unwrap();
    let entries = reread(w._tmp.path(), &w.thread).await;
    let card = question_entry(&entries).card.clone().unwrap();
    assert_eq!(card["chosen"], 1);
    assert_eq!(card["question_id"], id.as_str());
    w.close().await;
}

#[tokio::test]
async fn a_forked_question_cannot_be_answered_in_the_fork() {
    let w = world().await;
    let first = w.send(&w.thread, "t1", "hello").await;
    w.settled(&first).await;
    let id = w.ask(&w.thread).await;
    let second = w.send(&w.thread, "t2", "hello").await;
    w.settled(&second).await;
    let at = w.entries(&w.thread).await.last().unwrap().id.clone();
    let fork = w
        .core
        .harness()
        .fork_thread("f1".into(), &w.thread, &at)
        .await
        .unwrap();
    let copied = w.entries(&fork.id).await;
    let card = question_entry(&copied).card.clone().unwrap();
    assert_eq!(
        card["question_id"],
        id.as_str(),
        "the fork keeps the card as history"
    );
    let refused = w.core.turns().answer("a1".into(), &fork.id, &id, 0).await;
    assert!(
        matches!(refused, Err(TurnsError::InvalidCommand(_))),
        "{refused:?}"
    );
    w.close().await;
}

#[tokio::test]
async fn an_executor_cannot_ask() {
    assert!(Grants::tools_for(GrantKind::Thread).contains(&"person_ask"));
    assert!(!Grants::tools_for(GrantKind::Executor).contains(&"person_ask"));
    assert!(!Grants::tools_for(GrantKind::Project).contains(&"person_ask"));
    assert!(shadows_core::testing::PROMPT.contains(
        "When the person must choose, ask with `person_ask` and wait for the answer; do not ask \
         the same thing in text."
    ));
}
