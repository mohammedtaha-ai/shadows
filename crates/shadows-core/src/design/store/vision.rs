//! Atomic vision writes with immutable command results.

use crate::command::CommandContext;
use crate::db::{Storage, StorageError, append_event, classify, now, record_command};
use crate::design::{DesignChange, DesignOp, VisionContent, VisionView};
use crate::events::{Actor, DurableEvent};
use crate::projects::ProjectId;

impl Storage {
    pub async fn design_vision(&self, project: &ProjectId) -> Result<VisionView, StorageError> {
        let row: Option<(i64, Option<String>)> = sqlx::query_as(
            "SELECT COALESCE(w.revision, 0), w.vision_content FROM project p
             LEFT JOIN design_workspace w ON w.project_id = p.id
             WHERE p.id = ? AND p.removed_at IS NULL",
        )
        .bind(project.as_str())
        .fetch_optional(self.reader())
        .await?;
        let (revision, content) = row.ok_or(StorageError::NotFound("project"))?;
        Ok(VisionView {
            revision,
            content: content
                .map(|s| serde_json::from_str(&s))
                .transpose()?
                .unwrap_or_default(),
        })
    }

    pub async fn edit_design(
        &self,
        ctx: &CommandContext,
        project: &ProjectId,
        expected_revision: i64,
        ops: Vec<DesignOp>,
    ) -> Result<DesignChange, StorageError> {
        let (ctx, project, ts) = (ctx.clone(), project.clone(), now());
        self.write_txn(move |conn| {
            Box::pin(async move {
                if let Some(id) = classify(conn, &ctx, "Project", project.as_str()).await? {
                    let result: String = sqlx::query_scalar(
                        "SELECT result_json FROM design_command_result
                         WHERE id = ? AND project_id = ?",
                    )
                    .bind(id)
                    .bind(project.as_str())
                    .fetch_optional(&mut *conn)
                    .await?
                    .ok_or(StorageError::NotFound("design command result"))?;
                    return Ok(serde_json::from_str(&result)?);
                }
                let revision: Option<i64> = sqlx::query_scalar(
                    "SELECT COALESCE(w.revision, 0) FROM project p
                     LEFT JOIN design_workspace w ON w.project_id = p.id
                     WHERE p.id = ? AND p.removed_at IS NULL",
                )
                .bind(project.as_str())
                .fetch_optional(&mut *conn)
                .await?;
                let current = revision.ok_or(StorageError::NotFound("project"))?;
                if current != expected_revision {
                    return Err(StorageError::RevisionConflict {
                        current,
                        summary: "project design changed".into(),
                    });
                }
                // An ordered batch keeps the last complete put. Empty batches
                // are refused before any workspace is initialized.
                let content: VisionContent = ops
                    .into_iter()
                    .map(|op| match op { DesignOp::VisionPut { content } => content })
                    .next_back()
                    .ok_or_else(|| StorageError::Constraint("a design edit needs an operation".into()))?;
                let next = current.checked_add(1).ok_or_else(|| {
                    StorageError::Unavailable("design revision exhausted".into())
                })?;
                sqlx::query(
                    "INSERT INTO design_workspace
                       (project_id, revision, vision_revision, vision_content) VALUES (?,?,?,?)
                     ON CONFLICT(project_id) DO UPDATE SET revision = excluded.revision,
                       vision_revision = design_workspace.vision_revision + 1,
                       vision_content = excluded.vision_content",
                )
                .bind(project.as_str())
                .bind(next)
                .bind(1_i64)
                .bind(serde_json::to_string(&content)?)
                .execute(&mut *conn)
                .await?;
                let change = DesignChange { revision: next };
                let id = uuid::Uuid::new_v4().to_string();
                sqlx::query(
                    "INSERT INTO design_command_result (id, project_id, result_json) VALUES (?,?,?)",
                )
                .bind(&id)
                .bind(project.as_str())
                .bind(serde_json::to_string(&change)?)
                .execute(&mut *conn)
                .await?;
                let event = DurableEvent::new("ProjectDesignChanged", Actor::user(&ctx.principal_id))
                    .with_project(&project)
                    .with_payload(serde_json::json!({
                        "project_id": project, "revision": next,
                        "changed_parts": [], "changed_outcomes": [], "vision_changed": true
                    }));
                append_event(conn, &event, &ts).await?;
                record_command(conn, &ctx, "Project", project.as_str(), "DesignChange", &id, &ts).await?;
                Ok(change)
            })
        })
        .await
    }
}
