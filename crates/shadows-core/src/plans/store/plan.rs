//! One job: a plan's state lifecycle with its versions (spec §16.2, §16.10).

use sqlx::SqliteConnection;

use super::read::{parse_plan_state, parse_state, written_by};
use crate::command::CommandContext;
use crate::db::{Storage, StorageError, append_event, classify, now, record_command};
use crate::events::{Actor, DurableEvent};
use crate::plans::model::{PlanId, PlanState, PlanVersions, VersionLine, WorkflowId};
use crate::projects::ProjectId;

/// Design's association check, inside the caller's serialized transaction.
/// Archived plans remain valid identities; their versions are not touched.
pub(crate) async fn check_design_plan(
    conn: &mut SqliteConnection,
    project: &ProjectId,
    plan: &PlanId,
) -> Result<(), StorageError> {
    let exists: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM plan WHERE project_id=? AND id=?)")
            .bind(project.as_str())
            .bind(plan.as_str())
            .fetch_one(conn)
            .await?;
    if exists {
        Ok(())
    } else {
        Err(StorageError::Constraint(format!(
            "plan {plan} does not belong to project {project}"
        )))
    }
}

impl Storage {
    /// One plan with every version, oldest first (§16.10's GET /api/plans/{id}).
    pub async fn get_plan_versions(&self, plan: &PlanId) -> Result<PlanVersions, StorageError> {
        let mut conn = self.reader().acquire().await?;
        load_plan_versions(&mut conn, plan).await
    }

    /// Archives `plan` (§16.2): state becomes `Archived`, `archived_at` is set.
    /// Archiving an already archived plan answers it unchanged.
    pub async fn archive_plan(
        &self,
        ctx: &CommandContext,
        plan: &PlanId,
    ) -> Result<PlanVersions, StorageError> {
        let (ctx, plan_id, ts) = (ctx.clone(), plan.clone(), now());
        self.write_txn(move |conn| {
            Box::pin(async move {
                if classify(conn, &ctx, "Plan", plan_id.as_str())
                    .await?
                    .is_some()
                {
                    return load_plan_versions(conn, &plan_id).await;
                }
                let current = load_plan_versions(conn, &plan_id).await?;
                if current.state != PlanState::Archived {
                    sqlx::query("UPDATE plan SET state = 'Archived', archived_at = ? WHERE id = ?")
                        .bind(&ts)
                        .bind(plan_id.as_str())
                        .execute(&mut *conn)
                        .await?;
                    let actor = Actor::user(&ctx.principal_id);
                    append_event(
                        conn,
                        &DurableEvent::new("PlanArchived", actor)
                            .with_project(&current.project_id)
                            .with_payload(serde_json::json!({ "plan": plan_id })),
                        &ts,
                    )
                    .await?;
                    super::notifications::notify_plan(
                        conn,
                        &current.project_id,
                        &plan_id,
                        &[],
                        &Actor::user(&ctx.principal_id),
                        &ts,
                    )
                    .await?;
                }
                record_command(
                    conn,
                    &ctx,
                    "Plan",
                    plan_id.as_str(),
                    "Plan",
                    plan_id.as_str(),
                    &ts,
                )
                .await?;
                load_plan_versions(conn, &plan_id).await
            })
        })
        .await
    }

    /// Unarchives `plan` (§16.2): state returns to `Active`, `archived_at` is cleared.
    /// Unarchiving an already active plan answers it unchanged.
    pub async fn unarchive_plan(
        &self,
        ctx: &CommandContext,
        plan: &PlanId,
    ) -> Result<PlanVersions, StorageError> {
        let (ctx, plan_id, ts) = (ctx.clone(), plan.clone(), now());
        self.write_txn(move |conn| {
            Box::pin(async move {
                if classify(conn, &ctx, "Plan", plan_id.as_str())
                    .await?
                    .is_some()
                {
                    return load_plan_versions(conn, &plan_id).await;
                }
                let current = load_plan_versions(conn, &plan_id).await?;
                if current.state != PlanState::Active {
                    sqlx::query(
                        "UPDATE plan SET state = 'Active', archived_at = NULL WHERE id = ?",
                    )
                    .bind(plan_id.as_str())
                    .execute(&mut *conn)
                    .await?;
                    let actor = Actor::user(&ctx.principal_id);
                    append_event(
                        conn,
                        &DurableEvent::new("PlanUnarchived", actor)
                            .with_project(&current.project_id)
                            .with_payload(serde_json::json!({ "plan": plan_id })),
                        &ts,
                    )
                    .await?;
                    super::notifications::notify_plan(
                        conn,
                        &current.project_id,
                        &plan_id,
                        &[],
                        &Actor::user(&ctx.principal_id),
                        &ts,
                    )
                    .await?;
                }
                record_command(
                    conn,
                    &ctx,
                    "Plan",
                    plan_id.as_str(),
                    "Plan",
                    plan_id.as_str(),
                    &ts,
                )
                .await?;
                load_plan_versions(conn, &plan_id).await
            })
        })
        .await
    }
}

/// Reads the plan and each of its versions in order, oldest first.
/// A plan of a removed project is not found (§16.2).
pub(super) async fn load_plan_versions(
    conn: &mut SqliteConnection,
    plan_id: &PlanId,
) -> Result<PlanVersions, StorageError> {
    let (project, state, archived_at): (String, String, Option<String>) = sqlx::query_as(
        "SELECT p.project_id, p.state, p.archived_at
           FROM plan p
           JOIN project pr ON pr.id = p.project_id
          WHERE p.id = ? AND pr.removed_at IS NULL",
    )
    .bind(plan_id.as_str())
    .fetch_optional(&mut *conn)
    .await?
    .ok_or(StorageError::NotFound("plan"))?;

    type VersionRow = (
        String,
        i64,
        String,
        String,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
        String,
    );

    let rows: Vec<VersionRow> = sqlx::query_as(
        "SELECT id, version, state, title, change_reason,
                written_by_thread, written_by_operation, written_by_grant, created_at
           FROM workflow
          WHERE plan_id = ?
          ORDER BY version ASC",
    )
    .bind(plan_id.as_str())
    .fetch_all(&mut *conn)
    .await?;

    let mut versions = Vec::with_capacity(rows.len());
    for (id, version, state, title, change_reason, thread, operation, grant, created_at) in rows {
        let workflow_id = WorkflowId::from_stored(id);
        let written_by = written_by(conn, &workflow_id, (thread, operation, grant)).await?;
        versions.push(VersionLine {
            workflow_id,
            version,
            state: parse_state(&state)?,
            title,
            written_by,
            change_reason,
            created_at,
        });
    }

    Ok(PlanVersions {
        plan_id: plan_id.clone(),
        project_id: ProjectId::from_stored(project),
        state: parse_plan_state(&state)?,
        archived_at,
        versions,
    })
}

/// Every new version write rechecks the plan's state in its own transaction.
/// A service's earlier scope check cannot decide the state after an archive.
pub(super) async fn require_active(
    conn: &mut SqliteConnection,
    plan: &PlanId,
) -> Result<(), StorageError> {
    let state: String = sqlx::query_scalar("SELECT state FROM plan WHERE id = ?")
        .bind(plan.as_str())
        .fetch_optional(&mut *conn)
        .await?
        .ok_or(StorageError::NotFound("plan"))?;
    if state == "Archived" {
        return Err(StorageError::PlanArchived(plan.clone()));
    }
    Ok(())
}
