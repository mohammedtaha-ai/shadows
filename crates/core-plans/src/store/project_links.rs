//! Atomic project-link commands with plan dependency notifications.
use super::PlansStore;
use core_code::{Code, ProjectLink};
use core_db::{Actor, CommandContext, JournalEntry, Storage, StorageError};
use core_projects::{ProjectId, Projects};
impl PlansStore {
    pub(crate) async fn put_code_link(
        &self,
        ctx: &CommandContext,
        project: &ProjectId,
        linked: &ProjectId,
    ) -> Result<Option<ProjectLink>, StorageError> {
        let (ctx, project, linked, ts) =
            (ctx.clone(), project.clone(), linked.clone(), Storage::now());
        self.storage
            .write_txn(move |conn| {
                Box::pin(async move {
                    if let Some(done) =
                        CommandContext::classify(conn, &ctx, "Project", project.as_str()).await?
                    {
                        let done = ProjectId::from_recorded(done);
                        return Code::replayed_link_in(conn, &ctx, &project, &done)
                            .await
                            .map(Some);
                    }
                    Projects::require_live_in(conn, &project).await?;
                    match Projects::require_live_in(conn, &linked).await {
                        Ok(()) => {}
                        Err(StorageError::NotFound(_)) => return Ok(None),
                        Err(error) => return Err(error),
                    }
                    let made = Code::put_link_in(conn, &project, &linked, &ts).await?;
                    if made {
                        let event =
                            JournalEntry::new("ProjectLinked", Actor::user(&ctx.principal_id))
                                .with_project(project.as_str())
                                .with_payload(serde_json::json!({ "linked": linked }));
                        JournalEntry::append_event(conn, event.clone(), &ts).await?;
                        crate::Plans::notify_project_dependencies_in(
                            conn,
                            &project,
                            &Actor::user(&ctx.principal_id),
                            &ts,
                        )
                        .await?;
                    }
                    let (scope, key) = ("Project", project.as_str());
                    CommandContext::record_command(
                        conn,
                        &ctx,
                        scope,
                        key,
                        "ProjectLink",
                        linked.as_str(),
                        &ts,
                    )
                    .await?;
                    Code::load_link_in(conn, &project, &linked).await.map(Some)
                })
            })
            .await
    }
    pub(crate) async fn remove_code_link(
        &self,
        ctx: &CommandContext,
        project: &ProjectId,
        linked: &ProjectId,
    ) -> Result<bool, StorageError> {
        let (ctx, project, linked, ts) =
            (ctx.clone(), project.clone(), linked.clone(), Storage::now());
        self.storage
            .write_txn(move |conn| {
                Box::pin(async move {
                    if CommandContext::classify(conn, &ctx, "Project", project.as_str())
                        .await?
                        .is_some()
                    {
                        return Ok(true);
                    }
                    if !Code::remove_link_in(conn, &project, &linked).await? {
                        return Ok(false);
                    }
                    let event =
                        JournalEntry::new("ProjectUnlinked", Actor::user(&ctx.principal_id))
                            .with_project(project.as_str())
                            .with_payload(serde_json::json!({ "linked": linked }));
                    JournalEntry::append_event(conn, event.clone(), &ts).await?;
                    crate::Plans::notify_project_dependencies_in(
                        conn,
                        &project,
                        &Actor::user(&ctx.principal_id),
                        &ts,
                    )
                    .await?;
                    let (scope, key) = ("Project", project.as_str());
                    CommandContext::record_command(
                        conn,
                        &ctx,
                        scope,
                        key,
                        "ProjectLink",
                        linked.as_str(),
                        &ts,
                    )
                    .await?;
                    Ok(true)
                })
            })
            .await
    }
}
