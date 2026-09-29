//! One job: a project's Planner instructions (spec §13.8). Each save is a new
//! row with the next `number`; nothing is overwritten, and the current
//! instructions are the highest number, so no pointer can name another
//! project's row.

use sqlx::SqliteConnection;

use super::model::InstructionsVersion;
use crate::command::CommandContext;
use crate::events::{Actor, DurableEvent};
use crate::projects::ProjectId;
use crate::storage::{Storage, StorageError, append_event, classify, now, record_command};

type VersionRow = (String, i64, String, String);

impl Storage {
    /// Saves `body` as the project's next version. A replay answers the
    /// version it saved. Its `PlannerInstructionsSaved` event carries the
    /// number, never the body.
    pub async fn save_planner_instructions(
        &self,
        ctx: &CommandContext,
        project: &ProjectId,
        body: &str,
    ) -> Result<InstructionsVersion, StorageError> {
        let (ctx, project, body, ts) = (ctx.clone(), project.clone(), body.to_string(), now());
        self.write_txn(move |conn| {
            Box::pin(async move {
                if let Some(id) = classify(conn, &ctx, "Project", project.as_str()).await? {
                    return load_version(conn, &id).await;
                }
                let known: Option<i64> = sqlx::query_scalar("SELECT 1 FROM project WHERE id = ?")
                    .bind(project.as_str())
                    .fetch_optional(&mut *conn)
                    .await?;
                known.ok_or(StorageError::NotFound("project"))?;
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
                let event =
                    DurableEvent::new("PlannerInstructionsSaved", Actor::user(&ctx.principal_id))
                        .with_project(&project)
                        .with_payload(serde_json::json!({ "number": number }));
                append_event(conn, &event, &ts).await?;
                let (scope, key) = ("Project", project.as_str());
                record_command(conn, &ctx, scope, key, "PlannerInstructions", &id, &ts).await?;
                load_version(conn, &id).await
            })
        })
        .await
    }

    /// The project's highest-numbered version, or `None` before its first save.
    pub async fn current_planner_instructions(
        &self,
        project: &ProjectId,
    ) -> Result<Option<InstructionsVersion>, StorageError> {
        let row: Option<VersionRow> = sqlx::query_as(
            "SELECT id, number, body, created_at FROM planner_instructions_version
              WHERE project_id = ? ORDER BY number DESC LIMIT 1",
        )
        .bind(project.as_str())
        .fetch_optional(self.reader())
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
