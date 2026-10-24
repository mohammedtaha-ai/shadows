//! Invocation metadata read in a caller's snapshot.
use super::HarnessStore;
use crate::{InvocationUsage, InvocationVersions, InvocationView};
use core_db::StorageError;
use sqlx::SqliteConnection;
use std::collections::HashMap;
type VersionsRow = (Option<String>, Option<String>, Option<i64>, Option<String>);

type InvocationRow = (
    String,
    String,
    String,
    String,
    String,
    String,
    Option<String>,
    Option<String>,
    Option<i64>,
    Option<i64>,
);

fn into_view(r: InvocationRow) -> InvocationView {
    InvocationView {
        harness_kind: r.1,
        harness_version: r.2,
        agent_version: r.3,
        requested_model: r.4,
        requested_mode: r.5,
        requested_effort: r.6,
        observed_model: r.7,
        context_used: r.8,
        context_window: r.9,
    }
}

impl HarnessStore {
    pub(crate) async fn invocation_in(
        conn: &mut SqliteConnection,
        op: &str,
    ) -> Result<Option<InvocationView>, StorageError> {
        let row: Option<InvocationRow> = sqlx::query_as(
            "SELECT operation_id, harness_kind, harness_version, agent_version,
                requested_model, requested_mode, requested_effort,
                observed_model, context_used, context_window
           FROM agent_invocation WHERE operation_id = ?",
        )
        .bind(op)
        .fetch_optional(&mut *conn)
        .await?;
        Ok(row.map(into_view))
    }

    pub(crate) async fn invocations_in(
        conn: &mut SqliteConnection,
        ids: &[&str],
    ) -> Result<HashMap<String, InvocationView>, StorageError> {
        let mut invocations = HashMap::new();
        // Bounded batches stay below SQLite's parameter limit without dynamic SQL.
        for batch in ids.chunks(64) {
            let mut query = sqlx::query_as::<_, InvocationRow>(
                "SELECT operation_id, harness_kind, harness_version, agent_version,
                    requested_model, requested_mode, requested_effort,
                    observed_model, context_used, context_window
               FROM agent_invocation WHERE operation_id IN (
                    ?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,
                    ?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,
                    ?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,
                    ?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)",
            );
            for slot in 0..64 {
                query = query.bind(batch.get(slot).copied());
            }
            for row in query.fetch_all(&mut *conn).await? {
                invocations.insert(row.0.clone(), into_view(row));
            }
        }
        Ok(invocations)
    }
    pub(crate) async fn invocation_versions_in(
        conn: &mut SqliteConnection,
        op: &str,
    ) -> Result<Option<InvocationVersions>, StorageError> {
        let row: Option<VersionsRow> = sqlx::query_as(
            "SELECT prompt_version AS prompt, planner_instructions_version_id AS instructions,
                standards_version AS standards, standards_additions_version_id AS additions
             FROM agent_invocation WHERE operation_id=?",
        )
        .bind(op)
        .fetch_optional(conn)
        .await?;
        Ok(row.map(
            |(prompt, instructions, standards, additions)| InvocationVersions {
                prompt,
                instructions,
                standards,
                additions,
            },
        ))
    }
    pub(crate) async fn invocation_model_in(
        conn: &mut SqliteConnection,
        op: &str,
    ) -> Result<Option<(String, String)>, StorageError> {
        Ok(sqlx::query_as(
            "SELECT COALESCE(observed_model, requested_model), harness_kind
             FROM agent_invocation WHERE operation_id=?",
        )
        .bind(op)
        .fetch_optional(conn)
        .await?)
    }
    pub(crate) async fn invocation_standards_in(
        conn: &mut SqliteConnection,
        op: &str,
    ) -> Result<(Option<i64>, Option<String>), StorageError> {
        sqlx::query_as(
            "SELECT standards_version, standards_additions_version_id
             FROM agent_invocation WHERE operation_id=?",
        )
        .bind(op)
        .fetch_optional(conn)
        .await?
        .ok_or(StorageError::NotFound("invocation"))
    }
    pub(crate) async fn invocation_usage_in(
        conn: &mut SqliteConnection,
        op: &str,
    ) -> Result<Option<InvocationUsage>, StorageError> {
        type UsageRow = (
            String,
            String,
            String,
            Option<String>,
            Option<String>,
            String,
        );
        let row: Option<UsageRow> = sqlx::query_as(
            "SELECT usage_agent_slug AS agent_slug, usage_provider_slug AS provider_slug,
                usage_provider_name AS provider_name, usage_label AS label,
                usage_preset AS preset, requested_model AS model
             FROM agent_invocation WHERE operation_id=? AND harness_kind='aylul'
                AND usage_provider_slug IS NOT NULL",
        )
        .bind(op)
        .fetch_optional(conn)
        .await?;
        Ok(row.map(
            |(agent_slug, provider_slug, provider_name, label, preset, model)| InvocationUsage {
                agent_slug,
                provider_slug,
                provider_name,
                label,
                preset,
                model,
            },
        ))
    }
}

impl HarnessStore {
    pub(crate) async fn pinned_standards_in(
        conn: &mut SqliteConnection,
        op: &str,
    ) -> Result<(i64, Option<String>), StorageError> {
        let row = sqlx::query_as(
            "SELECT standards_version,standards_additions_version_id FROM agent_invocation
         WHERE operation_id=?",
        )
        .bind(op)
        .fetch_one(&mut *conn)
        .await?;

        Ok(row)
    }
}
