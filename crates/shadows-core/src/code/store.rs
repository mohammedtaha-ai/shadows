//! One job: the code index's SQLite queries (spec §15.4–§15.5). Every read
//! goes through the reader pool; every write is one short `write_txn` for one
//! file, so a first index never holds the single writer for long (§6.23).

use shadows_index::{Role, Tag};

use super::model::{Hit, Skipped};
use crate::db::{Storage, StorageError};
use crate::projects::ProjectId;

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

/// The row count past which an answer says `more`.
const HITS: i64 = 50;

type HitRow = (String, i64, String, String, Option<String>);

impl Storage {
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
        .fetch_optional(self.reader())
        .await?)
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
        .fetch_all(self.reader())
        .await?)
    }

    /// One write: the file's old tags go, its row is upserted, its new tags
    /// are inserted.
    pub(super) async fn write_code_file(
        &self,
        project: &ProjectId,
        file: FileRow,
        tags: Vec<Tag>,
    ) -> Result<(), StorageError> {
        let project = project.clone();
        self.write_txn(move |conn| {
            Box::pin(async move {
                sqlx::query("DELETE FROM code_tag WHERE project_id = ? AND path_key = ?")
                    .bind(project.as_str())
                    .bind(&file.path_key)
                    .execute(&mut *conn)
                    .await?;
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
                for tag in &tags {
                    let role = match tag.role {
                        Role::Definition => "definition",
                        Role::Reference => "reference",
                    };
                    sqlx::query(
                        "INSERT INTO code_tag
                           (project_id, path_key, path, name, kind, role, line, signature)
                         VALUES (?,?,?,?,?,?,?,?)",
                    )
                    .bind(project.as_str())
                    .bind(&file.path_key)
                    .bind(&file.path)
                    .bind(&tag.name)
                    .bind(&tag.kind)
                    .bind(role)
                    .bind(i64::from(tag.line))
                    .bind(&tag.signature)
                    .execute(&mut *conn)
                    .await?;
                }
                Ok(())
            })
        })
        .await
    }

    /// One write: a file's tags and its row go.
    pub(super) async fn delete_code_file(
        &self,
        project: &ProjectId,
        path_key: &str,
    ) -> Result<(), StorageError> {
        let (project, key) = (project.clone(), path_key.to_string());
        self.write_txn(move |conn| {
            Box::pin(async move {
                for sql in [
                    "DELETE FROM code_tag WHERE project_id = ? AND path_key = ?",
                    "DELETE FROM code_file WHERE project_id = ? AND path_key = ?",
                ] {
                    sqlx::query(sql)
                        .bind(project.as_str())
                        .bind(&key)
                        .execute(&mut *conn)
                        .await?;
                }
                Ok(())
            })
        })
        .await
    }

    /// The asker's project, then the projects it links to directly, by slug.
    pub(super) async fn code_scope(&self, home: &ProjectId) -> Result<Vec<ScopeRow>, StorageError> {
        let first: Option<(String, Option<String>)> =
            sqlx::query_as("SELECT slug, directory FROM project WHERE id = ?")
                .bind(home.as_str())
                .fetch_optional(self.reader())
                .await?;
        let (slug, directory) = first.ok_or(StorageError::NotFound("project"))?;
        let linked: Vec<(String, String, Option<String>)> = sqlx::query_as(
            "SELECT p.id, p.slug, p.directory FROM project_link l
               JOIN project p ON p.id = l.linked_project_id
              WHERE l.project_id = ? ORDER BY p.slug",
        )
        .bind(home.as_str())
        .fetch_all(self.reader())
        .await?;
        let mut out = vec![(home.clone(), slug, directory)];
        out.extend(
            linked
                .into_iter()
                .map(|(id, slug, dir)| (ProjectId::from_stored(id), slug, dir)),
        );
        Ok(out)
    }

    /// Tags named exactly `name` in `role`, in the scope's order, then by
    /// path, line and kind: at most 51, so a 51st says there are more.
    pub(super) async fn code_by_name(
        &self,
        scope: &[ScopeRow],
        name: &str,
        role: &str,
    ) -> Result<Vec<Hit>, StorageError> {
        let mut out = Vec::new();
        for (id, slug, _) in scope {
            let left = HITS + 1 - out.len() as i64;
            if left <= 0 {
                break;
            }
            let rows: Vec<HitRow> = sqlx::query_as(
                "SELECT path, line, kind, name, signature FROM code_tag
                  WHERE project_id = ? AND name = ? AND role = ?
                  ORDER BY path, line, kind LIMIT ?",
            )
            .bind(id.as_str())
            .bind(name)
            .bind(role)
            .bind(left)
            .fetch_all(self.reader())
            .await?;
            out.extend(rows.into_iter().map(|r| hit(slug, r)));
        }
        Ok(out)
    }

    /// Up to 10 distinct names in `role` containing `text`, case-insensitively
    /// (SQLite's `LIKE`), by name across the scope: each project's first 10,
    /// merged.
    pub(super) async fn code_suggestions(
        &self,
        scope: &[ScopeRow],
        text: &str,
        role: &str,
    ) -> Result<Vec<String>, StorageError> {
        let mut names = std::collections::BTreeSet::new();
        for (id, _, _) in scope {
            let found: Vec<String> = sqlx::query_scalar(
                "SELECT DISTINCT name FROM code_tag
                  WHERE project_id = ? AND role = ?
                    AND name LIKE '%' || ? || '%' ESCAPE '\\'
                  ORDER BY name LIMIT 10",
            )
            .bind(id.as_str())
            .bind(role)
            .bind(like_escape(text))
            .fetch_all(self.reader())
            .await?;
            names.extend(found);
        }
        Ok(names.into_iter().take(10).collect())
    }

    /// The definitions at `path_key` or under it, every one when it is
    /// empty; in the scope's order, then by path, line and name; at most 51.
    pub(super) async fn code_outline(
        &self,
        scope: &[ScopeRow],
        path_key: &str,
    ) -> Result<Vec<Hit>, StorageError> {
        let mut out = Vec::new();
        for (id, slug, _) in scope {
            let left = HITS + 1 - out.len() as i64;
            if left <= 0 {
                break;
            }
            let rows: Vec<HitRow> = sqlx::query_as(
                "SELECT path, line, kind, name, signature FROM code_tag
                  WHERE project_id = ? AND role = 'definition'
                    AND (? = '' OR path_key = ? OR path_key LIKE ? || '/%' ESCAPE '\\')
                  ORDER BY path, line, name LIMIT ?",
            )
            .bind(id.as_str())
            .bind(path_key)
            .bind(path_key)
            .bind(like_escape(path_key))
            .bind(left)
            .fetch_all(self.reader())
            .await?;
            out.extend(rows.into_iter().map(|r| hit(slug, r)));
        }
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
        .fetch_one(self.reader())
        .await?;
        let skipped: Vec<(String, i64)> = sqlx::query_as(
            "SELECT skipped_reason, COUNT(*) FROM code_file
              WHERE project_id = ? AND skipped_reason IS NOT NULL
              GROUP BY skipped_reason ORDER BY skipped_reason",
        )
        .bind(project.as_str())
        .fetch_all(self.reader())
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

fn hit(slug: &str, (path, line, kind, name, signature): HitRow) -> Hit {
    Hit {
        project: slug.to_string(),
        path,
        line: line as u32,
        kind,
        name,
        signature,
        matched_by: None,
    }
}

/// `text` with `LIKE`'s `%`, `_` and the escape `\` escaped.
fn like_escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        if matches!(c, '%' | '_' | '\\') {
            out.push('\\');
        }
        out.push(c);
    }
    out
}
