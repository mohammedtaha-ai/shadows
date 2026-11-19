//! The record a done mark writes, read back from its tables.

use std::sync::Arc;

use code_record::{CodeRecord, DoneRecord, PromisedSymbol, SymbolState, Unpromised, Verdict};
use core_db::Storage;
use core_plans::{FileAction, FileWork, SymbolKind, SymbolWork, TaskContract, WorkflowId};
use core_projects::ProjectId;

struct Fixture {
    _db: tempfile::TempDir,
    storage: Arc<Storage>,
}

impl Fixture {
    async fn new() -> Fixture {
        let db = tempfile::tempdir().unwrap();
        let storage = Arc::new(
            Storage::open(&db.path().join("record.sqlite3"))
                .await
                .unwrap(),
        );
        Fixture { _db: db, storage }
    }

    async fn record(&self, done: DoneRecord) {
        self.storage
            .write_txn(move |tx| {
                Box::pin(async move { CodeRecord::record_done_in(tx, &done).await })
            })
            .await
            .unwrap();
    }

    async fn forget(&self, task: u32) {
        self.storage
            .write_txn(move |tx| {
                Box::pin(async move { CodeRecord::forget_done_in(tx, &workflow(), 1, task).await })
            })
            .await
            .unwrap();
    }

    async fn files(&self) -> Vec<(String, String, Option<String>)> {
        sqlx::query_as("SELECT path, action, commit_sha FROM code_record_file ORDER BY task, path")
            .fetch_all(self.storage.reader())
            .await
            .unwrap()
    }

    async fn symbols(&self) -> Vec<(String, String, String, String)> {
        sqlx::query_as(
            "SELECT symbol_id, name, kind, state FROM code_record_symbol ORDER BY task, symbol_id",
        )
        .fetch_all(self.storage.reader())
        .await
        .unwrap()
    }

    async fn unpromised(&self) -> Vec<(String, String)> {
        sqlx::query_as("SELECT path, name FROM code_record_unpromised ORDER BY path, name")
            .fetch_all(self.storage.reader())
            .await
            .unwrap()
    }
}

fn workflow() -> WorkflowId {
    WorkflowId::from_recorded("wf-1".into())
}

fn contract(names: &[(&str, &str)]) -> TaskContract {
    serde_json::from_value(serde_json::json!({
        "files": [{"path": "auth/service.py", "action": "create", "purpose": "the service"}],
        "symbols": names.iter().map(|(id, name)| serde_json::json!({
            "id": id, "path": "auth/service.py", "kind": "function", "name": name,
            "returns": {"name": "", "type_name": "None", "meaning": "m"},
            "behavior": ["b"],
        })).collect::<Vec<_>>(),
    }))
    .unwrap()
}

fn verdict(states: &[(&str, &str, SymbolState)], unpromised: &[&str]) -> Verdict {
    Verdict {
        task: 1,
        symbols: states
            .iter()
            .map(|(id, name, state)| PromisedSymbol {
                id: id.to_string(),
                path: "auth/service.py".into(),
                name: name.to_string(),
                state: *state,
                why: None,
            })
            .collect(),
        unpromised: unpromised
            .iter()
            .map(|name| Unpromised {
                path: "auth/service.py".into(),
                name: name.to_string(),
            })
            .collect(),
        unchecked_files: Vec::new(),
    }
}

fn done(project: &str, contract: TaskContract, verdict: Option<Verdict>) -> DoneRecord {
    DoneRecord {
        project: ProjectId::from_recorded(project.into()),
        workflow: workflow(),
        version: 1,
        task: 1,
        attempt: Some("a-1".into()),
        commit: Some("abc".into()),
        contract,
        verdict,
    }
}

#[tokio::test]
async fn done_writes_the_record_and_not_done_removes_it() {
    let f = Fixture::new().await;
    let v = verdict(
        &[
            ("s1", "login", SymbolState::Found),
            ("s2", "logout", SymbolState::Missing),
        ],
        &["helper"],
    );
    f.record(done(
        "p",
        contract(&[("s1", "login"), ("s2", "logout")]),
        Some(v),
    ))
    .await;
    assert_eq!(
        f.files().await,
        vec![(
            "auth/service.py".into(),
            "create".into(),
            Some("abc".into())
        )]
    );
    assert_eq!(
        f.symbols().await,
        vec![
            (
                "s1".into(),
                "login".into(),
                "function".into(),
                "found".into()
            ),
            (
                "s2".into(),
                "logout".into(),
                "function".into(),
                "missing".into()
            ),
        ]
    );
    assert_eq!(
        f.unpromised().await,
        vec![("auth/service.py".into(), "helper".into())]
    );
    let json: String =
        sqlx::query_scalar("SELECT contract_json FROM code_record_symbol WHERE symbol_id = 's1'")
            .fetch_one(f.storage.reader())
            .await
            .unwrap();
    let symbol: SymbolWork = serde_json::from_str(&json).unwrap();
    assert_eq!(symbol.kind, SymbolKind::Function);

    f.forget(1).await;
    assert!(f.files().await.is_empty());
    assert!(f.symbols().await.is_empty());
    assert!(f.unpromised().await.is_empty());
    f.storage.close().await.unwrap();
}

#[tokio::test]
async fn a_second_done_replaces_the_rows() {
    let f = Fixture::new().await;
    let v = verdict(&[("s1", "login", SymbolState::Found)], &["helper"]);
    f.record(done(
        "p",
        contract(&[("s1", "login"), ("s2", "logout")]),
        Some(v),
    ))
    .await;
    // No verdict: every promised symbol is unchecked and none is unpromised.
    let mut second = contract(&[("s1", "login")]);
    second.files.push(FileWork {
        path: "auth/schema.sql".into(),
        action: FileAction::Modify,
        purpose: "p".into(),
    });
    // A file the task only reads is not one it left in the code.
    second.files.push(FileWork {
        path: "auth/readme.md".into(),
        action: FileAction::Read,
        purpose: "p".into(),
    });
    f.record(done("p", second, None)).await;
    assert_eq!(
        f.files().await,
        vec![
            (
                "auth/schema.sql".into(),
                "modify".into(),
                Some("abc".into())
            ),
            (
                "auth/service.py".into(),
                "create".into(),
                Some("abc".into())
            ),
        ]
    );
    assert_eq!(
        f.symbols().await,
        vec![(
            "s1".into(),
            "login".into(),
            "function".into(),
            "unchecked".into()
        )]
    );
    assert!(f.unpromised().await.is_empty());
    f.storage.close().await.unwrap();
}

#[tokio::test]
async fn a_task_without_a_contract_records_nothing() {
    let f = Fixture::new().await;
    f.record(done("p", contract(&[]), None)).await;
    f.record(done(
        "p",
        serde_json::from_str::<TaskContract>("{}").unwrap(),
        None,
    ))
    .await;
    assert!(f.files().await.is_empty());
    assert!(f.symbols().await.is_empty());
    f.storage.close().await.unwrap();
}

#[tokio::test]
async fn removing_the_project_removes_its_record() {
    let f = Fixture::new().await;
    let v = verdict(&[("s1", "login", SymbolState::Found)], &["helper"]);
    f.record(done("p", contract(&[("s1", "login")]), Some(v)))
        .await;
    let mut other = done("q", contract(&[("s1", "login")]), None);
    other.task = 2;
    f.record(other).await;
    f.storage
        .write_txn(|tx| {
            Box::pin(async move {
                CodeRecord::delete_project_in(tx, &ProjectId::from_recorded("p".into())).await
            })
        })
        .await
        .unwrap();
    assert_eq!(
        f.symbols().await,
        vec![(
            "s1".into(),
            "login".into(),
            "function".into(),
            "unchecked".into()
        )]
    );
    assert!(f.unpromised().await.is_empty());
    assert_eq!(f.files().await.len(), 1);
    f.storage.close().await.unwrap();
}
