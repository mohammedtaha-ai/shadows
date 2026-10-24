//! Removal composes table owners in one serialized transaction.
use super::error::ProjectLifecycleError;
use crate::CoreError;
use crate::code::Code;
use crate::grants::Grants;
use crate::plans::Plans;
use crate::threads::Threads;
use core_db::{Actor, CommandContext, JournalEntry, Storage};
use core_projects::{Project, ProjectId, Projects};
use std::sync::Arc;

#[derive(Clone)]
pub(super) struct ProjectLifecycleStore {
    storage: Arc<Storage>,
}

impl ProjectLifecycleStore {
    pub(super) fn with_storage(storage: Arc<Storage>) -> Self {
        Self { storage }
    }

    pub(super) async fn remove(
        &self,
        ctx: &CommandContext,
        project: &ProjectId,
    ) -> Result<Project, CoreError> {
        let (ctx, id, ts) = (ctx.clone(), project.clone(), Storage::now());
        self.storage
            .write_txn_as(move |conn| {
                Box::pin(async move {
                    if CommandContext::classify(conn, &ctx, "Project", id.as_str())
                        .await?
                        .is_some()
                    {
                        return Ok(Projects::retained_in(conn, &id).await?);
                    }
                    Projects::require_live_in(conn, &id).await?;
                    if Threads::has_live_project_threads_in(conn, &id).await? {
                        return Err(ProjectLifecycleError::ProjectHasThreads.into());
                    }
                    Projects::mark_removed_in(conn, &id, &ts).await?;
                    let actor = Actor::user(&ctx.principal_id);
                    Plans::notify_project_dependencies_in(conn, &id, &actor, &ts).await?;
                    Code::delete_code_index_in(conn, &id).await?;
                    Code::delete_code_links_in(conn, &id).await?;
                    Grants::revoke_project_grants_in(conn, &id, actor.clone(), &ts).await?;
                    JournalEntry::append_event(
                        conn,
                        JournalEntry {
                            event_id: uuid::Uuid::new_v4().to_string(),
                            kind: "ProjectRemoved".into(),
                            project_id: Some(id.as_str().to_owned()),
                            thread_id: None,
                            operation_id: None,
                            actor,
                            causation: None,
                            correlation_id: None,
                            payload_json: "{}".into(),
                        },
                        &ts,
                    )
                    .await?;
                    let key = id.as_str();
                    CommandContext::record_command(conn, &ctx, "Project", key, "Project", key, &ts)
                        .await?;
                    Ok(Projects::retained_in(conn, &id).await?)
                })
            })
            .await
    }
}
