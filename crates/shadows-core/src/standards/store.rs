//! Immutable project standards additions over the shared storage handle.

use super::{Standards, StandardsAdditions, StandardsAdditionsVersion};
use crate::projects::{ProjectId, Projects};
use core_db::{Actor, CommandContext, JournalEntry, Storage, StorageError};
use sqlx::SqliteConnection;
use std::sync::Arc;

type VersionRow = (String, i64, String, String);

#[derive(Clone)]
pub(super) struct StandardsStore {
    storage: Arc<Storage>,
}

impl StandardsStore {
    pub(super) fn new(storage: Arc<Storage>) -> Self {
        Self { storage }
    }
    pub(super) async fn save_standards_additions(
        &self,
        ctx: &CommandContext,
        project: &ProjectId,
        content: &StandardsAdditions,
    ) -> Result<StandardsAdditionsVersion, StorageError> {
        let (ctx, project, content, ts) = (
            ctx.clone(),
            project.clone(),
            content.clone(),
            Storage::now(),
        );
        self.storage
            .write_txn(move |conn| {
                Box::pin(async move {
                    if let Some(id) =
                        CommandContext::classify(conn, &ctx, "Project", project.as_str()).await?
                    {
                        return load_version(conn, &id).await;
                    }
                    Standards::validate(&content).map_err(StorageError::Constraint)?;
                    Projects::require_live_in(conn, &project).await?;
                    let last: i64 = sqlx::query_scalar(
                        "SELECT COALESCE(MAX(number),0) FROM standards_additions_version
                 WHERE project_id=?",
                    )
                    .bind(project.as_str())
                    .fetch_one(&mut *conn)
                    .await?;
                    let number = last.checked_add(1).ok_or_else(|| {
                        StorageError::Unavailable("standards version exhausted".into())
                    })?;
                    let id = uuid::Uuid::new_v4().to_string();
                    sqlx::query(
                        "INSERT INTO standards_additions_version
                   (id,project_id,number,content_json,created_at) VALUES (?,?,?,?,?)",
                    )
                    .bind(&id)
                    .bind(project.as_str())
                    .bind(number)
                    .bind(serde_json::to_string(&content)?)
                    .bind(&ts)
                    .execute(&mut *conn)
                    .await?;
                    let event = JournalEntry {
                        event_id: uuid::Uuid::new_v4().to_string(),
                        kind: "ProjectStandardsSaved".into(),
                        project_id: Some(project.as_str().to_owned()),
                        thread_id: None,
                        operation_id: None,
                        actor: Actor::user(&ctx.principal_id),
                        causation: None,
                        correlation_id: None,
                        payload_json: serde_json::json!({"number": number}).to_string(),
                    };
                    JournalEntry::append_event(conn, event, &ts).await?;
                    CommandContext::record_command(
                        conn,
                        &ctx,
                        "Project",
                        project.as_str(),
                        "StandardsAdditions",
                        &id,
                        &ts,
                    )
                    .await?;
                    load_version(conn, &id).await
                })
            })
            .await
    }

    pub(super) async fn current(
        &self,
        project: &ProjectId,
    ) -> Result<Option<StandardsAdditionsVersion>, StorageError> {
        let mut tx = self.storage.reader().begin().await?;
        let version = Standards::current_in(&mut tx, project).await?;
        tx.commit().await?;
        Ok(version)
    }
}

impl Standards {
    /// Reads additions in the caller's existing project snapshot.
    pub(crate) async fn current_in(
        conn: &mut SqliteConnection,
        project: &ProjectId,
    ) -> Result<Option<StandardsAdditionsVersion>, StorageError> {
        Projects::require_live_in(conn, project).await?;
        let row: Option<VersionRow> = sqlx::query_as(
            "SELECT id,number,content_json,created_at FROM standards_additions_version
             WHERE project_id=? ORDER BY number DESC LIMIT 1",
        )
        .bind(project.as_str())
        .fetch_optional(conn)
        .await?;
        row.map(into_version).transpose()
    }

    /// Loads an immutable version belonging to the caller's project.
    pub(crate) async fn load_version_in(
        conn: &mut SqliteConnection,
        project: &ProjectId,
        id: &str,
    ) -> Result<StandardsAdditionsVersion, StorageError> {
        let row: VersionRow = sqlx::query_as(
            "SELECT id,number,content_json,created_at FROM standards_additions_version
             WHERE id=? AND project_id=?",
        )
        .bind(id)
        .bind(project.as_str())
        .fetch_optional(conn)
        .await?
        .ok_or(StorageError::NotFound("standards additions"))?;
        into_version(row)
    }
}

fn into_version(
    (id, number, content, created_at): VersionRow,
) -> Result<StandardsAdditionsVersion, StorageError> {
    Ok(StandardsAdditionsVersion {
        id,
        number,
        content: serde_json::from_str(&content)?,
        created_at,
    })
}

async fn load_version(
    conn: &mut SqliteConnection,
    id: &str,
) -> Result<StandardsAdditionsVersion, StorageError> {
    let row: VersionRow = sqlx::query_as(
        "SELECT id,number,content_json,created_at FROM standards_additions_version WHERE id=?",
    )
    .bind(id)
    .fetch_optional(conn)
    .await?
    .ok_or(StorageError::NotFound("standards additions"))?;
    into_version(row)
}

#[cfg(all(test, feature = "test-support"))]
mod tests {
    use super::Standards;
    use core_db::StorageError;

    #[tokio::test]
    async fn additions_reads_keep_the_snapshot_and_scope_versions_to_the_project() {
        let db = tempfile::tempdir().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let core = crate::testing::app(db.path().join("standards.sqlite3"))
            .await
            .unwrap();
        let project = core
            .project_lifecycle()
            .create(
                "project".into(),
                "project",
                "Project",
                dir.path().to_str().unwrap(),
            )
            .await
            .unwrap();
        let other = core
            .project_lifecycle()
            .create(
                "other".into(),
                "other",
                "Other",
                dir.path().to_str().unwrap(),
            )
            .await
            .unwrap();
        let standards = core.standards();
        let first = standards
            .save_standards_additions("first".into(), &project.id, Default::default())
            .await
            .unwrap();
        let mut snapshot = standards.store.storage.reader().begin().await.unwrap();
        assert_eq!(
            Standards::current_in(&mut snapshot, &project.id)
                .await
                .unwrap(),
            Some(first.clone())
        );
        standards
            .save_standards_additions("second".into(), &project.id, Default::default())
            .await
            .unwrap();
        core.project_lifecycle()
            .remove("remove".into(), &project.id)
            .await
            .unwrap();
        assert_eq!(
            Standards::current_in(&mut snapshot, &project.id)
                .await
                .unwrap(),
            Some(first.clone()),
            "liveness and additions must use the same snapshot"
        );
        assert_eq!(
            Standards::load_version_in(&mut snapshot, &project.id, &first.id)
                .await
                .unwrap(),
            first
        );
        assert!(matches!(
            Standards::load_version_in(&mut snapshot, &other.id, &first.id).await,
            Err(StorageError::NotFound("standards additions"))
        ));
        assert!(matches!(
            Standards::load_version_in(&mut snapshot, &other.id, "missing").await,
            Err(StorageError::NotFound("standards additions"))
        ));
        snapshot.commit().await.unwrap();
        assert!(matches!(
            standards.current(&project.id).await,
            Err(crate::CoreError::Storage(StorageError::NotFound("project")))
        ));
        assert_eq!(
            standards
                .save_standards_additions("first".into(), &project.id, Default::default())
                .await
                .unwrap(),
            first,
            "removal must not change an immutable replay result"
        );
        core.code().shut_down().await;
    }
}
