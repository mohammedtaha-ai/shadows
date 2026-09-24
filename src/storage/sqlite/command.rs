//! One job: the command record — whether a command already happened
//! (spec §5.2, §6.19), and recording that it did.

use sqlx::SqliteConnection;

use super::StorageError;
use crate::command::CommandContext;

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
pub(in crate::storage) async fn classify(
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

pub(in crate::storage) async fn record_command(
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
