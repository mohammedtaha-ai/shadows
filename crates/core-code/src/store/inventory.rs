//! One job: file inventory persistence.

use std::collections::HashMap;

use super::CodeStore;
use crate::model::{InventoryEntry, InventoryKind};
use core_db::StorageError;
use core_projects::{ProjectId, Projects};

/// One listed path as `code_inventory` holds it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct InventoryRow {
    pub(crate) path_key: String,
    pub(crate) path: String,
    pub(crate) kind: &'static str,
    pub(crate) excluded_reason: Option<&'static str>,
}

/// A stored page: generation, incomplete, observed_at, and `(path_key, entry)` rows.
pub(crate) type StoredPage = (i64, bool, String, Vec<(String, InventoryEntry)>);

type StoredRow = (String, String, String, Option<String>, bool);

impl CodeStore {
    /// One write: the listing's rows are inserted or updated; the keys it
    /// lacks are deleted only when it is complete. The generation moves by
    /// one when any row changed, and is 1 on the first write.
    pub(crate) async fn write_inventory(
        &self,
        project: &ProjectId,
        rows: Vec<InventoryRow>,
        incomplete: bool,
        observed_at: &str,
    ) -> Result<(), StorageError> {
        let (project, observed_at) = (project.clone(), observed_at.to_string());
        self.storage
            .write_txn(move |conn| {
                Box::pin(async move {
                    // A listing that ends after its project was removed writes nothing.
                    match Projects::require_live_in(conn, &project).await {
                        Ok(()) => {}
                        Err(StorageError::NotFound(_)) => return Ok(()),
                        Err(error) => return Err(error),
                    }
                    let stored: Vec<(String, String, String, Option<String>)> = sqlx::query_as(
                        "SELECT path_key, path, kind, excluded_reason FROM code_inventory
                          WHERE project_id = ? ORDER BY path_key",
                    )
                    .bind(project.as_str())
                    .fetch_all(&mut *conn)
                    .await?;
                    let mut stored: HashMap<String, (String, String, Option<String>)> = stored
                        .into_iter()
                        .map(|(key, path, kind, reason)| (key, (path, kind, reason)))
                        .collect();
                    let mut changed = false;
                    for row in &rows {
                        let now = (
                            row.path.clone(),
                            row.kind.to_string(),
                            row.excluded_reason.map(String::from),
                        );
                        if stored.remove(&row.path_key).as_ref() == Some(&now) {
                            continue;
                        }
                        changed = true;
                        sqlx::query(
                            "INSERT INTO code_inventory
                               (project_id, path_key, path, kind, excluded_reason)
                             VALUES (?,?,?,?,?)
                             ON CONFLICT (project_id, path_key) DO UPDATE SET
                               path = excluded.path, kind = excluded.kind,
                               excluded_reason = excluded.excluded_reason",
                        )
                        .bind(project.as_str())
                        .bind(&row.path_key)
                        .bind(&row.path)
                        .bind(row.kind)
                        .bind(row.excluded_reason)
                        .execute(&mut *conn)
                        .await?;
                    }
                    if !incomplete {
                        let mut gone: Vec<&String> = stored.keys().collect();
                        gone.sort_unstable();
                        for key in gone {
                            changed = true;
                            sqlx::query(
                                "DELETE FROM code_inventory WHERE project_id = ? AND path_key = ?",
                            )
                            .bind(project.as_str())
                            .bind(key)
                            .execute(&mut *conn)
                            .await?;
                        }
                    }
                    let before: Option<i64> = sqlx::query_scalar(
                        "SELECT generation FROM code_inventory_scan WHERE project_id = ?",
                    )
                    .bind(project.as_str())
                    .fetch_optional(&mut *conn)
                    .await?;
                    let generation = match before {
                        None => 1,
                        Some(g) => g + i64::from(changed),
                    };
                    sqlx::query(
                        "INSERT INTO code_inventory_scan
                           (project_id, generation, incomplete, observed_at)
                         VALUES (?,?,?,?)
                         ON CONFLICT (project_id) DO UPDATE SET
                           generation = excluded.generation, incomplete = excluded.incomplete,
                           observed_at = excluded.observed_at",
                    )
                    .bind(project.as_str())
                    .bind(generation)
                    .bind(i64::from(incomplete))
                    .bind(&observed_at)
                    .execute(&mut *conn)
                    .await?;
                    Ok(())
                })
            })
            .await
    }

    /// In one read snapshot: the scan row, or `None` before the first listing,
    /// and at most `limit` rows at `prefix_key` or under it, after
    /// `after_key`, by key. "Under" is the byte range `key/` up to `key0`, as
    /// in `code_outline`. `language` is left for the caller.
    pub(crate) async fn inventory_page(
        &self,
        project: &ProjectId,
        prefix_key: &str,
        after_key: Option<&str>,
        limit: i64,
    ) -> Result<Option<StoredPage>, StorageError> {
        let mut snapshot = self.storage.reader().begin().await?;
        let scan: Option<(i64, i64, String)> = sqlx::query_as(
            "SELECT generation, incomplete, observed_at FROM code_inventory_scan
              WHERE project_id = ?",
        )
        .bind(project.as_str())
        .fetch_optional(&mut *snapshot)
        .await?;
        let Some((generation, incomplete, observed_at)) = scan else {
            snapshot.commit().await?;
            return Ok(None);
        };
        let rows: Vec<StoredRow> = sqlx::query_as(
            "SELECT i.path_key, i.path, i.kind, i.excluded_reason,
                    EXISTS (SELECT 1 FROM code_tag t
                             WHERE t.project_id = i.project_id AND t.path_key = i.path_key)
               FROM code_inventory i
              WHERE i.project_id = ?
                AND (? = '' OR i.path_key = ?
                     OR (i.path_key >= ? || '/' AND i.path_key < ? || '0'))
                AND (? IS NULL OR i.path_key > ?)
              ORDER BY i.path_key LIMIT ?",
        )
        .bind(project.as_str())
        .bind(prefix_key)
        .bind(prefix_key)
        .bind(prefix_key)
        .bind(prefix_key)
        .bind(after_key)
        .bind(after_key)
        .bind(limit)
        .fetch_all(&mut *snapshot)
        .await?;
        snapshot.commit().await?;
        let entries = rows
            .into_iter()
            .map(|(key, path, kind, excluded_reason, has_symbols)| {
                let kind = match kind.as_str() {
                    "directory" => InventoryKind::Directory,
                    "file" => InventoryKind::File,
                    _ => InventoryKind::Excluded,
                };
                let entry = InventoryEntry {
                    path,
                    kind,
                    language: None,
                    excluded_reason,
                    has_symbols,
                };
                (key, entry)
            })
            .collect();
        Ok(Some((generation, incomplete != 0, observed_at, entries)))
    }
}
