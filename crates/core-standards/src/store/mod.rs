//! Immutable project standards additions over the shared storage handle.

use super::{Standards, StandardsAdditions, StandardsAdditionsVersion};
use core_db::{Actor, CommandContext, JournalEntry, Storage, StorageError};
use core_projects::{ProjectId, Projects};
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
    pub async fn current_in(
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
    pub async fn load_version_in(
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
