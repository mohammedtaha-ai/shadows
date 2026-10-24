//! One job: a project's Planner instructions rows. Each save is a new row with
//! the next `number`; nothing is overwritten, and the current instructions are
//! the highest number, so no pointer can name another project's row.

use std::sync::Arc;

use sqlx::SqliteConnection;

use super::model::InstructionsVersion;
use crate::projects::{ProjectId, Projects};
use core_db::{Actor, CommandContext, JournalEntry, Storage, StorageError};

type VersionRow = (String, i64, String, String);

/// The service's SQL, over the shared storage handle.
#[derive(Clone)]
pub(super) struct InstructionsStore {
    storage: Arc<Storage>,
}

impl InstructionsStore {
    pub(super) fn new(storage: Arc<Storage>) -> Self {
        Self { storage }
    }

    /// Saves `body` as the project's next version. A replay answers the
    /// version it saved. Its `PlannerInstructionsSaved` event carries the
    /// number, never the body.
    pub(super) async fn save(
        &self,
        ctx: &CommandContext,
        project: &ProjectId,
        body: &str,
    ) -> Result<InstructionsVersion, StorageError> {
        let (ctx, project, body, ts) = (
            ctx.clone(),
            project.clone(),
            body.to_string(),
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
                    Projects::require_live_in(conn, &project).await?;
                    let number: i64 = sqlx::query_scalar(
                        "SELECT COALESCE(MAX(number), 0) + 1 FROM planner_instructions_version
                          WHERE project_id = ?",
                    )
                    .bind(project.as_str())
                    .fetch_one(&mut *conn)
                    .await?;
                    let id = uuid::Uuid::new_v4().to_string();
                    sqlx::query(
                        "INSERT INTO planner_instructions_version
                           (id, project_id, number, body, created_at)
                         VALUES (?,?,?,?,?)",
                    )
                    .bind(&id)
                    .bind(project.as_str())
                    .bind(number)
                    .bind(&body)
                    .bind(&ts)
                    .execute(&mut *conn)
                    .await?;
                    let event = JournalEntry {
                        event_id: uuid::Uuid::new_v4().to_string(),
                        kind: "PlannerInstructionsSaved".into(),
                        project_id: Some(project.as_str().to_owned()),
                        thread_id: None,
                        operation_id: None,
                        actor: Actor::user(&ctx.principal_id),
                        causation: None,
                        correlation_id: None,
                        payload_json: serde_json::json!({ "number": number }).to_string(),
                    };
                    JournalEntry::append_event(conn, event, &ts).await?;
                    let (scope, key) = ("Project", project.as_str());
                    CommandContext::record_command(
                        conn,
                        &ctx,
                        scope,
                        key,
                        "PlannerInstructions",
                        &id,
                        &ts,
                    )
                    .await?;
                    load_version(conn, &id).await
                })
            })
            .await
    }

    /// The project's highest-numbered version, or `None` before its first
    /// save, and for a removed project as for an unknown one.
    pub(super) async fn current(
        &self,
        project: &ProjectId,
    ) -> Result<Option<InstructionsVersion>, StorageError> {
        let mut conn = self.storage.reader().acquire().await?;
        match Projects::require_live_in(&mut conn, project).await {
            Ok(()) => {}
            Err(StorageError::NotFound(_)) => return Ok(None),
            Err(error) => return Err(error),
        }
        let row: Option<VersionRow> = sqlx::query_as(
            "SELECT id, number, body, created_at FROM planner_instructions_version
              WHERE project_id = ? ORDER BY number DESC LIMIT 1",
        )
        .bind(project.as_str())
        .fetch_optional(&mut *conn)
        .await?;
        Ok(row.map(into_version))
    }
}

fn into_version((id, number, body, created_at): VersionRow) -> InstructionsVersion {
    InstructionsVersion {
        id,
        number,
        body,
        created_at,
    }
}

async fn load_version(
    conn: &mut SqliteConnection,
    id: &str,
) -> Result<InstructionsVersion, StorageError> {
    let row: VersionRow = sqlx::query_as(
        "SELECT id, number, body, created_at FROM planner_instructions_version WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(&mut *conn)
    .await?
    .ok_or(StorageError::NotFound("planner instructions"))?;
    Ok(into_version(row))
}
