//! The comparison against a real `pyright-langserver`, on a temporary Python project.

use std::path::Path;
use std::sync::Arc;

use code_record::{CodeRecord, SymbolState, Unpromised, VerdictOutcome};
use codebase::{Codebase, CodebaseConfig};
use core_db::Storage;
use core_plans::{ContractValue, FileAction, FileWork, SymbolKind, SymbolWork, TaskContract};
use core_projects::{ProjectId, Projects};

struct Fixture {
    _db: tempfile::TempDir,
    dir: tempfile::TempDir,
    storage: Arc<Storage>,
    code: Codebase,
    record: CodeRecord,
    project: ProjectId,
}

impl Fixture {
    async fn new(files: &[(&str, &str)]) -> Fixture {
        let db = tempfile::tempdir().unwrap();
        let dir = tempfile::tempdir().unwrap();
        for (path, text) in files {
            let full = dir.path().join(path);
            std::fs::create_dir_all(full.parent().unwrap()).unwrap();
            std::fs::write(full, text).unwrap();
        }
        let storage = Arc::new(
            Storage::open(&db.path().join("record.sqlite3"))
                .await
                .unwrap(),
        );
        let projects = Projects::new(storage.clone());
        let project = projects
            .create("p".into(), "p", "P", dir.path().to_str().unwrap())
            .await
            .unwrap();
        let code = Codebase::new(storage.clone(), projects);
        code.start(CodebaseConfig {
            name_index: false,
            ..CodebaseConfig::default()
        })
        .await
        .unwrap();
        let record = CodeRecord::new(storage.clone(), code.clone());
        Fixture {
            _db: db,
            dir,
            storage,
            code,
            record,
            project: project.id,
        }
    }

    fn root(&self) -> &Path {
        self.dir.path()
    }

    async fn close(self) {
        self.code.shut_down_for_test().await;
        self.storage.close().await.unwrap();
    }
}

fn contract(files: &[(&str, FileAction)], symbols: &[(&str, &str, &str)]) -> TaskContract {
    TaskContract {
        files: files
            .iter()
            .map(|(path, action)| FileWork {
                path: path.to_string(),
                action: *action,
                purpose: "p".into(),
            })
            .collect(),
        symbols: symbols
            .iter()
            .map(|(id, path, name)| SymbolWork {
                id: id.to_string(),
                path: path.to_string(),
                kind: SymbolKind::Function,
                name: name.to_string(),
                inputs: Vec::new(),
                returns: Some(ContractValue {
                    name: String::new(),
                    type_name: "None".into(),
                    meaning: "m".into(),
                }),
                behavior: vec!["b".into()],
                errors: Vec::new(),
            })
            .collect(),
        produces: Vec::new(),
        uses: Vec::new(),
        steps: Vec::new(),
        invariants: Vec::new(),
        forbidden: Vec::new(),
        checks: Vec::new(),
        decisions: Vec::new(),
    }
}

const SERVICE: &str = "class Auth:\n    def refresh(self) -> None:\n        pass\n\n\ndef login() -> None:\n    pass\n";

#[tokio::test]
#[ignore = "needs pyright-langserver on PATH"]
async fn live_a_promised_function_is_found_and_a_forgotten_one_missing() {
    let f = Fixture::new(&[("auth/service.py", SERVICE)]).await;
    let c = contract(
        &[("auth/service.py", FileAction::Modify)],
        &[
            ("s1", "auth/service.py", "login"),
            ("s2", "auth/service.py", "Auth.refresh"),
            ("s3", "auth/service.py", "refresh_token"),
        ],
    );
    let verdict = f.record.verify("a1", &f.project, 3, &c).await.unwrap();
    let states: Vec<SymbolState> = verdict.symbols.iter().map(|s| s.state).collect();
    assert_eq!(
        states,
        vec![SymbolState::Found, SymbolState::Found, SymbolState::Missing],
        "{verdict:?}"
    );
    assert_eq!(verdict.outcome(), VerdictOutcome::Failed);
    assert_eq!(
        verdict.lines(),
        "T3 promises refresh_token in auth/service.py; the code does not have it."
    );
    assert_eq!(f.record.verdict("a1").await.unwrap(), Some(verdict));
    f.close().await;
}

#[tokio::test]
#[ignore = "needs pyright-langserver on PATH"]
async fn live_an_absent_promised_file_makes_its_symbols_missing() {
    let f = Fixture::new(&[("auth/service.py", SERVICE)]).await;
    let c = contract(
        &[
            ("auth/service.py", FileAction::Modify),
            ("auth/tokens.py", FileAction::Create),
        ],
        &[
            ("s1", "auth/service.py", "login"),
            ("s2", "auth/tokens.py", "issue"),
            ("s3", "auth/tokens.py", "revoke"),
        ],
    );
    let verdict = f.record.verify("a2", &f.project, 2, &c).await.unwrap();
    let states: Vec<SymbolState> = verdict.symbols.iter().map(|s| s.state).collect();
    assert_eq!(
        states,
        vec![
            SymbolState::Found,
            SymbolState::Missing,
            SymbolState::Missing
        ],
        "{verdict:?}"
    );
    assert_eq!(
        verdict.lines(),
        "auth/tokens.py is promised and does not exist."
    );
    assert!(f.root().join("auth/service.py").exists());
    f.close().await;
}

#[tokio::test]
#[ignore = "needs pyright-langserver on PATH"]
async fn live_an_unpromised_function_in_a_created_file_is_listed() {
    let f = Fixture::new(&[(
        "auth/tokens.py",
        "def issue() -> None:\n    pass\n\n\ndef helper() -> None:\n    pass\n",
    )])
    .await;
    let c = contract(
        &[("auth/tokens.py", FileAction::Create)],
        &[("s1", "auth/tokens.py", "issue")],
    );
    let verdict = f.record.verify("a3", &f.project, 1, &c).await.unwrap();
    assert_eq!(verdict.outcome(), VerdictOutcome::Passed, "{verdict:?}");
    assert_eq!(
        verdict.unpromised,
        vec![Unpromised {
            path: "auth/tokens.py".into(),
            name: "helper".into()
        }]
    );
    assert_eq!(verdict.lines(), "unpromised: helper in auth/tokens.py.");
    f.close().await;
}

#[tokio::test]
#[ignore = "needs pyright-langserver on PATH"]
async fn live_a_file_no_server_reads_is_unchecked() {
    let f = Fixture::new(&[
        ("auth/schema.sql", "CREATE TABLE token (id TEXT);\n"),
        ("auth/service.py", SERVICE),
    ])
    .await;
    let c = contract(
        &[("auth/schema.sql", FileAction::Create)],
        &[("s1", "auth/schema.sql", "token")],
    );
    let verdict = f.record.verify("a4", &f.project, 4, &c).await.unwrap();
    assert_eq!(verdict.symbols[0].state, SymbolState::Unchecked);
    assert_eq!(verdict.outcome(), VerdictOutcome::NotRun);
    assert_eq!(
        verdict.lines(),
        "unchecked: auth/schema.sql (no language server reads it)."
    );
    f.close().await;
}
