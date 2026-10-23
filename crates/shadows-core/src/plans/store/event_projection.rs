//! Identifier-only plan payloads read in the event subscriber's snapshot.
use crate::db::StorageError;
use crate::plans::Plans;
use sqlx::SqliteConnection;

impl Plans {
    pub(crate) async fn project_event_payload_in(
        conn: &mut SqliteConnection,
        kind: &str,
        payload: &str,
    ) -> Result<String, StorageError> {
        if !matches!(
            kind,
            "WorkflowDraftStarted"
                | "WorkflowEdited"
                | "WorkflowFrozen"
                | "PlanArchived"
                | "PlanUnarchived"
        ) {
            return Ok(payload.to_owned());
        }
        let row: Option<(String, String)> = sqlx::query_as(
            "SELECT plan_id, id FROM workflow
             WHERE id = COALESCE(json_extract(?, '$.workflow_id'),
                (SELECT id FROM workflow WHERE plan_id = json_extract(?, '$.plan')
                 ORDER BY version DESC LIMIT 1))",
        )
        .bind(payload)
        .bind(payload)
        .fetch_optional(conn)
        .await?;
        let (plan, workflow) = match row {
            Some((plan, workflow)) => (Some(plan), Some(workflow)),
            None => (None, None),
        };
        Ok(serde_json::json!({ "plan_id": plan, "workflow_id": workflow }).to_string())
    }
}
