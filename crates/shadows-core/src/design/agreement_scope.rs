//! Live grant attribution for agreement proposals.
use super::{AgreementContent, AgreementId, AgreementReview, AgreementVersion, Design};
use crate::{
    CoreError, ErrorCode, Grant, GrantKind,
    command::{CommandContext, Writer, fingerprint},
};

#[derive(Clone)]
pub(crate) struct AgreementOrigin {
    pub writer: Writer,
    pub operation: Option<crate::OperationId>,
}

pub(super) fn writer(grant: &Grant) -> Result<Writer, CoreError> {
    match grant.kind {
        GrantKind::Project => Ok(Writer::External {
            grant: grant.id.clone(),
        }),
        GrantKind::Thread => Ok(Writer::Planner {
            grant: grant.id.clone(),
            thread: grant.thread_id.clone().ok_or_else(|| CoreError::Refused {
                code: ErrorCode::GrantScope,
                message: "thread grant needs its conversation".into(),
            })?,
        }),
    }
}
fn context(
    grant: &Grant,
    id: String,
    kind: &str,
    params: serde_json::Value,
) -> Result<CommandContext, CoreError> {
    let (principal_kind, principal_id) = writer(grant)?.principal();
    Ok(CommandContext {
        principal_kind: principal_kind.into(),
        principal_id,
        command_id: id,
        command_kind: kind.into(),
        command_schema_ver: 1,
        request_fingerprint: fingerprint(kind, &params),
    })
}
impl Design {
    async fn agreement_origin(&self, grant: &Grant) -> Result<AgreementOrigin, CoreError> {
        let writer = writer(grant)?;
        let operation = match &writer {
            Writer::Planner { thread, .. } => self.handles.running_for(thread).await,
            _ => None,
        };
        Ok(AgreementOrigin { writer, operation })
    }
    pub async fn agreements_for(&self, grant: &Grant) -> Result<Vec<AgreementVersion>, CoreError> {
        self.storage
            .check_agreement_grant(&writer(grant)?, &grant.project_id)
            .await?;
        self.agreements(&grant.project_id).await
    }
    pub async fn agreement_for(
        &self,
        grant: &Grant,
        id: &AgreementId,
        version: Option<i64>,
    ) -> Result<AgreementVersion, CoreError> {
        self.storage
            .check_agreement_grant(&writer(grant)?, &grant.project_id)
            .await?;
        self.agreement(&grant.project_id, id, version).await
    }
    pub async fn review_agreement_for(
        &self,
        grant: &Grant,
        id: &AgreementId,
    ) -> Result<AgreementReview, CoreError> {
        self.storage
            .check_agreement_grant(&writer(grant)?, &grant.project_id)
            .await?;
        self.review_agreement(&grant.project_id, id).await
    }
    pub async fn start_agreement_for(
        &self,
        grant: &Grant,
        command_id: String,
        id: Option<&AgreementId>,
        content: Option<AgreementContent>,
        reason: Option<String>,
    ) -> Result<AgreementVersion, CoreError> {
        let content = content.map(super::agreements::normalize).transpose()?;
        let reason = reason.map(|s| s.trim().to_string());
        let ctx = context(
            grant,
            command_id,
            "AgreementStart",
            serde_json::json!({
                "project":grant.project_id,"agreement_id":id,"content":content,"reason":reason
            }),
        )?;
        self.storage
            .start_design_agreement(
                &ctx,
                Some(&self.agreement_origin(grant).await?),
                &grant.project_id,
                id,
                content,
                reason,
            )
            .await
            .map_err(super::agreements::map_error)
    }
    pub async fn edit_agreement_for(
        &self,
        grant: &Grant,
        command_id: String,
        id: &AgreementId,
        version: i64,
        expected_revision: i64,
        content: AgreementContent,
    ) -> Result<AgreementVersion, CoreError> {
        let content = super::agreements::normalize(content)?;
        let ctx = context(
            grant,
            command_id,
            "AgreementEdit",
            serde_json::json!({
                "project": grant.project_id, "agreement_id": id, "version": version,
                "expected_revision": expected_revision, "content": content
            }),
        )?;
        self.storage
            .edit_design_agreement(
                &ctx,
                Some(&self.agreement_origin(grant).await?),
                &grant.project_id,
                id,
                version,
                expected_revision,
                content,
            )
            .await
            .map_err(super::agreements::map_error)
    }
}
