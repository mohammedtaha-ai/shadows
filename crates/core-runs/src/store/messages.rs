//! Ordered plain questions and their durable replies.
use super::RunsStore;
use crate::{AgentMessage, AgentMessageId, Attempt, AttemptId, RunsError};
use core_db::Storage;
use core_operations::OperationId;
use core_plans::WorkflowId;
use core_threads::ThreadId;

#[derive(Debug, Clone, sqlx::FromRow)]
pub(crate) struct MessageRecord {
    pub id: String,
    pub attempt_id: String,
    pub thread_id: String,
    pub message: String,
    pub task: u32,
    pub state: String,
    pub operation_id: Option<String>,
    pub reply: Option<String>,
    pub reason: Option<String>,
    pub notice_json: Option<String>,
}

impl MessageRecord {
    pub fn view(&self) -> Result<AgentMessage, RunsError> {
        Ok(AgentMessage {
            id: AgentMessageId::from_recorded(self.id.clone()),
            attempt_id: AttemptId::from_recorded(self.attempt_id.clone()),
            task: self.task,
            state: serde_json::from_value(serde_json::Value::String(self.state.clone()))?,
        })
    }
}

impl RunsStore {
    pub async fn keep_message_notice(&self, id: &str, notice: String) -> Result<String, RunsError> {
        let id = id.to_string();
        self.storage
            .write_txn_as(move |tx| {
                Box::pin(async move {
                    sqlx::query_scalar(
                        "UPDATE run_agent_messages SET notice_json=COALESCE(notice_json,?)
                WHERE id=? RETURNING notice_json",
                    )
                    .bind(notice)
                    .bind(id)
                    .fetch_one(&mut *tx)
                    .await
                    .map_err(Into::into)
                })
            })
            .await
    }
    pub async fn dispatch_barrier(&self, thread: &ThreadId) -> Result<bool, RunsError> {
        Ok(sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM run_attempts WHERE thread_id=? AND
            (state='running' OR (released_at IS NULL
                AND state IN ('failed','stopped','interrupted'))))",
        )
        .bind(thread.as_str())
        .fetch_one(self.storage.reader())
        .await?)
    }
    pub async fn add_message(
        &self,
        attempt: &Attempt,
        text: &str,
    ) -> Result<(AgentMessage, bool), RunsError> {
        let (attempt, text) = (attempt.clone(), text.to_string());
        self.storage
            .write_txn_as(move |tx| {
                Box::pin(async move {
                    let existing: Option<String> = sqlx::query_scalar(
                        "SELECT id FROM run_agent_messages
                WHERE attempt_id=? AND message=? AND state IN ('waiting','sent','replied')
                ORDER BY ordinal LIMIT 1",
                    )
                    .bind(attempt.id.as_str())
                    .bind(&text)
                    .fetch_optional(&mut *tx)
                    .await?;
                    let (id, inserted) = match existing {
                        Some(id) => (id, false),
                        None => {
                            let id = AgentMessageId::generate().as_str().to_string();
                            sqlx::query(
                                "INSERT INTO run_agent_messages
                        (id,attempt_id,thread_id,ordinal,message,state,created_at)
                        VALUES (?,?,?,(SELECT COALESCE(MAX(ordinal),0)+1 FROM run_agent_messages
                          WHERE thread_id=?),?,'waiting',?)",
                            )
                            .bind(&id)
                            .bind(attempt.id.as_str())
                            .bind(attempt.thread_id.as_str())
                            .bind(attempt.thread_id.as_str())
                            .bind(text)
                            .bind(Storage::now())
                            .execute(&mut *tx)
                            .await?;
                            (id, true)
                        }
                    };
                    let row: MessageRecord = sqlx::query_as(
                        "SELECT m.*,a.task FROM run_agent_messages m
                JOIN run_attempts a ON a.id=m.attempt_id WHERE m.id=?",
                    )
                    .bind(id)
                    .fetch_one(&mut *tx)
                    .await?;
                    Ok((row.view()?, inserted))
                })
            })
            .await
    }

    pub async fn message(&self, id: &str) -> Result<MessageRecord, RunsError> {
        sqlx::query_as(
            "SELECT m.*,a.task FROM run_agent_messages m
            JOIN run_attempts a ON a.id=m.attempt_id WHERE m.id=?",
        )
        .bind(id)
        .fetch_optional(self.storage.reader())
        .await?
        .ok_or(RunsError::NotFound)
    }

    pub async fn next_message(
        &self,
        thread: &ThreadId,
    ) -> Result<Option<MessageRecord>, RunsError> {
        Ok(sqlx::query_as(
            "SELECT m.*,a.task FROM run_agent_messages m
            JOIN run_attempts a ON a.id=m.attempt_id WHERE m.thread_id=? AND m.state='waiting'
            ORDER BY m.ordinal LIMIT 1",
        )
        .bind(thread.as_str())
        .fetch_optional(self.storage.reader())
        .await?)
    }

    pub async fn message_in_flight(&self, thread: &ThreadId) -> Result<bool, RunsError> {
        Ok(sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM run_agent_messages
            WHERE thread_id=? AND state='sent')",
        )
        .bind(thread.as_str())
        .fetch_one(self.storage.reader())
        .await?)
    }

    pub async fn set_message_sent(
        &self,
        id: &str,
        operation: &OperationId,
    ) -> Result<(), RunsError> {
        let (id, operation) = (id.to_string(), operation.as_str().to_string());
        self.storage
            .write_txn_as(move |tx| {
                Box::pin(async move {
                    sqlx::query(
                        "UPDATE run_agent_messages SET state='sent',operation_id=? WHERE id=?",
                    )
                    .bind(operation)
                    .bind(id)
                    .execute(&mut *tx)
                    .await?;
                    Ok(())
                })
            })
            .await
    }

    pub async fn answer_message(
        &self,
        id: &str,
        reply: Option<String>,
        reason: Option<String>,
    ) -> Result<(), RunsError> {
        let id = id.to_string();
        self.storage
            .write_txn_as(move |tx| {
                Box::pin(async move {
                    sqlx::query(
                        "UPDATE run_agent_messages
                SET state=?,reply=?,reason=?,answered_at=? WHERE id=?",
                    )
                    .bind(if reason.is_some() {
                        "failed"
                    } else {
                        "replied"
                    })
                    .bind(reply)
                    .bind(reason)
                    .bind(Storage::now())
                    .bind(id)
                    .execute(&mut *tx)
                    .await?;
                    Ok(())
                })
            })
            .await
    }

    pub async fn message_delivered(&self, id: &str) -> Result<(), RunsError> {
        let id = id.to_string();
        self.storage
            .write_txn_as(move |tx| {
                Box::pin(async move {
                    sqlx::query(
                        "UPDATE run_agent_messages SET delivered_at=?,
                state=CASE WHEN state='failed' THEN 'failed' ELSE 'delivered' END WHERE id=?",
                    )
                    .bind(Storage::now())
                    .bind(id)
                    .execute(&mut *tx)
                    .await?;
                    Ok(())
                })
            })
            .await
    }

    pub async fn pending_for_task(
        &self,
        workflow: &WorkflowId,
        version: i64,
        task: u32,
    ) -> Result<bool, RunsError> {
        Ok(sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM run_agent_messages m
            JOIN run_attempts a ON a.id=m.attempt_id WHERE a.workflow_id=? AND a.version=?
            AND a.task=? AND m.state IN ('waiting','sent'))",
        )
        .bind(workflow.as_str())
        .bind(version)
        .bind(i64::from(task))
        .fetch_one(self.storage.reader())
        .await?)
    }

    pub async fn unfinished_messages(&self) -> Result<Vec<MessageRecord>, RunsError> {
        Ok(sqlx::query_as(
            "SELECT m.*,a.task FROM run_agent_messages m JOIN run_attempts a
            ON a.id=m.attempt_id WHERE m.delivered_at IS NULL
            ORDER BY m.thread_id,m.ordinal",
        )
        .fetch_all(self.storage.reader())
        .await?)
    }
}
