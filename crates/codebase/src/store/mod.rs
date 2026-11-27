//! The code index SQL over the shared storage handle.

mod inventory;
mod links;
mod reach;
mod servers;

pub(crate) use inventory::InventoryRow;
pub(crate) use servers::{AddedRow, ServerChange};

use std::collections::HashMap;
use std::sync::Arc;

use sqlx::SqliteConnection;

use super::Codebase;
use super::model::Skipped;
use core_db::{Storage, StorageError};
use core_projects::{ProjectId, Projects};

/// What `code_file` holds for one file: its stamp, and why it was not
/// parsed, if it was not.
pub(super) struct FileRow {
    pub(super) path_key: String,
    pub(super) path: String,
    pub(super) size: i64,
    pub(super) modified_ms: i64,
    pub(super) language: &'static str,
    pub(super) skipped: Option<&'static str>,
}

/// A project in a question's scope: its id, slug and folder.
pub(super) type ScopeRow = (ProjectId, String, Option<String>);

/// Private persistence for the code service.
pub(super) struct CodeStore {
    pub(super) storage: Arc<Storage>,
}

impl CodeStore {
    pub(super) fn with_storage(storage: Arc<Storage>) -> Self {
        Self { storage }
    }

    /// A file's stored path, size and modified time, or `None` before its first index.
    pub(super) async fn code_file_stamp(
        &self,
        project: &ProjectId,
        path_key: &str,
    ) -> Result<Option<(String, i64, i64)>, StorageError> {
        Ok(sqlx::query_as(
            "SELECT path, size, modified_ms FROM code_file WHERE project_id = ? AND path_key = ?",
        )
        .bind(project.as_str())
        .bind(path_key)
        .fetch_optional(self.storage.reader())
        .await?)
    }

    /// Every stored stamp of the project, by file key: what one scan compares
    /// its whole walk against.
    pub(super) async fn code_file_stamps(
        &self,
        project: &ProjectId,
    ) -> Result<HashMap<String, (String, i64, i64)>, StorageError> {
        let rows: Vec<(String, String, i64, i64)> = sqlx::query_as(
            "SELECT path_key, path, size, modified_ms FROM code_file WHERE project_id = ?",
        )
        .bind(project.as_str())
        .fetch_all(self.storage.reader())
        .await?;
        Ok(rows
            .into_iter()
            .map(|(key, path, size, modified_ms)| (key, (path, size, modified_ms)))
            .collect())
    }

    /// Every file key the project's index holds, by key.
    pub(super) async fn code_file_keys(
        &self,
        project: &ProjectId,
    ) -> Result<Vec<String>, StorageError> {
        Ok(sqlx::query_scalar(
            "SELECT path_key FROM code_file WHERE project_id = ? ORDER BY path_key",
        )
        .bind(project.as_str())
        .fetch_all(self.storage.reader())
        .await?)
    }

    /// Whether the index holds a file at `folder_key` or under it; "under" is
    /// the byte range `key/` up to `key0` (`0` follows `/`), exact on every
    /// platform, where `LIKE` ignores ASCII case.
    pub(super) async fn code_files_under(
        &self,
        project: &ProjectId,
        folder_key: &str,
    ) -> Result<bool, StorageError> {
        Ok(sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM code_file WHERE project_id = ?
                 AND (path_key = ? OR (path_key >= ? || '/' AND path_key < ? || '0')))",
        )
        .bind(project.as_str())
        .bind(folder_key)
        .bind(folder_key)
        .bind(folder_key)
        .fetch_one(self.storage.reader())
        .await?)
    }

    /// One write: the file's row is upserted.
    pub(super) async fn write_code_file(
        &self,
        project: &ProjectId,
        file: FileRow,
    ) -> Result<(), StorageError> {
        let project = project.clone();
        self.storage
            .write_txn(move |conn| {
                Box::pin(async move {
                    // A worker still finishing a file when its project is removed
                    // writes nothing: this write and the removal are serialized.
                    match Projects::require_live_in(conn, &project).await {
                        Ok(()) => {}
                        Err(StorageError::NotFound(_)) => return Ok(()),
                        Err(error) => return Err(error),
                    }
                    sqlx::query(
                        "INSERT INTO code_file
                       (project_id, path_key, path, size, modified_ms, language, skipped_reason)
                     VALUES (?,?,?,?,?,?,?)
                     ON CONFLICT (project_id, path_key) DO UPDATE SET
                       path = excluded.path, size = excluded.size,
                       modified_ms = excluded.modified_ms, language = excluded.language,
                       skipped_reason = excluded.skipped_reason",
                    )
                    .bind(project.as_str())
                    .bind(&file.path_key)
                    .bind(&file.path)
                    .bind(file.size)
                    .bind(file.modified_ms)
                    .bind(file.language)
                    .bind(file.skipped)
                    .execute(&mut *conn)
                    .await?;
                    Ok(())
                })
            })
            .await
    }

    /// One write: a file's row goes.
    pub(super) async fn delete_code_file(
        &self,
        project: &ProjectId,
        path_key: &str,
    ) -> Result<(), StorageError> {
        let (project, key) = (project.clone(), path_key.to_string());
        self.storage
            .write_txn(move |conn| {
                Box::pin(async move {
                    sqlx::query("DELETE FROM code_file WHERE project_id = ? AND path_key = ?")
                        .bind(project.as_str())
                        .bind(&key)
                        .execute(&mut *conn)
                        .await?;
                    Ok(())
                })
            })
            .await
    }

    /// The asker's project, then its live direct links in slug order.
    pub(super) async fn code_scope(&self, home: &ProjectId) -> Result<Vec<ScopeRow>, StorageError> {
        let mut snapshot = self.storage.reader().begin().await?;
        let first = Projects::get_in(&mut snapshot, home).await?;
        let linked: Vec<String> = sqlx::query_scalar(
            "SELECT linked_project_id FROM project_link WHERE project_id = ?
              ORDER BY linked_project_id",
        )
        .bind(home.as_str())
        .fetch_all(&mut *snapshot)
        .await?;
        let mut rows = Vec::new();
        for id in linked {
            match Projects::get_in(&mut snapshot, &ProjectId::from_recorded(id)).await {
                Ok(p) => rows.push((p.id, p.slug, p.directory)),
                Err(StorageError::NotFound(_)) => {}
                Err(error) => return Err(error),
            }
        }
        rows.sort_by(|a, b| a.1.cmp(&b.1));
        let mut out = vec![(first.id, first.slug, first.directory)];
        out.extend(rows);
        snapshot.commit().await?;
        Ok(out)
    }

    /// The project's indexed files, and its skipped files by reason.
    pub(super) async fn code_counts(
        &self,
        project: &ProjectId,
    ) -> Result<(u32, Vec<Skipped>), StorageError> {
        let files: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM code_file WHERE project_id = ? AND skipped_reason IS NULL",
        )
        .bind(project.as_str())
        .fetch_one(self.storage.reader())
        .await?;
        let skipped: Vec<(String, i64)> = sqlx::query_as(
            "SELECT skipped_reason, COUNT(*) FROM code_file
              WHERE project_id = ? AND skipped_reason IS NOT NULL
              GROUP BY skipped_reason ORDER BY skipped_reason",
        )
        .bind(project.as_str())
        .fetch_all(self.storage.reader())
        .await?;
        let skipped = skipped
            .into_iter()
            .map(|(reason, count)| Skipped {
                reason,
                count: count as u32,
            })
            .collect();
        Ok((files as u32, skipped))
    }
}

impl Codebase {
    /// Removes an index in the caller's project-removal transaction.
    pub async fn delete_code_index_in(
        conn: &mut SqliteConnection,
        project: &ProjectId,
    ) -> Result<(), StorageError> {
        sqlx::query("DELETE FROM code_file WHERE project_id = ?")
            .bind(project.as_str())
            .execute(&mut *conn)
            .await?;
        sqlx::query("DELETE FROM code_inventory WHERE project_id = ?")
            .bind(project.as_str())
            .execute(&mut *conn)
            .await?;
        sqlx::query("DELETE FROM code_inventory_scan WHERE project_id = ?")
            .bind(project.as_str())
            .execute(&mut *conn)
            .await?;
        Ok(())
    }
}
