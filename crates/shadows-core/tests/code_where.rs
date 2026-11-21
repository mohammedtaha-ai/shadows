//! The record through `AppCore`: `code_where` (the record's half, ranked, then
//! the code's) and the record's `digest`.

use std::sync::Arc;
use std::time::Duration;

use code_record::{CodeRecord, DoneRecord, RecordedAnswer};
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

    fn answer(&self, key: &str, answer: RecordedAnswer) {
        self.core.code_record().record_answer_for_tests(key, answer);
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
    assert_eq!(
        symbol.live,
        Live::Gap {
            why: "its file is absent now".into()
        }
    );
    assert_eq!(symbol.uses_live, UsesLive::NotAsked);
    f.close().await;
}

#[tokio::test]
async fn a_symbol_entry_carries_its_contract_inputs_and_returns() {
    let f = Fixture::new().await;
    let inputs = serde_json::json!([
        {"name": "email", "type_name": "str", "meaning": "the address to sign in with"},
        {"name": "password", "type_name": "str", "meaning": "the secret it is checked against"},
    ]);
    let returns = serde_json::json!({"name": "", "type_name": "Session", "meaning": "the session"});
    f.done(
        1,
        1,
        serde_json::json!({
            "files": [{"path": "api/auth.py", "action": "create", "purpose": "signing in"}],
            "symbols": [
                {"id": "s1", "path": "api/auth.py", "kind": "function", "name": "sign_in",
                 "inputs": inputs, "returns": returns, "behavior": ["signs a person in"]},
                {"id": "s2", "path": "api/auth.py", "kind": "class", "name": "Session",
                 "behavior": ["holds a signed-in person"]},
            ],
        }),
    )
    .await;
    let answer = f.ask("api/auth.py").await;
    let entry = file(&answer, "api/auth.py");
    let json = serde_json::to_value(&entry.symbols).unwrap();
    let sign_in = &json[1];
    assert_eq!(sign_in["name"], "sign_in");
    assert_eq!(sign_in["inputs"], inputs);
    assert_eq!(sign_in["returns"], returns);
    let class = &json[0];
    assert_eq!(class["name"], "Session");
    assert_eq!(class["inputs"], serde_json::json!([]));
    assert!(class.get("returns").is_none(), "{class}");
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

/// A rewritten `documentSymbol` answer: functions at their one-based lines.
fn functions(names: &[(&str, u32)]) -> RecordedAnswer {
    RecordedAnswer::Symbols(serde_json::Value::Array(
        names
            .iter()
            .map(|(name, line)| {
                serde_json::json!({"name": name, "kind": 12, "path": "", "line": line,
                                   "character": 5})
            })
            .collect(),
    ))
}

/// A rewritten `findReferences` result.
fn references(places: &[(&str, u32)], truncated: bool) -> RecordedAnswer {
    RecordedAnswer::References {
        result: serde_json::Value::Array(
            places
                .iter()
                .map(|(path, line)| serde_json::json!({"path": path, "line": line, "character": 1}))
                .collect(),
        ),
        truncated,
    }
}

fn budget_text() -> String {
    "the question's budget ran out (30 asks, 20 s)".into()
}

#[tokio::test]
async fn a_found_symbol_has_its_line_and_its_uses_grouped_by_part() {
    let f = Fixture::new().await;
    f.done(
        1,
        1,
        contract("db/users.py", "create", "users", &["load_user"]),
    )
    .await;
    f.write("db/users.py", "def load_user(): pass\n");
    f.answer("db/users.py", functions(&[("load_user", 3)]));
    f.answer(
        "db/users.py#references@3",
        references(
            &[("web/page.ts", 9), ("db/users.py", 3), ("db/other.py", 2)],
            false,
        ),
    );
    let answer = f.ask("load_user").await;
    let entry = file(&answer, "db/users.py");
    assert_eq!(entry.live, Live::Confirmed);
    assert_eq!(
        entry.confirmed,
        Some(shadows_core::Confirmation::FileAndSymbol)
    );
    let symbol = &entry.symbols[0];
    assert_eq!(
        (symbol.live.clone(), symbol.line),
        (Live::Confirmed, Some(3))
    );
    assert_eq!(symbol.uses_live, UsesLive::Answered);
    let uses: Vec<_> = symbol
        .uses
        .iter()
        .map(|u| {
            (
                u.path.as_str(),
                u.line,
                u.owner.as_ref().map(|o| o.part_title.as_str()),
            )
        })
        .collect();
    assert_eq!(
        uses,
        vec![
            ("db/other.py", 2, Some("Seeded database")),
            ("web/page.ts", 9, Some("Seeded frontend")),
        ]
    );
    assert_eq!(symbol.uses_total, Some(2));
    assert_eq!(
        answer.asked, 3,
        "the workspace ask, documentSymbol, findReferences"
    );
    f.close().await;
}

#[tokio::test]
async fn a_symbol_gone_from_its_file_is_a_live_gap() {
    let f = Fixture::new().await;
    f.done(
        1,
        1,
        contract("db/users.py", "create", "users", &["load_user"]),
    )
    .await;
    f.write("db/users.py", "pass\n");
    f.answer("db/users.py", functions(&[("other", 1)]));
    let answer = f.ask("load_user").await;
    let symbol = &file(&answer, "db/users.py").symbols[0];
    assert_eq!(
        symbol.live,
        Live::Gap {
            why: "load_user is not in db/users.py now".into()
        }
    );
    assert_eq!(symbol.uses_live, UsesLive::NotAsked);
    f.close().await;
}

#[tokio::test]
async fn an_absent_file_is_a_gap_and_is_not_asked() {
    let f = Fixture::new().await;
    f.done(
        1,
        1,
        contract("db/users.py", "create", "users", &["load_user"]),
    )
    .await;
    f.answer(
        "#workspace:load_user",
        RecordedAnswer::Workspace(serde_json::json!([])),
    );
    let answer = f.ask("load_user").await;
    let entry = file(&answer, "db/users.py");
    assert_eq!(
        entry.live,
        Live::Gap {
            why: "db/users.py is absent now".into()
        }
    );
    assert_eq!(answer.asked, 1, "only the workspace ask");
    f.close().await;
}

#[tokio::test]
async fn a_file_without_symbols_is_confirmed_file_only_without_an_ask() {
    let f = Fixture::new().await;
    f.done(1, 1, contract("db/one.sql", "create", "one", &[]))
        .await;
    f.write("db/one.sql", "--\n");
    let answer = f.ask("db/one.sql").await;
    let entry = file(&answer, "db/one.sql");
    assert_eq!(entry.live, Live::Confirmed);
    assert_eq!(entry.confirmed, Some(shadows_core::Confirmation::FileOnly));
    assert_eq!(answer.asked, 0);
    f.close().await;
}

#[tokio::test]
async fn no_server_makes_the_file_unchecked_and_the_record_still_answers() {
    let f = Fixture::new().await;
    f.done(
        1,
        1,
        contract("db/users.py", "create", "users", &["load_user"]),
    )
    .await;
    f.write("db/users.py", "def load_user(): pass\n");
    f.answer("db/users.py", RecordedAnswer::NoServer);
    let answer = f.ask("load_user").await;
    let entry = file(&answer, "db/users.py");
    let why = Live::Unchecked {
        why: "no language server reads .py".into(),
    };
    assert_eq!(entry.live, why);
    assert_eq!(entry.symbols[0].live, why);
    assert_eq!(entry.tasks.len(), 1);
    assert_eq!(entry.symbols[0].behavior, vec!["load_user does its work"]);
    f.close().await;
}

#[tokio::test]
async fn the_budget_stops_at_thirty_asks_and_says_so() {
    let f = Fixture::new().await;
    f.answer(
        "#workspace:budget",
        RecordedAnswer::Workspace(serde_json::json!([])),
    );
    for n in 0..25u32 {
        let path = format!("api/f{n:02}.py");
        let (a, b) = (format!("a{n}"), format!("b{n}"));
        f.done(
            1,
            n + 1,
            contract(&path, "create", "budget files", &[&a, &b]),
        )
        .await;
        f.write(&path, "pass\n");
        f.answer(&path, functions(&[(&a, 1), (&b, 2)]));
        f.answer(&format!("{path}#references@1"), references(&[], false));
        f.answer(&format!("{path}#references@2"), references(&[], false));
    }
    let answer = f.ask("budget").await;
    assert_eq!(answer.asked, 30);
    assert_eq!(answer.budget, Some(budget_text()));
    let last = file(&answer, "api/f19.py");
    assert_eq!(last.live, Live::Unchecked { why: budget_text() });
    assert_eq!(last.symbols[0].live, Live::Unchecked { why: budget_text() });
    f.close().await;
}

#[tokio::test]
async fn uses_beyond_ten_are_counted() {
    let f = Fixture::new().await;
    f.done(
        1,
        1,
        contract("db/users.py", "create", "users", &["load_user"]),
    )
    .await;
    f.write("db/users.py", "def load_user(): pass\n");
    f.answer("db/users.py", functions(&[("load_user", 3)]));
    let mut places: Vec<(String, u32)> = (1..=13).map(|n| ("web/a.ts".to_string(), n)).collect();
    places.push(("db/users.py".into(), 3));
    let places: Vec<(&str, u32)> = places.iter().map(|(p, l)| (p.as_str(), *l)).collect();
    f.answer("db/users.py#references@3", references(&places, false));
    let answer = f.ask("load_user").await;
    let symbol = &file(&answer, "db/users.py").symbols[0];
    assert_eq!(symbol.uses.len(), 10);
    assert_eq!(symbol.uses_more, 3);
    assert_eq!(symbol.uses_total, Some(13));
    assert_eq!(symbol.uses_live, UsesLive::Answered);
    f.close().await;
}

#[tokio::test]
async fn an_empty_reference_answer_is_answered_with_no_uses() {
    let f = Fixture::new().await;
    f.done(
        1,
        1,
        contract("db/users.py", "create", "users", &["load_user"]),
    )
    .await;
    f.write("db/users.py", "def load_user(): pass\n");
    f.answer("db/users.py", functions(&[("load_user", 3)]));
    f.answer("db/users.py#references@3", references(&[], false));
    let answer = f.ask("load_user").await;
    let symbol = &file(&answer, "db/users.py").symbols[0];
    assert_eq!(symbol.uses_live, UsesLive::Answered);
    assert!(symbol.uses.is_empty());
    assert_eq!((symbol.uses_more, symbol.uses_total), (0, Some(0)));
    f.close().await;
}

#[tokio::test]
async fn a_failed_reference_lookup_keeps_the_symbol_confirmed_and_says_why() {
    let f = Fixture::new().await;
    f.done(
        1,
        1,
        contract("db/users.py", "create", "users", &["load_user"]),
    )
    .await;
    f.write("db/users.py", "def load_user(): pass\n");
    f.answer("db/users.py", functions(&[("load_user", 3)]));
    f.answer(
        "db/users.py#references@3",
        RecordedAnswer::Failed("the server crashed".into()),
    );
    let answer = f.ask("load_user").await;
    let symbol = &file(&answer, "db/users.py").symbols[0];
    assert_eq!(symbol.live, Live::Confirmed);
    assert_eq!(
        symbol.uses_live,
        UsesLive::Failed {
            why: "the server crashed".into()
        }
    );
    assert!(symbol.uses.is_empty());
    assert_eq!(symbol.uses_total, None);
    f.close().await;
}

#[tokio::test]
async fn a_truncated_reference_answer_keeps_its_uses_and_has_no_total() {
    let f = Fixture::new().await;
    f.done(
        1,
        1,
        contract("db/users.py", "create", "users", &["load_user"]),
    )
    .await;
    f.write("db/users.py", "def load_user(): pass\n");
    f.answer("db/users.py", functions(&[("load_user", 3)]));
    let places: Vec<(String, u32)> = (1..=12).map(|n| ("web/a.ts".to_string(), n)).collect();
    let places: Vec<(&str, u32)> = places.iter().map(|(p, l)| (p.as_str(), *l)).collect();
    f.answer("db/users.py#references@3", references(&places, true));
    let answer = f.ask("load_user").await;
    let symbol = &file(&answer, "db/users.py").symbols[0];
    assert_eq!(symbol.uses.len(), 10);
    assert_eq!(symbol.uses_more, 2);
    assert!(matches!(symbol.uses_live, UsesLive::Truncated { .. }));
    assert_eq!(symbol.uses_total, None);
    f.close().await;
}

#[tokio::test]
async fn a_budget_spent_after_the_line_skips_the_lookup_and_says_so() {
    let f = Fixture::new().await;
    f.answer(
        "#workspace:spent",
        RecordedAnswer::Workspace(serde_json::json!([])),
    );
    // One workspace ask, then two asks per file: the 15th file's documentSymbol is the 30th.
    for n in 0..15u32 {
        let path = format!("api/f{n:02}.py");
        let name = format!("a{n}");
        f.done(1, n + 1, contract(&path, "create", "spent files", &[&name]))
            .await;
        f.write(&path, "pass\n");
        f.answer(&path, functions(&[(&name, 4)]));
        f.answer(&format!("{path}#references@4"), references(&[], false));
    }
    let answer = f.ask("spent").await;
    assert_eq!(answer.asked, 30);
    let symbol = &file(&answer, "api/f14.py").symbols[0];
    assert_eq!(
        (symbol.live.clone(), symbol.line),
        (Live::Confirmed, Some(4))
    );
    assert_eq!(symbol.uses_live, UsesLive::Skipped { why: budget_text() });
    assert!(symbol.uses.is_empty());
    assert_eq!(answer.budget, Some(budget_text()));
    f.close().await;
}

#[tokio::test]
async fn a_word_topic_lists_files_not_in_the_record_from_workspace_symbols() {
    let f = Fixture::new().await;
    f.done(1, 1, contract("db/users.py", "create", "users", &["login"]))
        .await;
    f.write("db/users.py", "def login(): pass\n");
    f.answer("db/users.py", functions(&[("login", 1)]));
    f.answer("db/users.py#references@1", references(&[], false));
    f.answer(
        "#workspace:login",
        RecordedAnswer::Workspace(serde_json::json!([
            {"name": "login", "kind": 12, "path": "db/users.py", "line": 1, "character": 5},
            {"name": "login_form", "kind": 12, "path": "web/login.ts", "line": 7, "character": 1},
            {"name": "LoginPage", "kind": 5, "path": "web/login.ts", "line": 2, "character": 1},
        ])),
    );
    let answer = f.ask("login").await;
    assert_eq!(answer.not_in_record.len(), 1);
    let other = &answer.not_in_record[0];
    assert_eq!(other.path, "web/login.ts");
    let symbols: Vec<_> = other
        .symbols
        .iter()
        .map(|s| (s.name.as_str(), s.line))
        .collect();
    assert_eq!(symbols, vec![("LoginPage", 2), ("login_form", 7)]);
    assert_eq!(other.owner.as_ref().unwrap().part_title, "Seeded frontend");
    f.close().await;
}

#[tokio::test]
async fn a_path_topic_lists_files_not_in_the_record_from_the_file_list() {
    let f = Fixture::new().await;
    f.done(1, 1, contract("api/auth/one.py", "create", "one", &[]))
        .await;
    f.write("api/auth/one.py", "pass\n");
    f.write("api/auth/two.py", "pass\n");
    let listing = shadows_core::CodebaseConfig {
        name_index: false,
        ..shadows_core::CodebaseConfig::default()
    };
    f.core.codebase().start(listing).await.unwrap();
    f.core.codebase().touch(&f.project).await;
    let mut answer = f.ask("api/auth").await;
    for _ in 0..100 {
        if !answer.not_in_record.is_empty() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
        answer = f.ask("api/auth").await;
    }
    let paths: Vec<_> = answer
        .not_in_record
        .iter()
        .map(|u| u.path.as_str())
        .collect();
    assert_eq!(paths, vec!["api/auth/two.py"]);
    assert_eq!(files(&answer), vec![(Rank::Path, "api/auth/one.py")]);
    f.core.codebase().shut_down_for_test().await;
    f.close().await;
}

/// Asks `topic` until `done` holds of the answer, for up to two minutes while
/// pyright starts and reads the folder.
async fn ask_until(f: &Fixture, topic: &str, done: impl Fn(&WhereAnswer) -> bool) -> WhereAnswer {
    let listing = shadows_core::CodebaseConfig {
        name_index: false,
        ..shadows_core::CodebaseConfig::default()
    };
    f.core.codebase().start(listing).await.unwrap();
    f.core.codebase().touch(&f.project).await;
    let mut answer = f.ask(topic).await;
    for _ in 0..120 {
        if done(&answer) {
            break;
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
        answer = f.ask(topic).await;
    }
    answer
}

#[tokio::test]
#[ignore = "needs pyright-langserver on PATH"]
async fn live_code_where_gives_a_functions_line_and_its_caller() {
    let f = Fixture::new().await;
    f.write("api/__init__.py", "");
    f.write("web/__init__.py", "");
    f.write(
        "api/auth.py",
        "import os\n\n\ndef login(name):\n    return name\n",
    );
    f.write(
        "web/views.py",
        "from api.auth import login\n\n\ndef page():\n    return login('a')\n",
    );
    f.done(
        1,
        1,
        contract("api/auth.py", "create", "sign in", &["login"]),
    )
    .await;
    let answer = ask_until(&f, "login", |a| {
        a.entries.iter().any(|e| match e {
            WhereEntry::File(file) => file.symbols.iter().any(|s| !s.uses.is_empty()),
            WhereEntry::Element(_) => false,
        })
    })
    .await;
    let symbol = &file(&answer, "api/auth.py").symbols[0];
    assert_eq!(
        (symbol.live.clone(), symbol.line),
        (Live::Confirmed, Some(4)),
        "{answer:?}"
    );
    assert_eq!(symbol.uses_live, UsesLive::Answered, "{answer:?}");
    assert!(
        symbol.uses.iter().any(|u| u.path == "web/views.py"
            && u.owner
                .as_ref()
                .is_some_and(|o| o.part_title == "Seeded frontend")),
        "{answer:?}"
    );
    f.core.codebase().shut_down_for_test().await;
    f.close().await;
}

#[tokio::test]
#[ignore = "needs pyright-langserver on PATH"]
async fn live_a_file_outside_the_record_is_listed_as_not_in_the_record() {
    let f = Fixture::new().await;
    f.write("api/auth.py", "def login(name):\n    return name\n");
    f.write("web/extra.py", "def login_helper():\n    return 1\n");
    f.done(
        1,
        1,
        contract("api/auth.py", "create", "sign in", &["login"]),
    )
    .await;
    let answer = ask_until(&f, "login", |a| !a.not_in_record.is_empty()).await;
    let paths: Vec<_> = answer
        .not_in_record
        .iter()
        .map(|u| u.path.as_str())
        .collect();
    assert_eq!(paths, vec!["web/extra.py"], "{answer:?}");
    assert_eq!(answer.not_in_record[0].symbols[0].name, "login_helper");
    f.core.codebase().shut_down_for_test().await;
    f.close().await;
}

#[tokio::test]
async fn the_digest_lists_files_by_part_and_layer_with_signatures_then_unowned_and_gaps() {
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
    let empty = f.core.code_record().digest(&f.project).await.unwrap();
    assert!(empty.is_empty());
    assert_eq!(empty.marks, 0);
    f.done(
        1,
        1,
        contract("web/app.ts", "create", "the screen", &["render"]),
    )
    .await;
    f.done(
        1,
        2,
        contract("db/0001.sql", "create", "first migration", &[]),
    )
    .await;
    f.done(
        1,
        3,
        serde_json::json!({
            "files": [{"path": "db/schema/users.sql", "action": "create", "purpose": "users"}],
            "symbols": [{"id": "s1", "path": "db/schema/users.sql", "kind": "function",
                "name": "make_user",
                "inputs": [{"name": "email", "type_name": "str", "meaning": "m"}],
                "returns": {"name": "", "type_name": "User", "meaning": "m"},
                "behavior": ["b"]}],
        }),
    )
    .await;
    f.done(
        2,
        1,
        contract(
            "db/schema/users.sql",
            "modify",
            "users, newer",
            &["add_index"],
        ),
    )
    .await;
    f.done(2, 2, contract("tools/run.py", "create", "runs it", &[]))
        .await;
    let id = f.table("accounts", "Hold every account").await;
    let mut value = contract("db/accounts.sql", "create", "accounts table", &[]);
    value["elements"] = serde_json::json!([{
        "id": id, "revision": 1, "action": "create", "file": "db/accounts.sql"
    }]);
    f.done(2, 3, value).await;
    let digest = f.core.code_record().digest(&f.project).await.unwrap();
    assert_eq!(digest.marks, 6);
    let parts: Vec<_> = digest.parts.iter().map(|p| p.title.as_str()).collect();
    assert_eq!(parts, ["Seeded database", "Seeded frontend"]);
    let database = &digest.parts[0];
    let files: Vec<_> = database
        .files
        .iter()
        .map(|file| {
            (
                file.layer.as_deref(),
                file.path.as_str(),
                file.purpose.as_str(),
            )
        })
        .collect();
    assert_eq!(
        files,
        [
            (Some("schema"), "db/schema/users.sql", "users, newer"),
            (Some("migrations"), "db/0001.sql", "first migration"),
            (Some("migrations"), "db/accounts.sql", "accounts table"),
        ]
    );
    let symbols: Vec<_> = database.files[0]
        .symbols
        .iter()
        .map(|s| s.name.as_str())
        .collect();
    assert_eq!(symbols, ["add_index", "make_user"]);
    let make_user = &database.files[0].symbols[1];
    assert_eq!(make_user.inputs[0].type_name, "str");
    assert_eq!(make_user.returns.as_ref().unwrap().type_name, "User");
    assert_eq!(digest.parts[1].files[0].path, "web/app.ts");
    assert_eq!(digest.parts[1].files[0].layer, None);
    let unowned: Vec<_> = digest.unowned.iter().map(|f| f.path.as_str()).collect();
    assert_eq!(unowned, ["tools/run.py"]);
    assert_eq!(digest.gaps.len(), 1);
    assert_eq!(digest.gaps[0].element_id, id);
    assert_eq!(digest.gaps[0].state, BuildState::Gap);
    assert!(!digest.gaps[0].why.is_empty());
    f.close().await;
}
