//! Design's shared-agreement entry points.
use super::{AgreementContent, AgreementId, AgreementVersion, Design};
use crate::db::CommandContext;
use crate::{CoreError, ErrorCode, ProjectId};

impl Design {
    pub async fn agreements(
        &self,
        project: &ProjectId,
    ) -> Result<Vec<AgreementVersion>, CoreError> {
        Ok(self.store.design_agreements(project).await?)
    }
    pub async fn agreement(
        &self,
        project: &ProjectId,
        id: &AgreementId,
        version: Option<i64>,
    ) -> Result<AgreementVersion, CoreError> {
        Ok(self.store.design_agreement(project, id, version).await?)
    }
    pub async fn start_agreement(
        &self,
        command_id: String,
        project: &ProjectId,
        id: Option<&AgreementId>,
        content: Option<AgreementContent>,
        reason: Option<String>,
    ) -> Result<AgreementVersion, CoreError> {
        let content = content.map(Agreements::normalize).transpose()?;
        let reason = reason.map(|s| s.trim().to_string());
        let ctx = CommandContext::user(
            command_id,
            "AgreementStart",
            serde_json::json!({
                "project": project, "agreement_id": id, "content": content, "reason": reason
            }),
        );
        self.store
            .start_design_agreement(&ctx, None, project, id, content, reason)
            .await
            .map_err(CoreError::invalid_constraint)
    }
    pub async fn edit_agreement(
        &self,
        command_id: String,
        project: &ProjectId,
        id: &AgreementId,
        version: i64,
        expected_revision: i64,
        content: AgreementContent,
    ) -> Result<AgreementVersion, CoreError> {
        let content = Agreements::normalize(content)?;
        let ctx = CommandContext::user(
            command_id,
            "AgreementEdit",
            serde_json::json!({
                "project": project, "agreement_id": id, "version": version,
                "expected_revision": expected_revision,
                "content": content
            }),
        );
        self.store
            .edit_design_agreement(&ctx, None, project, id, version, expected_revision, content)
            .await
            .map_err(CoreError::invalid_constraint)
    }
}

impl Agreements {
    pub(super) fn normalize(mut content: AgreementContent) -> Result<AgreementContent, CoreError> {
        content.capability = content.capability.trim().to_string();
        if content.capability.is_empty() {
            return Err(CoreError::Refused {
                code: ErrorCode::InvalidCommand,
                message: "the agreement needs a capability name".into(),
            });
        }
        Ok(content)
    }
}

pub(in crate::design) struct Agreements;
