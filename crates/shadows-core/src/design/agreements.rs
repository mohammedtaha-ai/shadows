//! Design's shared-agreement entry points.
use super::{AgreementContent, AgreementId, AgreementVersion, Design};
use crate::{CoreError, ErrorCode, ProjectId, StorageError, app::user_command};

impl Design {
    pub async fn review_agreement(
        &self,
        project: &ProjectId,
        id: &AgreementId,
    ) -> Result<super::AgreementReview, CoreError> {
        Ok(self.storage.review_design_agreement(project, id).await?)
    }
    pub async fn agree_agreement(
        &self,
        command_id: String,
        project: &ProjectId,
        id: &AgreementId,
        expected_revision: i64,
        review_id: String,
    ) -> Result<AgreementVersion, CoreError> {
        let ctx = user_command(
            command_id,
            "AgreementAgree",
            serde_json::json!({
                "project":project,"agreement_id":id,"expected_revision":expected_revision,
                "review_id":review_id
            }),
        );
        self.storage
            .agree_design_agreement(&ctx, project, id, expected_revision, review_id)
            .await
            .map_err(map_error)
    }
    pub async fn agreements(
        &self,
        project: &ProjectId,
    ) -> Result<Vec<AgreementVersion>, CoreError> {
        Ok(self.storage.design_agreements(project).await?)
    }
    pub async fn agreement(
        &self,
        project: &ProjectId,
        id: &AgreementId,
        version: Option<i64>,
    ) -> Result<AgreementVersion, CoreError> {
        Ok(self.storage.design_agreement(project, id, version).await?)
    }
    pub async fn start_agreement(
        &self,
        command_id: String,
        project: &ProjectId,
        id: Option<&AgreementId>,
        content: Option<AgreementContent>,
        reason: Option<String>,
    ) -> Result<AgreementVersion, CoreError> {
        let content = content.map(normalize).transpose()?;
        let reason = reason.map(|s| s.trim().to_string());
        let ctx = user_command(
            command_id,
            "AgreementStart",
            serde_json::json!({
                "project": project, "agreement_id": id, "content": content, "reason": reason
            }),
        );
        self.storage
            .start_design_agreement(&ctx, None, project, id, content, reason)
            .await
            .map_err(map_error)
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
        let content = normalize(content)?;
        let ctx = user_command(
            command_id,
            "AgreementEdit",
            serde_json::json!({
                "project": project, "agreement_id": id, "version": version,
                "expected_revision": expected_revision,
                "content": content
            }),
        );
        self.storage
            .edit_design_agreement(&ctx, None, project, id, version, expected_revision, content)
            .await
            .map_err(map_error)
    }
}

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
pub(super) fn map_error(error: StorageError) -> CoreError {
    match error {
        StorageError::Constraint(message) => CoreError::Refused {
            code: ErrorCode::InvalidCommand,
            message,
        },
        other => other.into(),
    }
}
