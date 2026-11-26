//! One job: the Active plans' live versions.
use crate::{LiveVersion, Plans, PlansError};
use core_design::ProjectLayout;
use core_projects::ProjectId;

impl Plans {
    /// Every Active plan of `project`, in `list`'s order, with its parts and its live version:
    /// its highest Frozen version and that version's tasks with their element pins. A Draft is
    /// named, never read.
    pub async fn live_versions(&self, project: &ProjectId) -> Result<Vec<LiveVersion>, PlansError> {
        self.store.live_versions(project, None).await
    }

    /// `live_versions` against a `layout` the caller already read, so one
    /// composed view resolves task ownership against the map it shows.
    pub async fn live_versions_for_layout(
        &self,
        project: &ProjectId,
        layout: &ProjectLayout,
    ) -> Result<Vec<LiveVersion>, PlansError> {
        self.store.live_versions(project, Some(layout)).await
    }
}
