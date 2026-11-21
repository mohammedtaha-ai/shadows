//! One job: reading the record, for a question or whole. Every read is on one
//! project, ordered by explicit keys, and each matching query stops at 21 rows.

use core_projects::ProjectId;

use super::CodeRecordStore;
use crate::CodeRecordError;

/// The bound of each matching query: one more than the entries a question lists.
const LOOKUP_LIMIT: i64 = 21;

/// The words a words query tests in SQL; further words are tested on the rows it returns.
const WORD_SLOTS: usize = 4;

/// One `code_record_file` row.
pub(crate) struct FileRow {
    pub path: String,
    pub workflow_id: String,
    pub version: i64,
    pub task: i64,
    pub action: String,
    pub purpose: String,
    pub commit: Option<String>,
}

/// One `code_record_symbol` row.
pub(crate) struct SymbolRow {
    pub path: String,
    pub workflow_id: String,
    pub version: i64,
    pub task: i64,
    pub symbol_id: String,
    pub name: String,
    pub kind: String,
    pub contract_json: String,
    pub state: String,
}

/// Every recorded row of some paths.
#[derive(Default)]
pub(crate) struct Detail {
    pub files: Vec<FileRow>,
    pub symbols: Vec<SymbolRow>,
}

type FileTuple = (String, String, i64, i64, String, String, Option<String>);
type SymbolTuple = (
    String,
    String,
    i64,
    i64,
    String,
    String,
    String,
    String,
    String,
);

impl CodeRecordStore {
    /// Paths of recorded files equal to `path` or under it as a folder.
    pub(super) async fn read_files_by_path(
        &self,
        project: &ProjectId,
        path: &str,
    ) -> Result<Vec<String>, CodeRecordError> {
        self.read_files_under(project, &[path.to_string()]).await
    }

    /// Paths of recorded files equal to one of `folders` or under it, one query
    /// per folder. LIKE's `%`, `_` and `\` in a folder are escaped, so they
    /// match only themselves.
    pub(super) async fn read_files_under(
        &self,
        project: &ProjectId,
        folders: &[String],
    ) -> Result<Vec<String>, CodeRecordError> {
        let mut paths = Vec::new();
        for folder in folders
            .iter()
            .map(|f| f.trim_matches('/'))
            .filter(|f| !f.is_empty())
        {
            paths.extend(
                sqlx::query_scalar::<_, String>(
                    "SELECT path FROM code_record_file WHERE project_id = ?1
                       AND (path = ?2 OR path LIKE
                            replace(replace(replace(?2, '\\', '\\\\'), '%', '\\%'), '_', '\\_')
                            || '/%' ESCAPE '\\')
                     ORDER BY path, workflow_id, version DESC, task DESC LIMIT ?3",
                )
                .bind(project.as_str())
                .bind(folder)
                .bind(LOOKUP_LIMIT)
                .fetch_all(self.storage.reader())
                .await?,
            );
        }
        Ok(distinct(paths))
    }

    /// Paths of recorded symbols named `name`, or a method `X.name`.
    pub(super) async fn read_symbols_named(
        &self,
        project: &ProjectId,
        name: &str,
    ) -> Result<Vec<String>, CodeRecordError> {
        let paths = sqlx::query_scalar::<_, String>(
            "SELECT path FROM code_record_symbol WHERE project_id = ?1
               AND (name = ?2 OR (length(name) > length(?2) + 1
                    AND substr(name, length(name) - length(?2)) = '.' || ?2))
             ORDER BY path, workflow_id, version DESC, task DESC LIMIT ?3",
        )
        .bind(project.as_str())
        .bind(name)
        .bind(LOOKUP_LIMIT)
        .fetch_all(self.storage.reader())
        .await?;
        Ok(distinct(paths))
    }

    /// Paths of files whose purpose, or of symbols whose contract, holds every
    /// (lowercase) word. An unused slot binds `''`, which `instr` always finds.
    pub(super) async fn read_rows_with_words(
        &self,
        project: &ProjectId,
        words: &[String],
    ) -> Result<Vec<String>, CodeRecordError> {
        if words.is_empty() {
            return Ok(Vec::new());
        }
        let slot = |n: usize| words.get(n).map_or("", String::as_str);
        let files = sqlx::query_as::<_, (String, String)>(
            "SELECT path, lower(purpose) FROM code_record_file WHERE project_id = ?1
               AND instr(lower(purpose), ?2) > 0 AND instr(lower(purpose), ?3) > 0
               AND instr(lower(purpose), ?4) > 0 AND instr(lower(purpose), ?5) > 0
             ORDER BY path, workflow_id, version DESC, task DESC LIMIT ?6",
        )
        .bind(project.as_str())
        .bind(slot(0))
        .bind(slot(1))
        .bind(slot(2))
        .bind(slot(3))
        .bind(LOOKUP_LIMIT)
        .fetch_all(self.storage.reader())
        .await?;
        let symbols = sqlx::query_as::<_, (String, String)>(
            "SELECT path, lower(contract_json) FROM code_record_symbol WHERE project_id = ?1
               AND instr(lower(contract_json), ?2) > 0 AND instr(lower(contract_json), ?3) > 0
               AND instr(lower(contract_json), ?4) > 0 AND instr(lower(contract_json), ?5) > 0
             ORDER BY path, workflow_id, version DESC, task DESC LIMIT ?6",
        )
        .bind(project.as_str())
        .bind(slot(0))
        .bind(slot(1))
        .bind(slot(2))
        .bind(slot(3))
        .bind(LOOKUP_LIMIT)
        .fetch_all(self.storage.reader())
        .await?;
        let rest = words.get(WORD_SLOTS..).unwrap_or_default();
        let mut paths: Vec<String> = files
            .into_iter()
            .chain(symbols)
            .filter(|(_, text)| rest.iter().all(|w| text.contains(w.as_str())))
            .map(|(path, _)| path)
            .collect();
        paths.sort();
        Ok(distinct(paths))
    }

    /// Every file and symbol row of `paths`, one query per path and table: by
    /// path, then version and task newest first, then workflow id.
    pub(super) async fn read_files_detail(
        &self,
        project: &ProjectId,
        paths: &[String],
    ) -> Result<Detail, CodeRecordError> {
        let mut detail = Detail::default();
        for path in paths {
            let files = sqlx::query_as::<_, FileTuple>(
                "SELECT path, workflow_id, version, task, action, purpose, commit_sha
                   FROM code_record_file WHERE project_id = ? AND path = ?
                 ORDER BY version DESC, task DESC, workflow_id",
            )
            .bind(project.as_str())
            .bind(path)
            .fetch_all(self.storage.reader())
            .await?;
            detail.files.extend(files.into_iter().map(
                |(path, workflow_id, version, task, action, purpose, commit)| FileRow {
                    path,
                    workflow_id,
                    version,
                    task,
                    action,
                    purpose,
                    commit,
                },
            ));
            let symbols = sqlx::query_as::<_, SymbolTuple>(
                "SELECT path, workflow_id, version, task, symbol_id, name, kind, contract_json,
                        state
                   FROM code_record_symbol WHERE project_id = ? AND path = ?
                 ORDER BY version DESC, task DESC, workflow_id, name, symbol_id",
            )
            .bind(project.as_str())
            .bind(path)
            .fetch_all(self.storage.reader())
            .await?;
            detail.symbols.extend(symbols.into_iter().map(
                |(
                    path,
                    workflow_id,
                    version,
                    task,
                    symbol_id,
                    name,
                    kind,
                    contract_json,
                    state,
                )| {
                    SymbolRow {
                        path,
                        workflow_id,
                        version,
                        task,
                        symbol_id,
                        name,
                        kind,
                        contract_json,
                        state,
                    }
                },
            ));
        }
        Ok(detail)
    }
}

impl CodeRecordStore {
    /// Every file and symbol row of the project, and its count of done marks,
    /// in one read snapshot: by path, then version and task newest first, then
    /// workflow id (and a symbol by name, then id).
    pub(super) async fn read_whole(
        &self,
        project: &ProjectId,
    ) -> Result<(Detail, u32), CodeRecordError> {
        let mut tx = self.storage.reader().begin().await?;
        let files = sqlx::query_as::<_, FileTuple>(
            "SELECT path, workflow_id, version, task, action, purpose, commit_sha
               FROM code_record_file WHERE project_id = ?
             ORDER BY path, version DESC, task DESC, workflow_id",
        )
        .bind(project.as_str())
        .fetch_all(&mut *tx)
        .await?;
        let symbols = sqlx::query_as::<_, SymbolTuple>(
            "SELECT path, workflow_id, version, task, symbol_id, name, kind, contract_json, state
               FROM code_record_symbol WHERE project_id = ?
             ORDER BY path, version DESC, task DESC, workflow_id, name, symbol_id",
        )
        .bind(project.as_str())
        .fetch_all(&mut *tx)
        .await?;
        // A done mark writes its rows into these three tables, and only it does.
        let marks: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM (
               SELECT workflow_id, version, task FROM code_record_file WHERE project_id = ?1
               UNION SELECT workflow_id, version, task FROM code_record_symbol WHERE project_id = ?1
               UNION SELECT workflow_id, version, task FROM code_record_element WHERE project_id = ?1)",
        )
        .bind(project.as_str())
        .fetch_one(&mut *tx)
        .await?;
        tx.commit().await?;
        let detail = Detail {
            files: files.into_iter().map(FileRow::from).collect(),
            symbols: symbols.into_iter().map(SymbolRow::from).collect(),
        };
        Ok((detail, u32::try_from(marks).unwrap_or(u32::MAX)))
    }
}

impl From<FileTuple> for FileRow {
    fn from((path, workflow_id, version, task, action, purpose, commit): FileTuple) -> Self {
        FileRow {
            path,
            workflow_id,
            version,
            task,
            action,
            purpose,
            commit,
        }
    }
}

impl From<SymbolTuple> for SymbolRow {
    fn from(
        (path, workflow_id, version, task, symbol_id, name, kind, contract_json, state): SymbolTuple,
    ) -> Self {
        SymbolRow {
            path,
            workflow_id,
            version,
            task,
            symbol_id,
            name,
            kind,
            contract_json,
            state,
        }
    }
}

/// Sorted paths without repeats.
fn distinct(mut paths: Vec<String>) -> Vec<String> {
    paths.sort();
    paths.dedup();
    paths
}
