//! Atomic project-link commands with plan dependency notifications.
use super::PlansStore;
use crate::code::{Code, ProjectLink};
use crate::db::{Actor, CommandContext, StorageError, append_event, classify, now, record_command};
use crate::events::DurableEvent;
use crate::projects::{ProjectId, Projects};
impl PlansStore {
    pub(in crate::plans) async fn put_code_link(
        &self,
        ctx: &CommandContext,
        project: &ProjectId,
        linked: &ProjectId,
    ) -> Result<Option<ProjectLink>, StorageError> {
        let (ctx, project, linked, ts) = (ctx.clone(), project.clone(), linked.clone(), now());
        self.storage
            .write_txn(move |conn| {
                Box::pin(async move {
                    if let Some(done) = classify(conn, &ctx, "Project", project.as_str()).await? {
                        let done = ProjectId::from_stored(done);
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
                            DurableEvent::new("ProjectLinked", Actor::user(&ctx.principal_id))
                                .with_project(&project)
                                .with_payload(serde_json::json!({ "linked": linked }));
                        append_event(conn, &event, &ts).await?;
                        crate::plans::Plans::notify_project_dependencies_in(
                            conn,
                            &project,
                            &Actor::user(&ctx.principal_id),
                            &ts,
                        )
                        .await?;
                    }
                    let (scope, key) = ("Project", project.as_str());
                    record_command(conn, &ctx, scope, key, "ProjectLink", linked.as_str(), &ts)
                        .await?;
                    Code::load_link_in(conn, &project, &linked).await.map(Some)
                })
            })
            .await
    }
    pub(in crate::plans) async fn remove_code_link(
        &self,
        ctx: &CommandContext,
        project: &ProjectId,
        linked: &ProjectId,
    ) -> Result<bool, StorageError> {
        let (ctx, project, linked, ts) = (ctx.clone(), project.clone(), linked.clone(), now());
        self.storage
            .write_txn(move |conn| {
                Box::pin(async move {
                    if classify(conn, &ctx, "Project", project.as_str())
                        .await?
                        .is_some()
                    {
                        return Ok(true);
                    }
                    if !Code::remove_link_in(conn, &project, &linked).await? {
                        return Ok(false);
                    }
                    let event =
                        DurableEvent::new("ProjectUnlinked", Actor::user(&ctx.principal_id))
                            .with_project(&project)
                            .with_payload(serde_json::json!({ "linked": linked }));
                    append_event(conn, &event, &ts).await?;
                    crate::plans::Plans::notify_project_dependencies_in(
                        conn,
                        &project,
                        &Actor::user(&ctx.principal_id),
                        &ts,
                    )
                    .await?;
                    let (scope, key) = ("Project", project.as_str());
                    record_command(conn, &ctx, scope, key, "ProjectLink", linked.as_str(), &ts)
                        .await?;
                    Ok(true)
                })
            })
            .await
    }
}
