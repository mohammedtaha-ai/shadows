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

/// Rows one write transaction of a listing carries at most.
const BATCH_ROWS: usize = 5_000;
/// Rows one `INSERT` carries: five bound values each, under SQLite's limit.
const STATEMENT_ROWS: usize = 500;

impl CodeStore {
    /// The listing's rows are inserted or updated; the keys it lacks are
    /// deleted only when it is complete. The stored list is read outside any
    /// write, and the changes are written in batches of `BATCH_ROWS`, each its
    /// own transaction, so Shadows' one write lock is never held for a whole
    /// big listing. The scan row is written last: before a first listing ends
    /// there is none, so readers see it as still being made. A later listing
    /// that changes rows moves the generation by one in its first batch, with
    /// `incomplete` set until its last write, so a cursor from before it is
    /// stale and a page read during it says the list may be partial.
    /// Generation 1 on the first write. Nothing more is written once the
    /// project is removed. Callers write one listing of a project at a time.
    pub(in super::super) async fn write_inventory(
        &self,
        project: &ProjectId,
        rows: Vec<InventoryRow>,
        incomplete: bool,
        observed_at: &str,
    ) -> Result<(), StorageError> {
        let stored: Vec<(String, String, String, Option<String>)> = sqlx::query_as(
            "SELECT path_key, path, kind, excluded_reason FROM code_inventory
              WHERE project_id = ? ORDER BY path_key",
        )
        .bind(project.as_str())
        .fetch_all(self.storage.reader())
        .await?;
        let mut stored: HashMap<String, (String, String, Option<String>)> = stored
            .into_iter()
            .map(|(key, path, kind, reason)| (key, (path, kind, reason)))
            .collect();
        let mut upserts = Vec::new();
        for row in rows {
            let now = (
                row.path.clone(),
                row.kind.to_string(),
                row.excluded_reason.map(String::from),
            );
            if stored.remove(&row.path_key).as_ref() != Some(&now) {
                upserts.push(row);
            }
        }
        let mut gone: Vec<String> = if incomplete {
            Vec::new()
        } else {
            stored.into_keys().collect()
        };
        gone.sort_unstable();

        let mut first = true;
        for batch in upserts.chunks(BATCH_ROWS) {
            if !self
                .write_inventory_batch(project, batch, &[], first)
                .await?
            {
                return Ok(());
            }
            first = false;
        }
        for batch in gone.chunks(BATCH_ROWS) {
            if !self
                .write_inventory_batch(project, &[], batch, first)
                .await?
            {
                return Ok(());
            }
            first = false;
        }

        let (project, observed_at) = (project.clone(), observed_at.to_string());
        self.storage
            .write_txn(move |conn| {
                Box::pin(async move {
                    if !CodeStore::live_in(conn, &project).await? {
                        return Ok(());
                    }
                    let before: Option<i64> = sqlx::query_scalar(
                        "SELECT generation FROM code_inventory_scan WHERE project_id = ?",
                    )
                    .bind(project.as_str())
                    .fetch_optional(&mut *conn)
                    .await?;
                    // A changing listing on a stored list moved it in its first batch.
                    let generation = before.unwrap_or(1);
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

    /// One batch of a listing's changes in its own write. The first batch of
    /// a listing on a stored list moves the generation by one and marks the
    /// list incomplete. Answers false, writing nothing, once the project is
    /// removed.
    async fn write_inventory_batch(
        &self,
        project: &ProjectId,
        upserts: &[InventoryRow],
        gone: &[String],
        first: bool,
    ) -> Result<bool, StorageError> {
        let project = project.clone();
        let upserts = upserts.to_vec();
        let gone = gone.to_vec();
        self.storage
            .write_txn(move |conn| {
                Box::pin(async move {
                    if !CodeStore::live_in(conn, &project).await? {
                        return Ok(false);
                    }
                    if first {
                        sqlx::query(
                            "UPDATE code_inventory_scan
                                SET generation = generation + 1, incomplete = 1
                              WHERE project_id = ?",
                        )
                        .bind(project.as_str())
                        .execute(&mut *conn)
                        .await?;
                    }
                    // Several rows a statement, each under SQLite's bound-variable limit.
                    for rows in upserts.chunks(STATEMENT_ROWS) {
                        let mut insert = sqlx::QueryBuilder::new(
                            "INSERT OR REPLACE INTO code_inventory
                               (project_id, path_key, path, kind, excluded_reason) ",
                        );
                        insert.push_values(rows, |mut row, r| {
                            row.push_bind(project.as_str())
                                .push_bind(&r.path_key)
                                .push_bind(&r.path)
                                .push_bind(r.kind)
                                .push_bind(r.excluded_reason);
                        });
                        insert.build().execute(&mut *conn).await?;
                    }
                    for key in &gone {
                        sqlx::query(
                            "DELETE FROM code_inventory WHERE project_id = ? AND path_key = ?",
                        )
                        .bind(project.as_str())
                        .bind(key)
                        .execute(&mut *conn)
                        .await?;
                    }
                    Ok(true)
                })
            })
            .await
    }

    /// Whether the project is still live; a listing that ends after its
    /// project was removed writes nothing.
    async fn live_in(conn: &mut core_db::Tx, project: &ProjectId) -> Result<bool, StorageError> {
        match Projects::require_live_in(conn, project).await {
            Ok(()) => Ok(true),
            Err(StorageError::NotFound(_)) => Ok(false),
            Err(error) => Err(error),
        }
    }

    /// In one read snapshot: the scan row, or `None` before the first listing,
    /// and at most `limit` rows at `prefix_key` or under it, after
    /// `after_key`, by key. "Under" is the byte range `key/` up to `key0`, as
    /// in `code_outline`. `language` is left for the caller.
    pub(in super::super) async fn inventory_page(
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
