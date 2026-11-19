//! One job: the code record's persistence (its verdicts, and the record a
//! done mark writes), private to the code record.

use std::sync::Arc;

use core_db::{Storage, StorageError, Tx};
use core_plans::{FileAction, WorkflowId};
use core_projects::ProjectId;

use crate::{CodeRecord, CodeRecordError, DoneRecord, SymbolState, Unpromised, Verdict};

#[derive(Clone)]
pub(crate) struct CodeRecordStore {
    storage: Arc<Storage>,
}

impl CodeRecordStore {
    pub(super) fn new(storage: Arc<Storage>) -> Self {
        Self { storage }
    }

    /// Stores the verdict under `key`, replacing one stored before.
    pub(super) async fn put(
        &self,
        key: &str,
        project: &ProjectId,
        verdict: &Verdict,
    ) -> Result<(), CodeRecordError> {
        let (key, project, json, ts) = (
            key.to_string(),
            project.clone(),
            serde_json::to_string(verdict)?,
            Storage::now(),
        );
        self.storage
            .write_txn(move |conn| {
                Box::pin(async move {
                    sqlx::query(
                        "INSERT INTO code_record_verdict (attempt_id, project_id, verdict_json, verified_at)
                         VALUES (?, ?, ?, ?)
                         ON CONFLICT (attempt_id) DO UPDATE SET
                           project_id = excluded.project_id,
                           verdict_json = excluded.verdict_json,
                           verified_at = excluded.verified_at",
                    )
                    .bind(&key)
                    .bind(project.as_str())
                    .bind(&json)
                    .bind(&ts)
                    .execute(&mut *conn)
                    .await?;
                    Ok(())
                })
            })
            .await?;
        Ok(())
    }

    pub(super) async fn verdict(&self, key: &str) -> Result<Option<Verdict>, CodeRecordError> {
        let json: Option<String> =
            sqlx::query_scalar("SELECT verdict_json FROM code_record_verdict WHERE attempt_id = ?")
                .bind(key)
                .fetch_optional(self.storage.reader())
                .await?;
        Ok(json.map(|j| serde_json::from_str(&j)).transpose()?)
    }
}

impl CodeRecord {
    /// Every verdict and record row of a removed project, inside the
    /// removal's own write.
    pub async fn delete_project_in(conn: &mut Tx, project: &ProjectId) -> Result<(), StorageError> {
        let project = project.as_str();
        sqlx::query("DELETE FROM code_record_verdict WHERE project_id = ?")
            .bind(project)
            .execute(&mut *conn)
            .await?;
        sqlx::query("DELETE FROM code_record_file WHERE project_id = ?")
            .bind(project)
            .execute(&mut *conn)
            .await?;
        sqlx::query("DELETE FROM code_record_symbol WHERE project_id = ?")
            .bind(project)
            .execute(&mut *conn)
            .await?;
        sqlx::query("DELETE FROM code_record_unpromised WHERE project_id = ?")
            .bind(project)
            .execute(&mut *conn)
            .await?;
        Ok(())
    }
}

/// One contract file as its row stores it.
struct RecordedFile<'a> {
    path: &'a str,
    action: String,
    purpose: &'a str,
}

/// One promised symbol as its row stores it.
struct RecordedSymbol<'a> {
    id: &'a str,
    path: &'a str,
    name: &'a str,
    kind: String,
    contract_json: String,
    state: String,
}

impl DoneRecord {
    /// Every file the task created or modified: what it left in the code.
    fn files(&self) -> Result<Vec<RecordedFile<'_>>, StorageError> {
        self.contract
            .files
            .iter()
            .filter(|file| matches!(file.action, FileAction::Create | FileAction::Modify))
            .map(|file| {
                Ok(RecordedFile {
                    path: &file.path,
                    action: text(&file.action)?,
                    purpose: &file.purpose,
                })
            })
            .collect()
    }

    /// Every promised symbol, its state the verdict's for its id and path:
    /// unchecked when there is no verdict or it does not name the symbol.
    fn symbols(&self) -> Result<Vec<RecordedSymbol<'_>>, StorageError> {
        self.contract
            .symbols
            .iter()
            .map(|symbol| {
                let state = self
                    .verdict
                    .as_ref()
                    .and_then(|v| {
                        v.symbols
                            .iter()
                            .find(|p| p.id == symbol.id && p.path == symbol.path)
                    })
                    .map_or(SymbolState::Unchecked, |p| p.state);
                Ok(RecordedSymbol {
                    id: &symbol.id,
                    path: &symbol.path,
                    name: &symbol.name,
                    kind: text(&symbol.kind)?,
                    contract_json: serde_json::to_string(symbol)?,
                    state: text(&state)?,
                })
            })
            .collect()
    }

    /// The verdict's unpromised symbols; none without a verdict.
    fn unpromised(&self) -> &[Unpromised] {
        self.verdict
            .as_ref()
            .map(|v| v.unpromised.as_slice())
            .unwrap_or_default()
    }
}

/// A snake_case enum value as its stored text: `create`, `function`, `found`.
fn text(value: &impl serde::Serialize) -> Result<String, StorageError> {
    match serde_json::to_value(value)? {
        serde_json::Value::String(s) => Ok(s),
        other => Ok(other.to_string()),
    }
}

impl CodeRecord {
    /// Replaces the task's record with `done`, inside the marking write: each
    /// contract file, each promised symbol with its state from the verdict,
    /// and the verdict's unpromised symbols.
    pub async fn record_done_in(tx: &mut Tx, done: &DoneRecord) -> Result<(), StorageError> {
        let (files, symbols) = (done.files()?, done.symbols()?);
        Self::forget_done_in(tx, &done.workflow, done.version, done.task).await?;
        let (project, workflow, version, task) = (
            done.project.as_str(),
            done.workflow.as_str(),
            done.version,
            i64::from(done.task),
        );
        let at = Storage::now();
        for file in files {
            sqlx::query(
                "INSERT INTO code_record_file (project_id, workflow_id, version, task, path,
                   action, purpose, commit_sha, recorded_at)
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
            )
            .bind(project)
            .bind(workflow)
            .bind(version)
            .bind(task)
            .bind(file.path)
            .bind(&file.action)
            .bind(file.purpose)
            .bind(done.commit.as_deref())
            .bind(&at)
            .execute(&mut *tx)
            .await?;
        }
        for symbol in symbols {
            sqlx::query(
                "INSERT INTO code_record_symbol (project_id, workflow_id, version, task,
                   symbol_id, path, name, kind, contract_json, state)
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            )
            .bind(project)
            .bind(workflow)
            .bind(version)
            .bind(task)
            .bind(symbol.id)
            .bind(symbol.path)
            .bind(symbol.name)
            .bind(&symbol.kind)
            .bind(&symbol.contract_json)
            .bind(&symbol.state)
            .execute(&mut *tx)
            .await?;
        }
        for u in done.unpromised() {
            sqlx::query(
                "INSERT OR IGNORE INTO code_record_unpromised
                   (project_id, workflow_id, version, task, path, name)
                 VALUES (?, ?, ?, ?, ?, ?)",
            )
            .bind(project)
            .bind(workflow)
            .bind(version)
            .bind(task)
            .bind(&u.path)
            .bind(&u.name)
            .execute(&mut *tx)
            .await?;
        }
        Ok(())
    }

    /// Deletes the task's record, inside the write that marks it not done.
    pub async fn forget_done_in(
        tx: &mut Tx,
        workflow: &WorkflowId,
        version: i64,
        task: u32,
    ) -> Result<(), StorageError> {
        let (workflow, task) = (workflow.as_str(), i64::from(task));
        sqlx::query(
            "DELETE FROM code_record_file WHERE workflow_id = ? AND version = ? AND task = ?",
        )
        .bind(workflow)
        .bind(version)
        .bind(task)
        .execute(&mut *tx)
        .await?;
        sqlx::query(
            "DELETE FROM code_record_symbol WHERE workflow_id = ? AND version = ? AND task = ?",
        )
        .bind(workflow)
        .bind(version)
        .bind(task)
        .execute(&mut *tx)
        .await?;
        sqlx::query(
            "DELETE FROM code_record_unpromised WHERE workflow_id = ? AND version = ? AND task = ?",
        )
        .bind(workflow)
        .bind(version)
        .bind(task)
        .execute(&mut *tx)
        .await?;
        Ok(())
    }
}
