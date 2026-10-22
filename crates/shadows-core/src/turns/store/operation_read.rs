use crate::db::StorageError;
use crate::operations::Operations;
use crate::threads::ThreadId;
use crate::turns::model::{InvocationView, Operation, OperationId};
use sqlx::SqliteConnection;
use std::collections::HashMap;
impl super::TurnsStore<'_> {
    pub async fn get_operation(&self, op: &OperationId) -> Result<Operation, StorageError> {
        let mut conn = self.storage.reader().acquire().await?;
        let row = Operations::get_operation_in(&mut conn, op).await?;
        Ok(into_operation(row, invocation_of(&mut conn, op).await?))
    }
    pub async fn list_operations_for_thread(
        &self,
        thread: &ThreadId,
    ) -> Result<Vec<Operation>, StorageError> {
        let mut conn = self.storage.reader().begin().await?;
        let rows = Operations::list_operations_for_thread_in(&mut conn, thread).await?;
        let ids: Vec<_> = rows.iter().map(|row| row.id.as_str()).collect();
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
        Ok(rows
            .into_iter()
            .map(|row| {
                let invocation = invocations.remove(row.id.as_str());
                into_operation(row, invocation)
            })
            .collect())
    }
}
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

pub(super) async fn invocation_of(
    conn: &mut SqliteConnection,
    op: &OperationId,
) -> Result<Option<InvocationView>, StorageError> {
    let row: Option<InvocationRow> = sqlx::query_as(
        "SELECT operation_id, harness_kind, harness_version, agent_version,
                requested_model, requested_mode, requested_effort,
                observed_model, context_used, context_window
           FROM agent_invocation WHERE operation_id = ?",
    )
    .bind(op.as_str())
    .fetch_optional(&mut *conn)
    .await?;
    Ok(row.map(into_view))
}

fn into_operation(
    r: crate::operations::OperationRecord,
    invocation: Option<InvocationView>,
) -> Operation {
    Operation {
        id: r.id,
        kind: r.kind,
        status_kind: r.status_kind,
        thread_id: r.thread_id,
        runtime_instance_id: r.runtime_instance_id,
        outcome_json: r.outcome_json,
        failure_stage: r.failure_stage,
        failure_reason: r.failure_reason,
        interrupt_reason: r.interrupt_reason,
        cancel_requested_at: r.cancel_requested_at,
        created_at: r.created_at,
        started_at: r.started_at,
        finished_at: r.finished_at,
        invocation,
    }
}

#[cfg(all(test, feature = "test-support"))]
mod tests {
    use crate::db::Storage;
    use crate::testing::turn::{default_turn_settings, new_turn, turn_command};
    use crate::turns::model::FailureStage;

    #[tokio::test]
    async fn listing_keeps_order_and_missing_invocations_without_other_threads() {
        let db = tempfile::tempdir().unwrap();
        let root = tempfile::tempdir().unwrap();
        let core = crate::testing::app(db.path().join("listing.sqlite3"))
            .await
            .unwrap();
        let project = core
            .projects()
            .create(
                "project".into(),
                "project",
                "Project",
                root.path().to_str().unwrap(),
            )
            .await
            .unwrap();
        let turns = core.turns();
        let settings = default_turn_settings();
        let mut expected = Vec::new();
        let mut selected = None;
        for name in ["selected", "other"] {
            let thread = core
                .threads()
                .create(name.into(), &project.id, name, None, None)
                .await
                .unwrap();
            assert!(turns.operations(&thread.id).await.unwrap().is_empty());
            if name == "selected" {
                selected = Some(thread.id.clone());
            }
            let count = if name == "selected" { 66 } else { 3 };
            for index in 0..count {
                let command =
                    turn_command(&format!("{name}-{index}"), &thread.id, "hello", &settings);
                let started = Storage::start_turn(
                    &turns.storage,
                    &command,
                    new_turn(&thread.id, &turns.runtime, "hello", &settings),
                )
                .await
                .unwrap();
                let op = started.operation_id.clone();
                turns
                    .storage
                    .write_txn(move |conn| {
                        Box::pin(async move {
                            if index == 1 {
                                sqlx::query("DELETE FROM agent_invocation WHERE operation_id=?")
                                    .bind(started.operation_id.as_str())
                                    .execute(&mut *conn)
                                    .await?;
                            }
                            Ok(())
                        })
                    })
                    .await
                    .unwrap();
                // The fixture commits turns without spawning an adapter.
                super::super::TurnsStore {
                    storage: &turns.storage,
                }
                .mark_operation_failed(&op, FailureStage::Prepare, "fixture")
                .await
                .unwrap();
                if name == "selected" {
                    expected.insert(0, (op, index != 1));
                }
            }
        }
        let actual: Vec<_> = turns
            .operations(&selected.unwrap())
            .await
            .unwrap()
            .into_iter()
            .map(|op| (op.id, op.invocation.is_some()))
            .collect();
        assert_eq!(actual, expected);
        core.code().shut_down().await;
    }
}
