use serde::{Deserialize, Serialize};

use super::ids::{ProjectId, Timestamp};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Project {
    pub id: ProjectId,
    pub name: String,
    pub created_at: Timestamp,
}
