//! Agreed-version checks shared with Plans in its write transaction.
use crate::{AgreementId, AgreementRole, AgreementState, PartId, ProjectId};
use core_db::StorageError;
use sqlx::SqliteConnection;

impl crate::design::Design {
    pub(crate) async fn check_agreement_binding_in(
        conn: &mut SqliteConnection,
        project: &ProjectId,
        agreement: &AgreementId,
        number: i64,
        part: &PartId,
        role: AgreementRole,
        operations: &[String],
    ) -> Result<(), StorageError> {
        let version =
            super::agreements::AgreementsRows::load(conn, project, agreement, Some(number))
                .await
                .map_err(|e| match e {
                    StorageError::NotFound(_) => StorageError::Constraint(
                        "binding needs an Agreed version in this project; a person \
                     agrees the agreement on its Contracts page first"
                            .into(),
                    ),
                    other => other,
                })?;
        if version.state != AgreementState::Agreed {
            return Err(StorageError::Constraint(
                "binding needs an Agreed version; a person agrees the agreement on \
             its Contracts page first"
                    .into(),
            ));
        }
        if !version
            .content
            .parties
            .iter()
            .any(|p| p.part_id == *part && p.role == role)
        {
            return Err(StorageError::Constraint(
                "binding role/part is not declared in this version".into(),
            ));
        }
        let ids = super::super::agreement_validation::AgreementValidation::operation_ids(
            &version.content,
        );
        if operations.is_empty() || operations.iter().any(|id| !ids.contains(id)) {
            return Err(StorageError::Constraint(
                "binding operation is absent from its exact version".into(),
            ));
        }
        Ok(())
    }
}
