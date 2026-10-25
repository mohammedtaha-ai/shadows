//! Agreement review composed above Design and plan bindings.
use super::{AgreementReview, Plans};
use crate::{Grant, PlansError, ProjectId, StorageError};
use core_db::CommandContext;
use core_design::{AgreementId, AgreementVersion};
impl Plans {
    pub async fn review_agreement(
        &self,
        project: &ProjectId,
        id: &AgreementId,
    ) -> Result<super::AgreementReview, PlansError> {
        Ok(self.store.review_design_agreement(project, id).await?)
    }
    pub async fn agree_agreement(
        &self,
        command_id: String,
        project: &ProjectId,
        id: &AgreementId,
        expected_revision: i64,
        review_id: String,
    ) -> Result<AgreementVersion, PlansError> {
        let ctx = CommandContext::user(
            command_id,
            "AgreementAgree",
            serde_json::json!({
                "project":project,"agreement_id":id,"expected_revision":expected_revision,
                "review_id":review_id
            }),
        );
        self.store
            .agree_design_agreement(&ctx, project, id, expected_revision, review_id)
            .await
            .map_err(PlansError::invalid_constraint)
    }
    pub async fn review_agreement_for(
        &self,
        grant: &Grant,
        id: &AgreementId,
    ) -> Result<AgreementReview, PlansError> {
        let mut conn = self
            .store
            .storage
            .reader()
            .begin()
            .await
            .map_err(StorageError::from)?;
        core_grants::Grants::check_writer(
            &mut conn,
            &super::scope::Scope::writer_of(grant)?,
            &grant.project_id,
        )
        .await?;
        self.review_agreement(&grant.project_id, id).await
    }
}
