//! Snapshot reads of exact shared-agreement versions.
use super::DesignStore;
use super::parts;
use crate::design::{AgreementContent, AgreementId, AgreementState, AgreementVersion, Design};
use core_db::StorageError;
use core_projects::ProjectId;
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
            project_id: ProjectId::from_recorded(self.project_id),
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
impl AgreementsRows {
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
}

impl DesignStore {
    pub async fn check_agreement_grant(
        &self,
        writer: &crate::grants::Writer,
        project: &ProjectId,
    ) -> Result<(), crate::CoreError> {
        let mut tx = self.storage.reader().begin().await?;
        crate::grants::Grants::check_writer(&mut tx, writer, project).await?;
        tx.commit().await?;
        Ok(())
    }
    pub async fn design_agreement(
        &self,
        project: &ProjectId,
        id: &AgreementId,
        version: Option<i64>,
    ) -> Result<AgreementVersion, StorageError> {
        let mut tx = self.storage.reader().begin().await?;
        parts::PartsRows::revision(&mut tx, project).await?;
        let result = AgreementsRows::load(&mut tx, project, id, version).await?;
        tx.commit().await?;
        Ok(result)
    }
    pub async fn design_agreements(
        &self,
        project: &ProjectId,
    ) -> Result<Vec<AgreementVersion>, StorageError> {
        let mut tx = self.storage.reader().begin().await?;
        parts::PartsRows::revision(&mut tx, project).await?;
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

impl crate::design::Design {
    pub(crate) async fn agreement_in(
        conn: &mut sqlx::SqliteConnection,
        project: &ProjectId,
        id: &AgreementId,
        version: Option<i64>,
    ) -> Result<AgreementVersion, StorageError> {
        AgreementsRows::load(conn, project, id, version).await
    }
    pub(crate) async fn agreed_before_in(
        conn: &mut sqlx::SqliteConnection,
        project: &ProjectId,
        id: &AgreementId,
        version: i64,
    ) -> Result<Option<i64>, StorageError> {
        Ok(sqlx::query_scalar(
            "SELECT MAX(version) FROM agreement_version WHERE project_id=? \
             AND agreement_id=? AND state='Agreed' AND version<?",
        )
        .bind(project.as_str())
        .bind(id.as_str())
        .bind(version)
        .fetch_one(conn)
        .await?)
    }
    pub(crate) async fn mark_agreed_in(
        conn: &mut sqlx::SqliteConnection,
        id: &AgreementId,
        version: i64,
        ts: &str,
    ) -> Result<(), StorageError> {
        sqlx::query(
            "UPDATE agreement_version SET state='Agreed',agreed_at=? \
                    WHERE agreement_id=? AND version=?",
        )
        .bind(ts)
        .bind(id.as_str())
        .bind(version)
        .execute(conn)
        .await?;
        Ok(())
    }
}

pub(in crate::design) struct AgreementsRows;
