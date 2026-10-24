//! Test-only owner entry points over the fixture's shared storage handle.
use super::{NewThreadEntry, PlanningThread, ThreadEntry, ThreadId, Threads, TurnContext, store};
use crate::projects::ProjectId;
use core_db::StorageError;
use core_db::{CommandContext, Storage};
use std::sync::Arc;

impl Threads {
    pub async fn create_planning_thread_for_test(
        storage: &Arc<Storage>,
        ctx: &CommandContext,
        project_id: &ProjectId,
        title: &str,
        harness: &str,
        profile: Option<&str>,
    ) -> Result<PlanningThread, crate::CoreError> {
        store::ThreadsStore::with_storage(storage.clone())
            .create_planning_thread(ctx, project_id, title, harness, profile)
            .await
    }
    pub async fn turn_context_for_test(
        storage: &Arc<Storage>,
        thread_id: &ThreadId,
    ) -> Result<TurnContext, StorageError> {
        store::ThreadsStore::with_storage(storage.clone())
            .turn_context(thread_id)
            .await
    }
    pub async fn record_harness_session_for_test(
        storage: &Arc<Storage>,
        thread_id: &ThreadId,
        session_id: &str,
    ) -> Result<bool, StorageError> {
        store::ThreadsStore::with_storage(storage.clone())
            .record_harness_session(thread_id, session_id)
            .await
    }
    pub async fn list_threads_for_project_for_test(
        storage: &Arc<Storage>,
        project_id: &ProjectId,
    ) -> Result<Vec<PlanningThread>, StorageError> {
        store::ThreadsStore::with_storage(storage.clone())
            .list_threads_for_project(project_id)
            .await
    }
    pub async fn list_threads_for_profile_for_test(
        storage: &Arc<Storage>,
        slug: &str,
    ) -> Result<Vec<PlanningThread>, StorageError> {
        store::ThreadsStore::with_storage(storage.clone())
            .list_threads_for_profile(slug)
            .await
    }
    pub async fn append_thread_entry_for_test(
        storage: &Arc<Storage>,
        thread_id: &ThreadId,
        entry: NewThreadEntry<'_>,
    ) -> Result<ThreadEntry, StorageError> {
        store::ThreadsStore::with_storage(storage.clone())
            .append_thread_entry(thread_id, entry)
            .await
    }
    pub async fn list_thread_entries_for_test(
        storage: &Arc<Storage>,
        thread_id: &ThreadId,
    ) -> Result<Vec<ThreadEntry>, StorageError> {
        store::ThreadsStore::with_storage(storage.clone())
            .list_thread_entries(thread_id)
            .await
    }
    pub async fn get_thread_for_test(
        storage: &Storage,
        thread: &ThreadId,
    ) -> Result<PlanningThread, StorageError> {
        let mut conn = storage.reader().acquire().await?;
        Self::get_in(&mut conn, thread).await
    }
    pub async fn title_from_harness_for_test(
        storage: &Arc<Storage>,
        thread: &ThreadId,
        title: &str,
    ) -> Result<bool, StorageError> {
        let Some(title) = Self::harness_title(title) else {
            return Ok(false);
        };
        let thread = thread.clone();
        let ts = core_db::Storage::now();
        storage
            .write_txn(move |conn| {
                Box::pin(
                    async move { Self::title_from_harness_in(conn, &thread, &title, &ts).await },
                )
            })
            .await
    }
}
