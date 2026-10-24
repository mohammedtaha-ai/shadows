//! One job: the links and the active limit, as commands. Each is
//! a `CommandContext::user` with a `CommandId`, written by one store method; a
//! refusal the store cannot phrase is written here.

use super::Code;
use super::model::{CodeSettings, ProjectLink};
use crate::CodeError;
use core_db::CommandContext;
use core_projects::ProjectId;

/// The values `active_limit` may take.
const LIMITS: std::ops::RangeInclusive<u32> = 1..=20;

impl Code {
    /// The projects `project` reads, by their slug.
    pub async fn links(&self, project: &ProjectId) -> Result<Vec<ProjectLink>, CodeError> {
        Ok(self.inner.store.code_links(project).await?)
    }

    /// The stored settings.
    pub async fn settings(&self) -> Result<CodeSettings, CodeError> {
        let limit = self.inner.store.code_active_limit().await?;
        Ok(CodeSettings {
            active_limit: limit as u32,
        })
    }

    /// "CodeActiveLimitSet", params { "active_limit" }: 1 to 20, else
    /// INVALID_COMMAND. The active set follows the stored limit at once, so
    /// lowering it stops the least recently used workers.
    pub async fn set_active_limit(
        &self,
        command_id: String,
        active_limit: u32,
    ) -> Result<CodeSettings, CodeError> {
        if !LIMITS.contains(&active_limit) {
            return Err(CodeError::InvalidCommand(format!(
                "the active limit must be from 1 to 20, not {active_limit}"
            )));
        }
        let params = serde_json::json!({ "active_limit": active_limit });
        let c = CommandContext::user(command_id, "CodeActiveLimitSet", params);
        let store = &self.inner.store;
        let set = store.set_code_active_limit(&c, active_limit).await?;
        let mut active = self.inner.active.lock().await;
        // The stored one, not `set`: a replay answers an older value. Read
        // under the lock, so of two calls at once the last to take it reads
        // the newer limit, and the set never follows the older one.
        let now = store.code_active_limit().await?.max(1) as usize;
        active.limit(now);
        active.settle(self);
        Ok(CodeSettings { active_limit: set })
    }
}
