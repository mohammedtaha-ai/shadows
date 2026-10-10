//! Versioned project additions to compiled-in standards (§23.2).

use crate::command::CommandContext;
use crate::db::{Storage, StorageError, append_event, classify, now, record_command};
use crate::design::{StandardsAdditions, StandardsAdditionsVersion, standards};
use crate::events::{Actor, DurableEvent};
use crate::projects::ProjectId;
use crate::turns::OperationId;
use sqlx::SqliteConnection;

type VersionRow = (String, i64, String, String);

impl Storage {
    /// The immutable additions selected at invocation start, with project ownership checked.
    pub async fn standards_additions_for_invocation(
        &self,
        operation: &OperationId,
    ) -> Result<Option<StandardsAdditionsVersion>, StorageError> {
        type PinnedRow = (
            Option<String>,
            Option<String>,
            Option<i64>,
            Option<String>,
            Option<String>,
        );
        let row: Option<PinnedRow> = sqlx::query_as(
            "SELECT i.standards_additions_version_id,v.id,v.number,v.content_json,v.created_at
             FROM operation o JOIN agent_invocation i ON i.operation_id=o.id
             JOIN planning_thread t ON t.id=o.thread_id
             JOIN project p ON p.id=t.project_id AND p.removed_at IS NULL
             LEFT JOIN standards_additions_version v
               ON v.id=i.standards_additions_version_id AND v.project_id=p.id
             WHERE o.id=?",
        )
        .bind(operation.as_str())
        .fetch_optional(self.reader())
        .await?;
        let (selected, id, number, content, created_at) =
            row.ok_or(StorageError::NotFound("invocation"))?;
        if selected.is_none() {
            return Ok(None);
        }
        let missing = || StorageError::NotFound("invocation standards additions");
        Ok(Some(into_version((
            id.ok_or_else(missing)?,
            number.ok_or_else(missing)?,
            content.ok_or_else(missing)?,
            created_at.ok_or_else(missing)?,
        ))?))
    }

    pub async fn save_standards_additions(
        &self,
        ctx: &CommandContext,
        project: &ProjectId,
        content: &StandardsAdditions,
    ) -> Result<StandardsAdditionsVersion, StorageError> {
        let (ctx, project, content, ts) = (ctx.clone(), project.clone(), content.clone(), now());
        self.write_txn(move |conn| {
            Box::pin(async move {
                if let Some(id) = classify(conn, &ctx, "Project", project.as_str()).await? {
                    return load_version(conn, &id).await;
                }
                standards::validate(&content).map_err(StorageError::Constraint)?;
                let known: Option<i64> =
                    sqlx::query_scalar("SELECT 1 FROM project WHERE id=? AND removed_at IS NULL")
                        .bind(project.as_str())
                        .fetch_optional(&mut *conn)
                        .await?;
                known.ok_or(StorageError::NotFound("project"))?;
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
                let event =
                    DurableEvent::new("ProjectStandardsSaved", Actor::user(&ctx.principal_id))
                        .with_project(&project)
                        .with_payload(serde_json::json!({"number": number}));
                append_event(conn, &event, &ts).await?;
                record_command(
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

    /// None means a live project has never saved additions. Missing projects refuse.
    pub async fn current_standards_additions(
        &self,
        project: &ProjectId,
    ) -> Result<Option<StandardsAdditionsVersion>, StorageError> {
        let mut tx = self.reader().begin().await?;
        let known: Option<i64> =
            sqlx::query_scalar("SELECT 1 FROM project WHERE id=? AND removed_at IS NULL")
                .bind(project.as_str())
                .fetch_optional(&mut *tx)
                .await?;
        known.ok_or(StorageError::NotFound("project"))?;
        let version = current_in(&mut tx, project).await?;
        tx.commit().await?;
        Ok(version)
    }
}

/// Reads additions within a caller's captured workspace snapshot.
pub(super) async fn current_in(
    conn: &mut SqliteConnection,
    project: &ProjectId,
) -> Result<Option<StandardsAdditionsVersion>, StorageError> {
    let row: Option<VersionRow> = sqlx::query_as(
        "SELECT id,number,content_json,created_at FROM standards_additions_version
         WHERE project_id=? ORDER BY number DESC LIMIT 1",
    )
    .bind(project.as_str())
    .fetch_optional(conn)
    .await?;
    row.map(into_version).transpose()
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
