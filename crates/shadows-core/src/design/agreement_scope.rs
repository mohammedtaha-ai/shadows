//! Live grant attribution for agreement proposals.
use super::{AgreementContent, AgreementId, AgreementVersion, Design};
use crate::db::{CommandContext, fingerprint};
use crate::{CoreError, Grant, grants::Writer};

#[derive(Clone)]
pub(crate) struct AgreementOrigin {
    pub writer: Writer,
    pub operation: Option<crate::OperationId>,
}

impl AgreementScope {
    pub(super) fn writer(grant: &Grant) -> Result<Writer, CoreError> {
        Writer::from_grant(grant, Some("thread grant needs its conversation"))
    }
}

fn context(
    grant: &Grant,
    id: String,
    kind: &str,
    params: serde_json::Value,
) -> Result<CommandContext, CoreError> {
    let (principal_kind, principal_id) = AgreementScope::writer(grant)?.principal();
    Ok(CommandContext {
        agent: None,
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
        let writer = AgreementScope::writer(grant)?;
        let operation = match &writer {
            Writer::Planner { thread, .. } => self.operations.running_for(thread).await?,
            _ => None,
        };
        Ok(AgreementOrigin { writer, operation })
    }
    pub async fn agreements_for(&self, grant: &Grant) -> Result<Vec<AgreementVersion>, CoreError> {
        self.store
            .check_agreement_grant(&AgreementScope::writer(grant)?, &grant.project_id)
            .await?;
        self.agreements(&grant.project_id).await
    }
    pub async fn agreement_for(
        &self,
        grant: &Grant,
        id: &AgreementId,
        version: Option<i64>,
    ) -> Result<AgreementVersion, CoreError> {
        self.store
            .check_agreement_grant(&AgreementScope::writer(grant)?, &grant.project_id)
            .await?;
        self.agreement(&grant.project_id, id, version).await
    }

    pub async fn start_agreement_for(
        &self,
        grant: &Grant,
        command_id: String,
        id: Option<&AgreementId>,
        content: Option<AgreementContent>,
        reason: Option<String>,
    ) -> Result<AgreementVersion, CoreError> {
        let content = content
            .map(super::agreements::Agreements::normalize)
            .transpose()?;
        let reason = reason.map(|s| s.trim().to_string());
        let ctx = context(
            grant,
            command_id,
            "AgreementStart",
            serde_json::json!({
                "project":grant.project_id,"agreement_id":id,"content":content,"reason":reason
            }),
        )?;
        self.store
            .start_design_agreement(
                &ctx,
                Some(&self.agreement_origin(grant).await?),
                &grant.project_id,
                id,
                content,
                reason,
            )
            .await
            .map_err(CoreError::invalid_constraint)
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
        let content = super::agreements::Agreements::normalize(content)?;
        let ctx = context(
            grant,
            command_id,
            "AgreementEdit",
            serde_json::json!({
                "project": grant.project_id, "agreement_id": id, "version": version,
                "expected_revision": expected_revision, "content": content
            }),
        )?;
        self.store
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
            .map_err(CoreError::invalid_constraint)
    }
}

pub(in crate::design) struct AgreementScope;
