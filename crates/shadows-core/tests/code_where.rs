//! `code_where` through `AppCore`: the record's half, ranked, then the code's.

use std::sync::Arc;
use std::time::Duration;

use code_record::{CodeRecord, DoneRecord};
use core_plans::{TaskContract, WorkflowId};
use shadows_core::testing::{LiveHandles, Storage, acp};
use shadows_core::{
    AppCore, BuildState, CodeRecordError, CoreParts, ElementContent, ElementInput, ElementSeed,
    Grant, Live, ProjectId, Rank, SendTurn, TableContent, UsesLive, WhereAnswer, WhereEntry,
};

struct Fixture {
    core: Arc<AppCore>,
    storage: Arc<Storage>,
    project: ProjectId,
    grant: Grant,
    seed: ElementSeed,
    tmp: tempfile::TempDir,
}

impl Fixture {
    async fn new() -> Self {
        let tmp = tempfile::tempdir().unwrap();
        let storage = Arc::new(
            Storage::open(&tmp.path().join("where.sqlite3"))
                .await
                .unwrap(),
        );
        let runtime = Arc::new(AppCore::runtime_for_tests(storage.clone()).await.unwrap().0);
        let core = AppCore::assemble(CoreParts {
            operations: AppCore::operations_for_tests(storage.clone()),
            secrets: shadows_core::testing::secrets(storage.clone(), None),
            storage: storage.clone(),
            runtime,
            sessions: acp::fake_sessions(storage.clone()),
            handles: Arc::new(LiveHandles::default()),
            bus: tokio::sync::broadcast::channel(32).0,
            ui: tokio::sync::broadcast::channel(32).0,
            mcp_url: acp::MCP_URL.into(),
        });
        let project = core
            .project_lifecycle()
            .create(
                "project".into(),
                "where",
                "Where",
                tmp.path().to_str().unwrap(),
            )
            .await
            .unwrap()
            .id;
        let seed = core
            .design()
            .seed_elements_for_tests(&project)
            .await
            .unwrap();
        let thread = core
            .threads()
            .create("thread".into(), &project, "Planner", None, None)
            .await
            .unwrap();
        core.turns()
            .send(
                thread.id.clone(),
                SendTurn {
                    attachments: vec![],
                    command_id: "turn".into(),
                    prompt: "hang".into(),
                    model: "fake-large".into(),
                    mode: "acceptEdits".into(),
                    effort: Some("high".into()),
                    focus: None,
                    plan: None,
                    client_tab: None,
                },
            )
            .await
            .unwrap();
        let (grant, _) = shadows_core::Grants::issue_thread_grant_for_test(&storage, &thread.id)
            .await
            .unwrap();
        Self {
            core,
            storage,
            project,
            grant,
            seed,
            tmp,
        }
    }

    /// Writes `path` under the project folder.
    fn write(&self, path: &str, text: &str) {
        let full = self.tmp.path().join(path);
        std::fs::create_dir_all(full.parent().unwrap()).unwrap();
        std::fs::write(full, text).unwrap();
    }

    /// Records task `task` of plan version `version` as done with `contract`.
    async fn done(&self, version: i64, task: u32, contract: serde_json::Value) {
        let done = DoneRecord {
            project: self.project.clone(),
            workflow: WorkflowId::from_recorded("wf-1".into()),
            version,
            task,
            attempt: None,
            commit: Some(format!("c{version}{task}")),
            contract: serde_json::from_value::<TaskContract>(contract).unwrap(),
            verdict: None,
        };
        self.storage
            .write_txn(move |tx| {
                Box::pin(async move { CodeRecord::record_done_in(tx, &done).await })
            })
            .await
            .unwrap();
    }

    async fn table(&self, name: &str, purpose: &str) -> String {
        let input = ElementInput {
            id: None,
            expected_revision: None,
            part_id: self.seed.database.clone(),
            outcomes: vec![self.seed.outcome.clone()],
            content: ElementContent::Table(TableContent {
                name: name.into(),
                purpose: purpose.into(),
                columns: vec![shadows_core::TableColumn {
                    name: "id".into(),
                    type_name: "TEXT".into(),
                    meaning: String::new(),
                    primary_key: true,
                    references: None,
                }],
            }),
        };
        self.core
            .design()
            .put_element_for(&self.grant, format!("put-{name}"), input)
            .await
            .unwrap()
            .id
    }

    async fn ask(&self, topic: &str) -> WhereAnswer {
        self.core
            .code_record()
            .code_where(&self.project, topic)
            .await
            .unwrap()
    }

    async fn close(self) {
        self.core
            .shut_down(Duration::from_secs(3), std::future::pending())
            .await
            .unwrap();
        self.storage.close().await.unwrap();
    }
}

/// A contract creating or modifying `path` with functions `names`.
fn contract(path: &str, action: &str, purpose: &str, names: &[&str]) -> serde_json::Value {
    serde_json::json!({
        "files": [{"path": path, "action": action, "purpose": purpose}],
        "symbols": names.iter().enumerate().map(|(n, name)| serde_json::json!({
            "id": format!("s{}", n + 1), "path": path, "kind": "function", "name": name,
            "returns": {"name": "", "type_name": "None", "meaning": "m"},
            "behavior": [format!("{name} does its work")],
        })).collect::<Vec<_>>(),
    })
}

fn files(answer: &WhereAnswer) -> Vec<(Rank, &str)> {
    answer
        .entries
        .iter()
        .filter_map(|entry| match entry {
            WhereEntry::File(file) => Some((file.rank, file.path.as_str())),
            WhereEntry::Element(_) => None,
        })
        .collect()
}

fn file<'a>(answer: &'a WhereAnswer, path: &str) -> &'a shadows_core::FileEntry {
    answer
        .entries
        .iter()
        .find_map(|entry| match entry {
            WhereEntry::File(file) if file.path == path => Some(file),
            _ => None,
        })
        .unwrap_or_else(|| panic!("no entry for {path}: {answer:?}"))
}

#[tokio::test]
async fn a_symbol_name_finds_its_file_with_its_tasks_newest_first() {
    let f = Fixture::new().await;
    f.done(
        1,
        1,
        contract("api/users.py", "create", "the users", &["create_user"]),
    )
    .await;
    f.done(
        2,
        3,
        contract("api/users.py", "modify", "users change", &[]),
    )
    .await;
    let answer = f.ask("create_user").await;
    assert_eq!(files(&answer), vec![(Rank::Exact, "api/users.py")]);
    let entry = file(&answer, "api/users.py");
    let tasks: Vec<_> = entry.tasks.iter().map(|t| (t.version, t.task)).collect();
    assert_eq!(tasks, vec![(2, 3), (1, 1)]);
    assert_eq!(entry.tasks[0].commit.as_deref(), Some("c23"));
    assert_eq!(entry.symbols.len(), 1);
    let symbol = &entry.symbols[0];
    assert_eq!(symbol.name, "create_user");
    assert_eq!(symbol.behavior, vec!["create_user does its work"]);
    assert_eq!(symbol.live, Live::NotAsked);
    assert_eq!(symbol.uses_live, UsesLive::NotAsked);
    assert_eq!(answer.asked, 0);
    f.close().await;
}

#[tokio::test]
async fn a_path_prefix_finds_the_files_under_it_and_escapes_like_wildcards() {
    let f = Fixture::new().await;
    f.done(1, 1, contract("db_x/a.py", "create", "one", &[]))
        .await;
    f.done(1, 2, contract("dbax/b.py", "create", "two", &[]))
        .await;
    f.done(1, 3, contract("db_x.py", "create", "three", &[]))
        .await;
    let answer = f.ask("db_x").await;
    assert_eq!(files(&answer), vec![(Rank::Path, "db_x/a.py")]);
    let answer = f.ask("db_x/").await;
    assert_eq!(files(&answer), vec![(Rank::Path, "db_x/a.py")]);
    f.close().await;
}

#[tokio::test]
async fn a_part_title_finds_the_files_in_its_folder() {
    let f = Fixture::new().await;
    f.done(1, 1, contract("db/one.sql", "create", "one", &[]))
        .await;
    f.done(1, 2, contract("web/two.ts", "create", "two", &[]))
        .await;
    let answer = f.ask("seeded DATABASE").await;
    assert_eq!(files(&answer), vec![(Rank::Part, "db/one.sql")]);
    f.close().await;
}

#[tokio::test]
async fn an_element_id_or_name_finds_the_element_with_its_state() {
    let f = Fixture::new().await;
    let id = f.table("accounts", "Hold every account").await;
    let mut value = contract("db/accounts.sql", "create", "accounts table", &[]);
    value["elements"] = serde_json::json!([{
        "id": id, "revision": 1, "action": "create", "file": "db/accounts.sql"
    }]);
    f.done(1, 1, value).await;
    f.write("db/accounts.sql", "CREATE TABLE accounts (id TEXT);");
    for topic in [id.as_str(), "Accounts"] {
        let answer = f.ask(topic).await;
        let element = answer
            .entries
            .iter()
            .find_map(|entry| match entry {
                WhereEntry::Element(element) => Some(element),
                WhereEntry::File(_) => None,
            })
            .unwrap();
        assert_eq!(element.rank, Rank::Exact);
        assert_eq!(element.element.id, id);
        assert_eq!(element.state.state, BuildState::Built, "{topic}");
        assert_eq!(element.state.built_revision, Some(1));
    }
    f.close().await;
}

#[tokio::test]
async fn the_owner_is_the_folder_part_and_the_first_layer() {
    let f = Fixture::new().await;
    sqlx::query(
        "UPDATE design_part SET content_json = json_set(content_json, '$.layers', json(?))
         WHERE project_id = ? AND id = ?",
    )
    .bind(
        serde_json::json!([
            {"layer": "schema", "place": "db/schema", "not_used": null},
            {"layer": "migrations", "place": "db", "not_used": null},
        ])
        .to_string(),
    )
    .bind(f.project.as_str())
    .bind(f.seed.database.as_str())
    .execute(f.storage.reader())
    .await
    .unwrap();
    f.done(
        1,
        1,
        contract("db/schema/users.sql", "create", "users", &[]),
    )
    .await;
    let answer = f.ask("db/schema").await;
    let owner = file(&answer, "db/schema/users.sql").owner.clone().unwrap();
    assert_eq!(owner.part_id, f.seed.database);
    assert_eq!(owner.part_title, "Seeded database");
    assert_eq!(owner.layer.as_deref(), Some("schema"));
    f.close().await;
}

#[tokio::test]
async fn a_blank_or_long_topic_is_refused() {
    let f = Fixture::new().await;
    for topic in ["   ".to_string(), "x".repeat(201)] {
        let error = f
            .core
            .code_record()
            .code_where(&f.project, &topic)
            .await
            .unwrap_err();
        let CodeRecordError::InvalidTopic(message) = error else {
            panic!("{error:?}");
        };
        assert_eq!(message, "a topic needs 1 to 200 characters");
    }
    f.close().await;
}

#[tokio::test]
async fn nothing_matching_answers_no_entries() {
    let f = Fixture::new().await;
    f.done(
        1,
        1,
        contract("api/users.py", "create", "the users", &["create_user"]),
    )
    .await;
    let answer = f.ask("payments").await;
    assert!(answer.entries.is_empty(), "{answer:?}");
    assert_eq!(answer.entries_more, 0);
    assert!(answer.not_in_record.is_empty());
    f.close().await;
}
