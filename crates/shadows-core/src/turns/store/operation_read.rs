use crate::harness::Harness;
use crate::operations::Operations;
use crate::turns::model::{InvocationView, Operation, OperationId};
use core_db::StorageError;
use core_threads::ThreadId;
impl super::TurnsStore {
    pub async fn get_operation(&self, op: &OperationId) -> Result<Operation, StorageError> {
        let mut conn = self.storage.reader().acquire().await?;
        let row = Operations::get_operation_in(&mut conn, op).await?;
        Ok(into_operation(
            row,
            Harness::invocation_in(&mut conn, op.as_str()).await?,
        ))
    }
    pub async fn list_operations_for_thread(
        &self,
        thread: &ThreadId,
    ) -> Result<Vec<Operation>, StorageError> {
        let mut conn = self.storage.reader().begin().await?;
        let rows = Operations::list_operations_for_thread_in(&mut conn, thread).await?;
        let ids: Vec<_> = rows.iter().map(|row| row.id.as_str()).collect();
        let mut invocations = Harness::invocations_in(&mut conn, &ids).await?;
        Ok(rows
            .into_iter()
            .map(|row| {
                let invocation = invocations.remove(row.id.as_str());
                into_operation(row, invocation)
            })
            .collect())
    }
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
            .project_lifecycle()
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
                let started = crate::Turns::start_turn_for_test(
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
                                crate::harness::Harness::delete_invocation_for_test(
                                    conn,
                                    started.operation_id.as_str(),
                                )
                                .await?;
                            }
                            Ok(())
                        })
                    })
                    .await
                    .unwrap();
                // The fixture commits turns without spawning an adapter.
                super::super::TurnsStore {
                    storage: turns.storage.clone(),
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
