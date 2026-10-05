//! Snapshot reads of exact shared-agreement versions.
use super::parts;
use crate::ProjectId;
use crate::db::{Storage, StorageError};
use crate::design::{AgreementContent, AgreementId, AgreementState, AgreementVersion, Design};
use sqlx::{FromRow, SqliteConnection};

#[derive(FromRow)]
struct VersionRow {
    agreement_id: String,
    project_id: String,
    version: i64,
    revision: i64,
    state: String,
    reason: Option<String>,
    writer_json: String,
    created_at: String,
    agreed_at: Option<String>,
    content_json: String,
}
impl VersionRow {
    fn domain(self) -> Result<AgreementVersion, StorageError> {
        let content: AgreementContent = serde_json::from_str(&self.content_json)?;
        let issues = Design::validate_agreement(&content);
        Ok(AgreementVersion {
            agreement_id: AgreementId::from_stored(self.agreement_id),
            project_id: ProjectId::from_stored(self.project_id),
            version: self.version,
            revision: self.revision,
            state: if self.state == "Draft" {
                AgreementState::Draft
            } else {
                AgreementState::Agreed
            },
            reason: self.reason,
            writer: serde_json::from_str(&self.writer_json)?,
            created_at: self.created_at,
            agreed_at: self.agreed_at,
            content,
            issues,
        })
    }
}
pub(super) async fn load(
    conn: &mut SqliteConnection,
    project: &ProjectId,
    id: &AgreementId,
    version: Option<i64>,
) -> Result<AgreementVersion, StorageError> {
    sqlx::query_as::<_, VersionRow>(
        "SELECT * FROM agreement_version WHERE project_id=? AND agreement_id=? \
         AND (? IS NULL OR version=?) ORDER BY version DESC LIMIT 1",
    )
    .bind(project.as_str())
    .bind(id.as_str())
    .bind(version)
    .bind(version)
    .fetch_optional(conn)
    .await?
    .ok_or(StorageError::NotFound("agreement version"))?
    .domain()
}
impl Storage {
    pub async fn check_agreement_grant(
        &self,
        writer: &crate::command::Writer,
        project: &ProjectId,
    ) -> Result<(), StorageError> {
        let mut tx = self.reader().begin().await?;
        crate::grants::check_writer(&mut tx, writer, project).await?;
        tx.commit().await?;
        Ok(())
    }
    pub async fn design_agreement(
        &self,
        project: &ProjectId,
        id: &AgreementId,
        version: Option<i64>,
    ) -> Result<AgreementVersion, StorageError> {
        let mut tx = self.reader().begin().await?;
        parts::revision(&mut tx, project).await?;
        let result = load(&mut tx, project, id, version).await?;
        tx.commit().await?;
        Ok(result)
    }
    pub async fn design_agreements(
        &self,
        project: &ProjectId,
    ) -> Result<Vec<AgreementVersion>, StorageError> {
        let mut tx = self.reader().begin().await?;
        parts::revision(&mut tx, project).await?;
        let rows = sqlx::query_as::<_, VersionRow>(
            "SELECT v.* FROM agreement_version v WHERE v.project_id=? \
             AND v.version=(SELECT MAX(n.version) FROM agreement_version n \
               WHERE n.agreement_id=v.agreement_id) ORDER BY v.agreement_id",
        )
        .bind(project.as_str())
        .fetch_all(&mut *tx)
        .await?;
        let result = rows
            .into_iter()
            .map(VersionRow::domain)
            .collect::<Result<Vec<_>, _>>()?;
        tx.commit().await?;
        Ok(result)
    }
}
