//! The standards version recorded for an invocation in its project snapshot.

use sqlx::SqliteConnection;

use crate::db::StorageError;
use crate::projects::Projects;
use crate::standards::{EffectiveStandards, Standards};
use crate::turns::{OperationId, Turns};

impl Turns {
    /// Resolves the invocation's pin without substituting newer additions.
    pub(crate) async fn standards_for_invocation_in(
        conn: &mut SqliteConnection,
        operation: &OperationId,
    ) -> Result<EffectiveStandards, StorageError> {
        let operation_row = crate::operations::Operations::get_operation_in(conn, operation)
            .await
            .map_err(|error| match error {
                StorageError::NotFound(_) => StorageError::NotFound("invocation"),
                other => other,
            })?;
        let thread = operation_row
            .thread_id
            .ok_or(StorageError::NotFound("invocation"))?;
        let pin: Option<Option<String>> = sqlx::query_scalar(
            "SELECT standards_additions_version_id FROM agent_invocation WHERE operation_id=?",
        )
        .bind(operation.as_str())
        .fetch_optional(&mut *conn)
        .await?;
        let pin = pin.ok_or(StorageError::NotFound("invocation"))?;
        let project = crate::threads::Threads::project_in(
            conn,
            &crate::threads::ThreadId::from_stored(thread),
        )
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

#[cfg(all(test, feature = "test-support"))]
mod tests {
    use crate::db::{Storage, StorageError};
    use crate::testing::turn::{default_turn_settings, new_turn, turn_command};
    use crate::turns::{OperationId, Turns};

    #[tokio::test]
    async fn invocation_standards_preserve_pins_nulls_and_missing_results() {
        let db = tempfile::tempdir().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let core = crate::testing::app(db.path().join("pins.sqlite3"))
            .await
            .unwrap();
        let project = core
            .projects()
            .create(
                "project".into(),
                "project",
                "Project",
                dir.path().to_str().unwrap(),
            )
            .await
            .unwrap();
        let other = core
            .projects()
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
        let started = Storage::start_turn(&turns.storage, &command, turn)
            .await
            .unwrap();
        core.standards()
            .save_standards_additions("second".into(), &project.id, Default::default())
            .await
            .unwrap();
        let mut conn = turns.storage.reader().acquire().await.unwrap();
        assert_eq!(
            Turns::standards_for_invocation_in(&mut conn, &started.operation_id)
                .await
                .unwrap()
                .additions,
            Some(first)
        );
        sqlx::query("UPDATE agent_invocation SET standards_additions_version_id=NULL")
            .execute(&mut *conn)
            .await
            .unwrap();
        assert!(
            Turns::standards_for_invocation_in(&mut conn, &started.operation_id)
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
        sqlx::query("UPDATE agent_invocation SET standards_additions_version_id=?")
            .bind(&foreign.id)
            .execute(&mut *conn)
            .await
            .unwrap();
        assert!(matches!(
            Turns::standards_for_invocation_in(&mut conn, &started.operation_id).await,
            Err(StorageError::NotFound("invocation standards additions"))
        ));
        assert!(matches!(
            Turns::standards_for_invocation_in(&mut conn, &OperationId::generate()).await,
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
        core.threads()
            .remove("remove-thread".into(), &thread.id)
            .await
            .unwrap();
        core.projects()
            .remove("remove-project".into(), &project.id)
            .await
            .unwrap();
        assert!(matches!(
            Turns::standards_for_invocation_in(&mut conn, &started.operation_id).await,
            Err(StorageError::NotFound("invocation"))
        ));
        core.code().shut_down().await;
    }
}
