//! One job: the links and the active limit, as commands (spec §15.6). Each is
//! a `user_command` with a `CommandId`, written by one store method; a
//! refusal the store cannot phrase is written here.

use super::Code;
use super::model::{CodeSettings, ProjectLink};
use crate::app::user_command;
use crate::error::{CoreError, ErrorCode};
use crate::projects::ProjectId;

/// The values `active_limit` may take (§15.6).
const LIMITS: std::ops::RangeInclusive<u32> = 1..=20;

impl Code {
    /// The projects `project` reads, by their slug.
    pub async fn links(&self, project: &ProjectId) -> Result<Vec<ProjectLink>, CoreError> {
        Ok(self.inner.storage.code_links(project).await?)
    }

    /// Lets `project` read `linked`'s index, one way: "ProjectLinkPut",
    /// params { "project", "linked" }. Linking twice answers the link.
    pub async fn link(
        &self,
        command_id: String,
        project: &ProjectId,
        linked: &ProjectId,
    ) -> Result<ProjectLink, CoreError> {
        if project == linked {
            return Err(invalid("a project cannot be linked to itself".into()));
        }
        let params = serde_json::json!({ "project": project, "linked": linked });
        let c = user_command(command_id, "ProjectLinkPut", params);
        match self
            .inner
            .storage
            .put_code_link(&c, project, linked)
            .await?
        {
            Some(link) => Ok(link),
            None => Err(invalid(format!("no such project {linked}"))),
        }
    }

    /// Removes the link: "ProjectLinkRemove", params { "project", "linked" }.
    pub async fn unlink(
        &self,
        command_id: String,
        project: &ProjectId,
        linked: &ProjectId,
    ) -> Result<(), CoreError> {
        let params = serde_json::json!({ "project": project, "linked": linked });
        let c = user_command(command_id, "ProjectLinkRemove", params);
        if self
            .inner
            .storage
            .remove_code_link(&c, project, linked)
            .await?
        {
            Ok(())
        } else {
            Err(invalid("no such link".into()))
        }
    }

    /// The stored settings.
    pub async fn settings(&self) -> Result<CodeSettings, CoreError> {
        let limit = self.inner.storage.code_active_limit().await?;
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
    ) -> Result<CodeSettings, CoreError> {
        if !LIMITS.contains(&active_limit) {
            return Err(invalid(format!(
                "the active limit must be from 1 to 20, not {active_limit}"
            )));
        }
        let params = serde_json::json!({ "active_limit": active_limit });
        let c = user_command(command_id, "CodeActiveLimitSet", params);
        let storage = &self.inner.storage;
        let set = storage.set_code_active_limit(&c, active_limit).await?;
        let mut active = self.inner.active.lock().await;
        // The stored one, not `set`: a replay answers an older value. Read
        // under the lock, so of two calls at once the last to take it reads
        // the newer limit, and the set never follows the older one.
        let now = storage.code_active_limit().await?.max(1) as usize;
        active.limit(now);
        active.settle(self);
        Ok(CodeSettings { active_limit: set })
    }
}

fn invalid(message: String) -> CoreError {
    CoreError::Refused {
        code: ErrorCode::InvalidCommand,
        message,
    }
}
