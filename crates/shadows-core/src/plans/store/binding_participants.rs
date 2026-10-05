//! Captured current and historical adoption reads for Design.
use crate::db::StorageError;
use crate::{
    AgreementBinding, AgreementId, AgreementRole, BindingParticipant, PartId, PlanId, PlanState,
    ProjectId, WorkflowId, WorkflowState,
};
use sqlx::{FromRow, SqliteConnection};

#[derive(FromRow)]
struct ParticipantRow {
    plan_id: String,
    workflow_id: String,
    title: String,
    plan_version: i64,
    revision: i64,
    plan_state: String,
    workflow_state: String,
    current: bool,
    task: u32,
    agreement_version: i64,
    part_id: String,
    role: String,
    operations_json: String,
    agreement_id: String,
}
pub(crate) async fn agreement_participants_in(
    conn: &mut SqliteConnection,
    project: &ProjectId,
    agreement: &AgreementId,
) -> Result<Vec<BindingParticipant>, StorageError> {
    read(conn, project, Some(agreement), None).await
}
pub(crate) async fn part_bindings_in(
    conn: &mut SqliteConnection,
    project: &ProjectId,
    part: &PartId,
) -> Result<Vec<BindingParticipant>, StorageError> {
    read(conn, project, None, Some(part)).await
}
async fn read(
    conn: &mut SqliteConnection,
    project: &ProjectId,
    agreement: Option<&AgreementId>,
    part: Option<&PartId>,
) -> Result<Vec<BindingParticipant>, StorageError> {
    let rows = sqlx::query_as::<_, ParticipantRow>(
        "SELECT p.id AS plan_id,w.id AS workflow_id,w.title,w.version AS plan_version, \
         w.revision,p.state AS plan_state,w.state AS workflow_state, \
         w.version=(SELECT MAX(n.version) FROM workflow n WHERE n.plan_id=p.id) AS current, \
         t.number AS task,b.agreement_version,b.part_id,b.role,b.operations_json,b.agreement_id \
         FROM task_agreement_binding b JOIN task t ON t.id=b.task_id \
         JOIN workflow w ON w.id=b.workflow_id JOIN plan p ON p.id=w.plan_id \
         WHERE p.project_id=? AND (? IS NULL OR b.agreement_id=?) \
         AND (? IS NULL OR b.part_id=?) ORDER BY p.id,w.version,t.number,b.agreement_id,b.role",
    )
    .bind(project.as_str())
    .bind(agreement.map(AgreementId::as_str))
    .bind(agreement.map(AgreementId::as_str))
    .bind(part.map(PartId::as_str))
    .bind(part.map(PartId::as_str))
    .fetch_all(conn)
    .await?;
    rows.into_iter()
        .map(|r| {
            Ok(BindingParticipant {
                plan_id: PlanId::from_stored(r.plan_id),
                workflow_id: WorkflowId::from_stored(r.workflow_id),
                title: r.title,
                plan_version: r.plan_version,
                revision: r.revision,
                current: r.current,
                plan_state: if r.plan_state == "Active" {
                    PlanState::Active
                } else {
                    PlanState::Archived
                },
                workflow_state: if r.workflow_state == "Draft" {
                    WorkflowState::Draft
                } else {
                    WorkflowState::Frozen
                },
                binding: AgreementBinding {
                    task: r.task,
                    agreement_id: AgreementId::from_stored(r.agreement_id),
                    version: r.agreement_version,
                    part_id: PartId::from_stored(r.part_id),
                    role: if r.role == "provides" {
                        AgreementRole::Provides
                    } else {
                        AgreementRole::Uses
                    },
                    operations: serde_json::from_str(&r.operations_json)?,
                },
            })
        })
        .collect()
}
