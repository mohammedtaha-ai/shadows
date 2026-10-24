//! The mode remembered for one conversation.
use super::HarnessStore;
use crate::CoreError;
use crate::harness::Harness;
use core_db::{CommandContext, Storage, StorageError};
use core_operations::Operations;
use core_projects::Projects;
use core_threads::{ThreadId, Threads};
use shadows_agent::policy;
use sqlx::SqliteConnection;

impl HarnessStore {
    pub(in crate::harness) async fn forget_mode(
        &self,
        thread: &ThreadId,
    ) -> Result<(), StorageError> {
        let thread = thread.clone();
        self.storage
            .write_txn(move |conn| {
                Box::pin(async move { Harness::remove_mode_in(conn, &thread).await })
            })
            .await
    }

    pub(in crate::harness) async fn opening_mode(
        &self,
        thread: &ThreadId,
    ) -> Result<String, CoreError> {
        let thread = thread.clone();
        self.storage
            .write_txn_as(move |conn| {
                Box::pin(async move {
                    let context = Threads::turn_context_in(conn, &thread).await?;
                    let selected = Harness::selected_mode_in(conn, &thread).await?;
                    if Self::mode_in(conn, &thread, &context.harness)
                        .await?
                        .is_some_and(|mode| mode != selected)
                    {
                        Harness::remove_mode_in(conn, &thread).await?;
                    }
                    Ok(selected)
                })
            })
            .await
    }

    pub(super) async fn mode_in(
        conn: &mut SqliteConnection,
        thread: &ThreadId,
        harness: &str,
    ) -> Result<Option<String>, StorageError> {
        Ok(sqlx::query_scalar(
            "SELECT mode FROM harness_thread_mode WHERE thread_id = ? AND harness_kind = ?",
        )
        .bind(thread.as_str())
        .bind(harness)
        .fetch_optional(conn)
        .await?)
    }

    pub(in crate::harness) async fn remember_mode_in(
        conn: &mut SqliteConnection,
        thread: &ThreadId,
        harness: &str,
        mode: &str,
    ) -> Result<(), StorageError> {
        sqlx::query(
            "INSERT INTO harness_thread_mode (thread_id, harness_kind, mode) VALUES (?, ?, ?)
             ON CONFLICT (thread_id) DO UPDATE
             SET harness_kind = excluded.harness_kind, mode = excluded.mode",
        )
        .bind(thread.as_str())
        .bind(harness)
        .bind(mode)
        .execute(conn)
        .await?;
        Ok(())
    }

    pub(in crate::harness) async fn change_mode(
        &self,
        command: CommandContext,
        thread: ThreadId,
        mode: String,
    ) -> Result<bool, CoreError> {
        self.storage
            .write_txn_as(move |conn| {
                Box::pin(async move {
                    let replay =
                        CommandContext::classify(conn, &command, "Thread", thread.as_str()).await?;
                    let context = Threads::turn_context_in(conn, &thread).await?;
                    if replay.is_none() {
                        Harness::check_mode_in(conn, &thread, &mode).await?;
                        Self::remember_mode_in(conn, &thread, &context.harness, &mode).await?;
                        CommandContext::record_command(
                            conn,
                            &command,
                            "Thread",
                            thread.as_str(),
                            "PlanningThread",
                            thread.as_str(),
                            &Storage::now(),
                        )
                        .await?;
                    }
                    Ok(Operations::has_open_in(conn, &thread).await?)
                })
            })
            .await
    }
}

impl Harness {
    pub(crate) async fn remove_mode_in(
        conn: &mut SqliteConnection,
        thread: &ThreadId,
    ) -> Result<(), StorageError> {
        sqlx::query("DELETE FROM harness_thread_mode WHERE thread_id = ?")
            .bind(thread.as_str())
            .execute(conn)
            .await?;
        Ok(())
    }

    pub(crate) async fn copy_mode_in(
        conn: &mut SqliteConnection,
        source: &ThreadId,
        fork: &ThreadId,
    ) -> Result<(), StorageError> {
        sqlx::query(
            "INSERT INTO harness_thread_mode (thread_id, harness_kind, mode)
             SELECT ?, harness_kind, mode FROM harness_thread_mode WHERE thread_id = ?",
        )
        .bind(fork.as_str())
        .bind(source.as_str())
        .execute(conn)
        .await?;
        Ok(())
    }

    pub(in crate::harness) async fn check_mode_in(
        conn: &mut SqliteConnection,
        thread: &ThreadId,
        mode: &str,
    ) -> Result<(), CoreError> {
        let context = Threads::turn_context_in(conn, thread).await?;
        if !policy::allowed_modes(&context.harness).contains(&mode) {
            return Err(CoreError::SettingNotOffered {
                what: "mode".into(),
                id: mode.into(),
                detail: None,
            });
        }
        let project = Projects::get_in(conn, &context.project_id).await?;
        if !project
            .allowed_modes
            .get(&context.harness)
            .is_some_and(|m| m.iter().any(|m| m == mode))
        {
            return Err(CoreError::ModeNotAllowed(mode.into()));
        }
        Ok(())
    }

    pub(in crate::harness) async fn selected_mode_in(
        conn: &mut SqliteConnection,
        thread: &ThreadId,
    ) -> Result<String, CoreError> {
        let context = Threads::turn_context_in(conn, thread).await?;
        if let Some(mode) = HarnessStore::mode_in(conn, thread, &context.harness).await? {
            match Self::check_mode_in(conn, thread, &mode).await {
                Ok(()) => return Ok(mode),
                Err(CoreError::ModeNotAllowed(_) | CoreError::SettingNotOffered { .. }) => {}
                Err(error) => return Err(error),
            }
        }
        policy::default_mode(&context.harness)
            .map(str::to_owned)
            .ok_or_else(|| CoreError::HarnessUnavailable(context.harness))
    }
}
