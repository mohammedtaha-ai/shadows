use serde::{Deserialize, Serialize};

use super::ids::{ProjectId, ResearchId, Timestamp};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResearchArtifact {
    pub id: ResearchId,
    pub project_id: ProjectId,
    pub title: String,
    pub source: Option<String>,
    pub summary: String,
    pub created_at: Timestamp,
}
