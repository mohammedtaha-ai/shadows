//! Persisted exact shared-agreement pins for task versions.
use crate::db::StorageError;
use crate::{AgreementBinding, AgreementId, AgreementRole, PartId, ProjectId, WorkflowId};
use sqlx::SqliteConnection;

pub(super) async fn load(
    conn: &mut SqliteConnection,
    workflow: &WorkflowId,
) -> Result<Vec<AgreementBinding>, StorageError> {
    type Row = (u32, String, i64, String, String, String);
    let rows: Vec<Row> = sqlx::query_as(
        "SELECT t.number,b.agreement_id,b.agreement_version,b.part_id,b.role,b.operations_json \
         FROM task_agreement_binding b JOIN task t ON t.id=b.task_id \
         WHERE b.workflow_id=? ORDER BY t.number,b.agreement_id,b.role",
    )
    .bind(workflow.as_str())
    .fetch_all(conn)
    .await?;
    rows.into_iter()
        .map(|(task, id, version, part, role, operations)| {
            Ok(AgreementBinding {
                task,
                agreement_id: AgreementId::from_stored(id),
                version,
                part_id: PartId::from_stored(part),
                role: if role == "provides" {
                    AgreementRole::Provides
                } else {
                    AgreementRole::Uses
                },
                operations: serde_json::from_str(&operations)?,
            })
        })
        .collect()
}
pub(super) async fn validate(
    conn: &mut SqliteConnection,
    project: &ProjectId,
    bindings: &[AgreementBinding],
) -> Result<(), StorageError> {
    for binding in bindings {
        crate::design::check_agreement_binding_in(conn, project, binding).await?;
    }
    Ok(())
}
pub(super) async fn clear(
    conn: &mut SqliteConnection,
    workflow: &WorkflowId,
) -> Result<(), StorageError> {
    sqlx::query("DELETE FROM task_agreement_binding WHERE workflow_id=?")
        .bind(workflow.as_str())
        .execute(conn)
        .await?;
    Ok(())
}
pub(super) async fn write(
    conn: &mut SqliteConnection,
    workflow: &WorkflowId,
    bindings: &[AgreementBinding],
) -> Result<(), StorageError> {
    for binding in bindings {
        let role = match binding.role {
            AgreementRole::Provides => "provides",
            AgreementRole::Uses => "uses",
        };
        sqlx::query(
            "INSERT INTO task_agreement_binding \
            (workflow_id,task_id,agreement_id,agreement_version,part_id,role, \
             operations_json,project_id) \
            SELECT ?,t.id,?,?,?,?,?,p.project_id FROM task t \
            JOIN workflow w ON w.id=t.workflow_id JOIN plan p ON p.id=w.plan_id \
            WHERE t.workflow_id=? AND t.number=?",
        )
        .bind(workflow.as_str())
        .bind(binding.agreement_id.as_str())
        .bind(binding.version)
        .bind(binding.part_id.as_str())
        .bind(role)
        .bind(serde_json::to_string(&binding.operations)?)
        .bind(workflow.as_str())
        .bind(binding.task)
        .execute(&mut *conn)
        .await?;
    }
    Ok(())
}
