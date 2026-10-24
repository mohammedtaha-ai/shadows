//! Invocation writes in the transaction that owns the turn command or ending.
use super::HarnessStore;
use crate::harness::NewInvocation;
use core_db::StorageError;
use sqlx::SqliteConnection;

impl HarnessStore {
    pub(in crate::harness) async fn record_invocation_in(
        conn: &mut SqliteConnection,
        operation: &str,
        input: NewInvocation<'_>,
        ts: &str,
    ) -> Result<(), StorageError> {
        let NewInvocation {
            identity,
            settings,
            versions,
            snapshot,
        } = input;
        let [
            role,
            kind,
            harness_path,
            harness_version,
            agent_path,
            agent_version,
        ] = identity;
        sqlx::query(
            "INSERT INTO agent_invocation
               (id, operation_id, role, harness_kind, harness_path, harness_version,
                agent_path, agent_version, requested_model, requested_mode,
                requested_effort, prompt_version,
                planner_instructions_version_id, standards_version,
                standards_additions_version_id, created_at)
             VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)",
        )
        .bind(uuid::Uuid::new_v4().to_string())
        .bind(operation)
        .bind(role)
        .bind(kind)
        .bind(harness_path)
        .bind(harness_version)
        .bind(agent_path)
        .bind(agent_version)
        .bind(&settings.model)
        .bind(&settings.mode)
        .bind(&settings.effort)
        .bind(&versions.prompt)
        .bind(&versions.instructions)
        .bind(versions.standards)
        .bind(&versions.additions)
        .bind(ts)
        .execute(&mut *conn)
        .await?;
        let recorded = crate::operations::Operations::get_operation_in(
            conn,
            &crate::operations::OperationId::from_stored(operation.to_owned()),
        )
        .await?;
        if let Some(thread) = recorded.thread_id {
            Self::remember_mode_in(
                conn,
                &core_threads::ThreadId::from_recorded(thread),
                kind,
                &settings.mode,
            )
            .await?;
        }
        if let Some(snapshot) = &snapshot {
            sqlx::query(
                "UPDATE agent_invocation SET usage_agent_slug=?,
                usage_provider_slug=?,usage_provider_name=?,usage_label=?,usage_preset=?
                WHERE operation_id=?",
            )
            .bind(&snapshot.agent_slug)
            .bind(&snapshot.provider_slug)
            .bind(&snapshot.provider_name)
            .bind(&snapshot.label)
            .bind(&snapshot.preset)
            .bind(operation)
            .execute(&mut *conn)
            .await?;
        }
        Ok(())
    }
    pub(in crate::harness) async fn observe_invocation_in(
        conn: &mut SqliteConnection,
        operation: &str,
        seen: &shadows_agent::events::TurnObservation,
    ) -> Result<(), StorageError> {
        let as_int = |v: Option<u64>| v.and_then(|n| i64::try_from(n).ok());
        sqlx::query(
            "UPDATE agent_invocation SET native_session_id=?, observed_model=?,
                context_used=?, context_window=? WHERE operation_id=?",
        )
        .bind(&seen.native_session_id)
        .bind(&seen.observed_model)
        .bind(as_int(seen.context_used))
        .bind(as_int(seen.context_window))
        .bind(operation)
        .execute(conn)
        .await?;
        Ok(())
    }
}

#[cfg(all(test, feature = "test-support"))]
impl crate::harness::Harness {
    pub(crate) async fn delete_invocation_for_test(
        conn: &mut SqliteConnection,
        operation: &str,
    ) -> Result<(), StorageError> {
        sqlx::query("DELETE FROM agent_invocation WHERE operation_id=?")
            .bind(operation)
            .execute(conn)
            .await?;
        Ok(())
    }
    pub(crate) async fn set_invocation_pin_for_test(
        conn: &mut SqliteConnection,
        operation: &str,
        pin: Option<&str>,
    ) -> Result<(), StorageError> {
        sqlx::query(
            "UPDATE agent_invocation SET standards_additions_version_id=? WHERE operation_id=?",
        )
        .bind(pin)
        .bind(operation)
        .execute(conn)
        .await?;
        Ok(())
    }
}
