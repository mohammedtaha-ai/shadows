//! Test-only owner calls over the fixture database.
use super::{Grant, GrantId, Grants, GrantsError, IssuedGrant, Token, store};
use crate::db::StorageError;
use crate::db::{CommandContext, Storage};
use crate::projects::ProjectId;
use crate::threads::{ThreadId, Threads};
use std::sync::Arc;
impl Grants {
    pub async fn issue_project_grant_for_test(
        storage: &Arc<Storage>,
        ctx: &CommandContext,
        project: &ProjectId,
    ) -> Result<IssuedGrant, StorageError> {
        store::GrantsStore::with_storage(storage.clone())
            .issue_project_grant(ctx, project)
            .await
    }
    pub async fn revoke_grant_for_test(
        storage: &Arc<Storage>,
        ctx: &CommandContext,
        id: &GrantId,
    ) -> Result<Grant, StorageError> {
        store::GrantsStore::with_storage(storage.clone())
            .revoke_grant(ctx, id)
            .await
    }
    pub async fn revoke_thread_grant_for_test(
        storage: &Arc<Storage>,
        id: &GrantId,
    ) -> Result<(), StorageError> {
        store::GrantsStore::with_storage(storage.clone())
            .revoke_thread_grant(id)
            .await
    }
    pub async fn revoke_all_thread_grants_for_test(
        storage: &Arc<Storage>,
    ) -> Result<u64, StorageError> {
        store::GrantsStore::with_storage(storage.clone())
            .revoke_all_thread_grants()
            .await
    }
    pub async fn grant_for_token_for_test(
        storage: &Arc<Storage>,
        raw: &str,
    ) -> Result<Option<Grant>, StorageError> {
        store::GrantsStore::with_storage(storage.clone())
            .grant_for_token(raw)
            .await
    }
    pub async fn list_project_grants_for_test(
        storage: &Arc<Storage>,
        project: &ProjectId,
    ) -> Result<Vec<Grant>, StorageError> {
        store::GrantsStore::with_storage(storage.clone())
            .list_project_grants(project)
            .await
    }
    pub async fn prepare_draft_for_test(
        storage: &Arc<Storage>,
        grant: &GrantId,
    ) -> Result<String, GrantsError> {
        store::GrantsStore::with_storage(storage.clone())
            .prepare_draft(grant)
            .await
    }
    pub async fn issue_thread_grant_for_test(
        storage: &Storage,
        thread: &ThreadId,
    ) -> Result<(Grant, Token), StorageError> {
        let thread = thread.clone();
        storage
            .write_txn(move |conn| {
                Box::pin(async move {
                    Threads::require_live_in(conn, &thread)
                        .await
                        .map_err(|e| match e {
                            StorageError::NotFound(_) => StorageError::NotFound("thread"),
                            other => other,
                        })?;
                    let project = Threads::project_in(conn, &thread).await?;
                    Self::issue_thread_grant_in(conn, &project, &thread).await
                })
            })
            .await
    }
}
