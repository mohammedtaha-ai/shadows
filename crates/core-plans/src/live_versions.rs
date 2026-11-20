//! One job: the Active plans' live versions.
use crate::{LiveVersion, Plans, PlansError};
use core_projects::ProjectId;

impl Plans {
    /// Every Active plan of `project`, in `list`'s order, with its parts and its live version:
    /// its highest Frozen version and that version's tasks with their element pins. A Draft is
    /// named, never read.
    pub async fn live_versions(&self, project: &ProjectId) -> Result<Vec<LiveVersion>, PlansError> {
        Ok(self.store.live_versions(project).await?)
    }
}
