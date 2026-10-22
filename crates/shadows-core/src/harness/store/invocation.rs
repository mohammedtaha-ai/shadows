//! Invocation entry points for transaction-local callers.
use super::HarnessStore;
use crate::db::StorageError;
use crate::harness::{Harness, InvocationUsage, InvocationVersions, InvocationView, NewInvocation};
use sqlx::SqliteConnection;
use std::collections::HashMap;
impl Harness {
    pub(crate) async fn record_invocation_in(
        conn: &mut SqliteConnection,
        operation: &str,
        input: NewInvocation<'_>,
        ts: &str,
    ) -> Result<(), StorageError> {
        HarnessStore::record_invocation_in(conn, operation, input, ts).await
    }
    pub(crate) async fn observe_invocation_in(
        conn: &mut SqliteConnection,
        operation: &str,
        seen: &shadows_agent::events::TurnObservation,
    ) -> Result<(), StorageError> {
        HarnessStore::observe_invocation_in(conn, operation, seen).await
    }
    pub(crate) async fn invocation_in(
        conn: &mut SqliteConnection,
        op: &str,
    ) -> Result<Option<InvocationView>, StorageError> {
        HarnessStore::invocation_in(conn, op).await
    }
    pub(crate) async fn invocations_in(
        conn: &mut SqliteConnection,
        ids: &[&str],
    ) -> Result<HashMap<String, InvocationView>, StorageError> {
        HarnessStore::invocations_in(conn, ids).await
    }
    pub(crate) async fn invocation_versions_in(
        conn: &mut SqliteConnection,
        op: &str,
    ) -> Result<Option<InvocationVersions>, StorageError> {
        HarnessStore::invocation_versions_in(conn, op).await
    }
    pub(crate) async fn invocation_model_in(
        conn: &mut SqliteConnection,
        op: &str,
    ) -> Result<Option<(String, String)>, StorageError> {
        HarnessStore::invocation_model_in(conn, op).await
    }
    pub(crate) async fn invocation_standards_in(
        conn: &mut SqliteConnection,
        op: &str,
    ) -> Result<(Option<i64>, Option<String>), StorageError> {
        HarnessStore::invocation_standards_in(conn, op).await
    }
    pub(crate) async fn invocation_usage_in(
        conn: &mut SqliteConnection,
        op: &str,
    ) -> Result<Option<InvocationUsage>, StorageError> {
        HarnessStore::invocation_usage_in(conn, op).await
    }
    pub(crate) async fn remember_settings_in(
        conn: &mut SqliteConnection,
        kind: &str,
        profile: Option<&str>,
        settings: &shadows_agent::TurnSettings,
        ts: &str,
    ) -> Result<(), StorageError> {
        HarnessStore::remember_settings(
            conn,
            &crate::harness::preference_key(kind, profile),
            settings,
            ts,
        )
        .await
    }
}

impl Harness {
    pub(crate) async fn pinned_standards_in(
        conn: &mut SqliteConnection,
        op: &str,
    ) -> Result<(i64, Option<String>), StorageError> {
        HarnessStore::pinned_standards_in(conn, op).await
    }
}
