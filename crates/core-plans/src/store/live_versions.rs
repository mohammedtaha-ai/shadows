//! One job: reading live versions in one snapshot.
use sqlx::SqliteConnection;

use super::PlansStore;
use super::task::TaskRows;
use crate::{LiveFrozen, LiveTask, LiveVersion, PlanId, Plans, WorkflowId};
use core_db::StorageError;
use core_design::ProjectLayout;
use core_projects::ProjectId;

impl PlansStore {
    /// `live_versions` in one reader transaction.
    pub async fn live_versions(
        &self,
        project: &ProjectId,
        layout: Option<&ProjectLayout>,
    ) -> Result<Vec<LiveVersion>, crate::PlansError> {
        let mut read = self.storage.reader().begin().await?;
        let loaded;
        let layout = match layout {
            Some(layout) => layout,
            None => {
                loaded = core_design::Design::layout_in(&mut read, project).await?;
                &loaded
            }
        };
        let versions = live_versions_in(&mut read, project, layout).await?;
        read.commit().await?;
        Ok(versions)
    }
}

/// `live_versions`, as `conn` sees it: the Active plans in `list`'s order (by when the plan
/// was created, then its id).
async fn live_versions_in(
    conn: &mut SqliteConnection,
    project: &ProjectId,
    layout: &ProjectLayout,
) -> Result<Vec<LiveVersion>, StorageError> {
    let plans: Vec<String> = sqlx::query_scalar(
        "SELECT id FROM plan WHERE project_id = ? AND state = 'Active'
             ORDER BY julianday(created_at), id",
    )
    .bind(project.as_str())
    .fetch_all(&mut *conn)
    .await?;
    let mut versions = Vec::with_capacity(plans.len());
    for plan in plans {
        let plan = PlanId::from_recorded(plan);
        let latest = |state: &'static str| {
            sqlx::query_as::<_, (String, i64, String)>(
                "SELECT id, version, title FROM workflow WHERE plan_id = ? AND state = ?
                     ORDER BY version DESC LIMIT 1",
            )
            .bind(plan.as_str().to_string())
            .bind(state)
        };
        let frozen = latest("Frozen").fetch_optional(&mut *conn).await?;
        let draft = latest("Draft").fetch_optional(&mut *conn).await?;
        let title = frozen
            .as_ref()
            .or(draft.as_ref())
            .map(|(_, _, title)| title.clone())
            .unwrap_or_default();
        let live = match frozen {
            Some((id, version, _)) => {
                let workflow_id = WorkflowId::from_recorded(id);
                let tasks = TaskRows::tasks_of(conn, &workflow_id)
                    .await?
                    .into_iter()
                    .map(|task| LiveTask {
                        ownership: crate::TaskOwnership::resolve(layout, &task.content),
                        number: task.content.number,
                        title: task.content.title,
                        elements: task
                            .content
                            .contract
                            .map(|c| c.elements)
                            .unwrap_or_default(),
                    })
                    .collect();
                Some(LiveFrozen {
                    workflow_id,
                    version,
                    tasks,
                })
            }
            None => None,
        };
        let explicit = Plans::plan_parts_in(conn, project, &plan).await?;
        // A Draft-only plan has only its saved links: a Draft is never read.
        let ownership: Vec<_> = live
            .iter()
            .flat_map(|frozen: &LiveFrozen| &frozen.tasks)
            .map(|task| task.ownership.clone())
            .collect();
        versions.push(LiveVersion {
            parts: Plans::effective_parts(&explicit, &ownership),
            outcomes: Plans::delivery_outcomes_in(conn, project, &plan).await?,
            plan_id: plan,
            title,
            live,
            draft: draft.map(|(id, _, _)| WorkflowId::from_recorded(id)),
        });
    }
    Ok(versions)
}
