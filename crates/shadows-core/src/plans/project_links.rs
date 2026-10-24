//! Project-link commands composed above code and plans.
use super::Plans;
use super::scope::Scope;
use crate::code::ProjectLink;
use crate::error::{CoreError, ErrorCode};
use core_db::CommandContext;
use core_projects::ProjectId;
fn invalid(message: String) -> CoreError {
    Scope::refused(ErrorCode::InvalidCommand, message)
}
impl Plans {
    pub async fn link_projects(
        &self,
        command_id: String,
        project: &ProjectId,
        linked: &ProjectId,
    ) -> Result<ProjectLink, CoreError> {
        if project == linked {
            return Err(invalid("a project cannot be linked to itself".into()));
        }
        let params = serde_json::json!({ "project": project, "linked": linked });
        let c = CommandContext::user(command_id, "ProjectLinkPut", params);
        match self.store.put_code_link(&c, project, linked).await? {
            Some(link) => Ok(link),
            None => Err(invalid(format!("no such project {linked}"))),
        }
    }
    pub async fn unlink_projects(
        &self,
        command_id: String,
        project: &ProjectId,
        linked: &ProjectId,
    ) -> Result<(), CoreError> {
        let params = serde_json::json!({ "project": project, "linked": linked });
        let c = CommandContext::user(command_id, "ProjectLinkRemove", params);
        if self.store.remove_code_link(&c, project, linked).await? {
            Ok(())
        } else {
            Err(invalid("no such link".into()))
        }
    }
}
