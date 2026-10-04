//! Agreed-version checks shared with Plans in its write transaction.
use crate::db::StorageError;
use crate::{AgreementBinding, AgreementState, ProjectId};
use sqlx::SqliteConnection;

pub(crate) async fn check_agreement_binding_in(
    conn: &mut SqliteConnection,
    project: &ProjectId,
    binding: &AgreementBinding,
) -> Result<(), StorageError> {
    let version =
        super::agreements::load(conn, project, &binding.agreement_id, Some(binding.version))
            .await
            .map_err(|e| match e {
                StorageError::NotFound(_) => StorageError::Constraint(
                    "binding needs an Agreed version in this project; a person agrees the agreement on its Contracts page first".into(),
                ),
                other => other,
            })?;
    if version.state != AgreementState::Agreed {
        return Err(StorageError::Constraint(
            "binding needs an Agreed version; a person agrees the agreement on its Contracts page first".into(),
        ));
    }
    if !version
        .content
        .parties
        .iter()
        .any(|p| p.part_id == binding.part_id && p.role == binding.role)
    {
        return Err(StorageError::Constraint(
            "binding role/part is not declared in this version".into(),
        ));
    }
    let ids = super::super::agreement_validation::operation_ids(&version.content);
    if binding.operations.is_empty() || binding.operations.iter().any(|id| !ids.contains(id)) {
        return Err(StorageError::Constraint(
            "binding operation is absent from its exact version".into(),
        ));
    }
    Ok(())
}
