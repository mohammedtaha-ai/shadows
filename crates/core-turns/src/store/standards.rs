use core_db::{Storage, StorageError};
use core_operations::OperationId;
use core_projects::Projects;
use core_standards::{EffectiveStandards, Standards};
use core_threads::ThreadId;
use sqlx::SqliteConnection;

impl crate::Turns {
    /// Resolves the invocation's pin without substituting newer additions.
    pub async fn standards_for_invocation_in(
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
            core_harness::Harness::invocation_standards_in(conn, operation.as_str()).await?;
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

impl crate::Turns {
    pub async fn invocation_context(
        storage: &std::sync::Arc<Storage>,
        thread: &ThreadId,
        operation: &OperationId,
    ) -> core_harness::InvocationContext {
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
        core_harness::InvocationContext { latest, standards }
    }
}
