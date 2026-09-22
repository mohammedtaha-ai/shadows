use sqlx::SqliteConnection;

use super::{Storage, StorageError, events::append_event, now};
use crate::command::CommandContext;
use crate::events::{Actor, DurableEvent};
use crate::project::{Project, ProjectDirectory, ProjectId};

/// `Some(outcome_ref)` when this exact command was already recorded, `None`
/// when it is new, `Err(CommandConflict)` when the id was reused with a
/// different request. Spec section 5.2.
///
/// Spec section 6.19 fixes the comparison at three fields: command kind,
/// schema version, and fingerprint. The schema version is not decoration — it
/// says which normalisation rules produced the fingerprint, so equal
/// fingerprints under different versions do not prove the requests are the
/// same one. Dropping it from this comparison would replay an outcome computed
/// under rules that no longer apply, and nothing would fail.
pub(super) async fn classify(
    conn: &mut SqliteConnection,
    ctx: &CommandContext,
    scope_kind: &str,
    scope_key: &str,
) -> Result<Option<String>, StorageError> {
    let existing: Option<(String, i64, String, Option<String>)> = sqlx::query_as(
        "SELECT command_kind, command_schema_ver, request_fingerprint, outcome_ref
           FROM command_record
          WHERE principal_kind = ? AND principal_id = ?
            AND command_scope_kind = ? AND command_scope_key = ?
            AND command_id = ?",
    )
    .bind(&ctx.principal_kind)
    .bind(&ctx.principal_id)
    .bind(scope_kind)
    .bind(scope_key)
    .bind(&ctx.command_id)
    .fetch_optional(&mut *conn)
    .await?;

    match existing {
        None => Ok(None),
        Some((kind, schema_ver, fp, outcome_ref)) => {
            if kind == ctx.command_kind
                && schema_ver == ctx.command_schema_ver
                && fp == ctx.request_fingerprint
            {
                Ok(Some(
                    outcome_ref.ok_or(StorageError::NotFound("outcome_ref"))?,
                ))
            } else {
                Err(StorageError::CommandConflict)
            }
        }
    }
}

pub(super) async fn record_command(
    conn: &mut SqliteConnection,
    ctx: &CommandContext,
    scope_kind: &str,
    scope_key: &str,
    entity_kind: &str,
    outcome_ref: &str,
    ts: &str,
) -> Result<(), StorageError> {
    sqlx::query(
        "INSERT INTO command_record
           (principal_kind, principal_id, command_scope_kind, command_scope_key,
            command_id, command_kind, command_schema_ver, request_fingerprint,
            outcome_kind, entity_kind, outcome_ref, recorded_at)
         VALUES (?,?,?,?,?,?,?,?,'Entity',?,?,?)",
    )
    .bind(&ctx.principal_kind)
    .bind(&ctx.principal_id)
    .bind(scope_kind)
    .bind(scope_key)
    .bind(&ctx.command_id)
    .bind(&ctx.command_kind)
    .bind(ctx.command_schema_ver)
    .bind(&ctx.request_fingerprint)
    .bind(entity_kind)
    .bind(outcome_ref)
    .bind(ts)
    .execute(&mut *conn)
    .await?;
    Ok(())
}

impl Storage {
    pub async fn create_project(
        &self,
        ctx: &CommandContext,
        slug: &str,
        name: &str,
        directory: &ProjectDirectory,
    ) -> Result<Project, StorageError> {
        let (ctx, slug, name, directory, ts) = (
            ctx.clone(),
            slug.to_string(),
            name.to_string(),
            directory.as_str().to_string(),
            now(),
        );
        self.write_txn(move |conn| {
            Box::pin(async move {
                if let Some(existing_id) = classify(conn, &ctx, "Global", "").await? {
                    return load_project(conn, &ProjectId::from_stored(existing_id)).await;
                }
                let id = ProjectId::generate();
                sqlx::query(
                    "INSERT INTO project (id, slug, name, directory, created_at) VALUES (?,?,?,?,?)",
                )
                .bind(id.as_str())
                .bind(&slug)
                .bind(&name)
                .bind(&directory)
                .bind(&ts)
                .execute(&mut *conn)
                .await?;

                append_event(
                    conn,
                    &DurableEvent::new("ProjectCreated", Actor::user(&ctx.principal_id))
                        .with_project(&id)
                        .with_payload(serde_json::json!({
                            "slug": slug, "name": name, "directory": directory,
                        })),
                    &ts,
                )
                .await?;

                record_command(conn, &ctx, "Global", "", "Project", id.as_str(), &ts).await?;
                load_project(conn, &id).await
            })
        })
        .await
    }

    pub async fn list_projects(&self) -> Result<Vec<Project>, StorageError> {
        let rows: Vec<ProjectRow> = sqlx::query_as(
            "SELECT id, slug, name, directory, created_at FROM project ORDER BY created_at, id",
        )
        .fetch_all(self.reader())
        .await?;
        Ok(rows.into_iter().map(project).collect())
    }
}

type ProjectRow = (String, String, String, Option<String>, String);

fn project((id, slug, name, directory, created_at): ProjectRow) -> Project {
    Project {
        id: ProjectId::from_stored(id),
        slug,
        name,
        directory,
        created_at,
    }
}

async fn load_project(
    conn: &mut SqliteConnection,
    id: &ProjectId,
) -> Result<Project, StorageError> {
    let row: ProjectRow =
        sqlx::query_as("SELECT id, slug, name, directory, created_at FROM project WHERE id = ?")
            .bind(id.as_str())
            .fetch_optional(&mut *conn)
            .await?
            .ok_or(StorageError::NotFound("project"))?;
    Ok(project(row))
}
