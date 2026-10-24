use core_db::{Storage, StorageError};
use core_operations::OperationId;
use core_projects::Projects;
use core_standards::{EffectiveStandards, Standards};
use core_threads::ThreadId;
use sqlx::SqliteConnection;

impl crate::turns::Turns {
    /// Resolves the invocation's pin without substituting newer additions.
    pub(crate) async fn standards_for_invocation_in(
        conn: &mut SqliteConnection,
        operation: &OperationId,
    ) -> Result<EffectiveStandards, StorageError> {
        let operation_row = core_operations::Operations::get_operation_in(conn, operation)
            .await
            .map_err(|error| match error {
                StorageError::NotFound(_) => StorageError::NotFound("invocation"),
                other => other,
            })?;
        let thread = operation_row
            .thread_id
            .ok_or(StorageError::NotFound("invocation"))?;
        let (_, pin) =
            crate::harness::Harness::invocation_standards_in(conn, operation.as_str()).await?;
        let project =
            core_threads::Threads::project_in(conn, &core_threads::ThreadId::from_recorded(thread))
                .await
                .map_err(|error| match error {
                    StorageError::NotFound(_) => StorageError::NotFound("invocation"),
                    other => other,
                })?;
        Projects::require_live_in(conn, &project)
            .await
            .map_err(|error| match error {
                StorageError::NotFound(_) => StorageError::NotFound("invocation"),
                other => other,
            })?;
        let additions = match pin {
            None => None,
            Some(id) => Some(
                Standards::load_version_in(conn, &project, &id)
                    .await
                    .map_err(|error| match error {
                        StorageError::NotFound(_) => {
                            StorageError::NotFound("invocation standards additions")
                        }
                        other => other,
                    })?,
            ),
        };
        Ok(EffectiveStandards {
            base: Standards::base().clone(),
            additions,
        })
    }
}

impl crate::turns::Turns {
    pub(in crate::turns) async fn invocation_context(
        storage: &std::sync::Arc<Storage>,
        thread: &ThreadId,
        operation: &OperationId,
    ) -> crate::harness::InvocationContext {
        let latest = super::TurnsStore::with_storage(storage.clone())
            .latest_invocation_versions(thread)
            .await;
        let standards = async {
            let mut conn = storage.reader().begin().await?;
            let effective = Self::standards_for_invocation_in(&mut conn, operation).await?;
            conn.commit().await?;
            Ok(effective)
        }
        .await;
        crate::harness::InvocationContext { latest, standards }
    }
}

#[cfg(all(test, feature = "test-support"))]
mod tests {
    use crate::OperationId;
    use crate::testing::turn::{default_turn_settings, new_turn, turn_command};
    use core_db::StorageError;

    #[tokio::test]
    async fn invocation_standards_preserve_pins_nulls_and_missing_results() {
        let db = tempfile::tempdir().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let core = crate::testing::app(db.path().join("pins.sqlite3"))
            .await
            .unwrap();
        let project = core
            .project_lifecycle()
            .create(
                "project".into(),
                "project",
                "Project",
                dir.path().to_str().unwrap(),
            )
            .await
            .unwrap();
        let other = core
            .project_lifecycle()
            .create(
                "other".into(),
                "other",
                "Other",
                dir.path().to_str().unwrap(),
            )
            .await
            .unwrap();
        let thread = core
            .threads()
            .create("thread".into(), &project.id, "Thread", None, None)
            .await
            .unwrap();
        let first = core
            .standards()
            .save_standards_additions("first".into(), &project.id, Default::default())
            .await
            .unwrap();
        let settings = default_turn_settings();
        let command = turn_command("turn", &thread.id, "hello", &settings);
        let turns = core.turns();
        let mut turn = new_turn(&thread.id, &turns.runtime, "hello", &settings);
        turn.standards_additions_version = Some(&first.id);
        let started = crate::Turns::start_turn_for_test(&turns.storage, &command, turn)
            .await
            .unwrap();
        core.standards()
            .save_standards_additions("second".into(), &project.id, Default::default())
            .await
            .unwrap();
        let mut conn = turns.storage.reader().acquire().await.unwrap();
        assert_eq!(
            crate::turns::Turns::standards_for_invocation_in(&mut conn, &started.operation_id)
                .await
                .unwrap()
                .additions,
            Some(first)
        );
        crate::harness::Harness::set_invocation_pin_for_test(
            &mut conn,
            started.operation_id.as_str(),
            None,
        )
        .await
        .unwrap();
        assert!(
            crate::turns::Turns::standards_for_invocation_in(&mut conn, &started.operation_id)
                .await
                .unwrap()
                .additions
                .is_none(),
            "a null pin must not select the newer additions"
        );
        let foreign = core
            .standards()
            .save_standards_additions("foreign".into(), &other.id, Default::default())
            .await
            .unwrap();
        crate::harness::Harness::set_invocation_pin_for_test(
            &mut conn,
            started.operation_id.as_str(),
            Some(&foreign.id),
        )
        .await
        .unwrap();
        assert!(matches!(
            crate::turns::Turns::standards_for_invocation_in(&mut conn, &started.operation_id)
                .await,
            Err(StorageError::NotFound("invocation standards additions"))
        ));
        assert!(matches!(
            crate::turns::Turns::standards_for_invocation_in(&mut conn, &OperationId::generate())
                .await,
            Err(StorageError::NotFound("invocation"))
        ));
        crate::turns::Turns::mark_operation_failed_for_test(
            &turns.storage,
            &started.operation_id,
            crate::turns::model::FailureStage::Prepare,
            "fixture ended before spawning",
        )
        .await
        .unwrap();
        core.turns()
            .remove_thread("remove-thread".into(), &thread.id)
            .await
            .unwrap();
        core.project_lifecycle()
            .remove("remove-project".into(), &project.id)
            .await
            .unwrap();
        assert!(matches!(
            crate::turns::Turns::standards_for_invocation_in(&mut conn, &started.operation_id)
                .await,
            Err(StorageError::NotFound("invocation"))
        ));
        core.code().shut_down().await;
    }
}
