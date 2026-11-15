//! Plain questions use the executor's thread without opening another attempt.
use crate::store::messages::MessageRecord;
use crate::{AgentMessage, AttemptId, AttemptState, Runs, RunsError};
use core_grants::Grant;
use core_grants::TaskScope;
use core_operations::OperationId;
use core_plans::WorkflowId;
use core_threads::ThreadId;
use core_turns::SendTurn;
use std::sync::Arc;
use std::time::Duration;

impl Runs {
    pub async fn message_agent(
        &self,
        grant: &Grant,
        workflow: &WorkflowId,
        task: u32,
        text: String,
    ) -> Result<AgentMessage, RunsError> {
        let plan = self.planner_plan(grant, workflow).await?;
        let text = text.trim();
        if text.is_empty() {
            return Err(RunsError::EmptyMessage);
        }
        let attempt = self
            .attempts(workflow, plan.version, task)
            .await?
            .into_iter()
            .next()
            .ok_or(RunsError::NotFound)?;
        let lock = self.message_lock(&attempt.thread_id).await;
        let held = lock.lock().await;
        let (message, _) = self.inner.store.add_message(&attempt, text).await?;
        drop(held);
        self.pump(&attempt.thread_id).await;
        self.inner.store.message(message.id.as_str()).await?.view()
    }

    pub(crate) async fn message_lock(&self, thread: &ThreadId) -> Arc<tokio::sync::Mutex<()>> {
        self.inner
            .messaging
            .lock()
            .await
            .entry(thread.clone())
            .or_insert_with(|| Arc::new(tokio::sync::Mutex::new(())))
            .clone()
    }

    pub(crate) fn pump(
        &self,
        thread: &ThreadId,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>> {
        let (runs, thread) = (self.clone(), thread.clone());
        Box::pin(async move {
            if let Err(error) = runs.pump_messages(&thread).await {
                tracing::warn!(%thread,%error,"runs.message_dispatch_failed");
            }
        })
    }

    async fn pump_messages(&self, thread: &ThreadId) -> Result<(), RunsError> {
        let lock = self.message_lock(thread).await;
        let _held = lock.lock().await;
        loop {
            if self.inner.store.message_in_flight(thread).await?
                || self.inner.store.dispatch_barrier(thread).await?
                || self.thread_busy(thread).await?
            {
                return Ok(());
            }
            let Some(message) = self.inner.store.next_message(thread).await? else {
                return Ok(());
            };
            match self.send_agent_message(&message).await {
                Ok(operation) => {
                    let (runs, message) = (self.clone(), message.clone());
                    tokio::spawn(async move {
                        runs.watch_message(message, operation).await;
                    });
                    return Ok(());
                }
                Err(error) => {
                    self.inner
                        .store
                        .answer_message(&message.id, None, Some(error.to_string()))
                        .await?;
                    self.notify_reply_logged(&message.id).await;
                }
            }
        }
    }

    /// Whether a turn of any kind runs on `thread` now.
    pub(crate) async fn thread_busy(&self, thread: &ThreadId) -> Result<bool, RunsError> {
        Ok(self
            .inner
            .turns
            .operations(thread)
            .await
            .map_err(|e| RunsError::Unavailable(e.to_string()))?
            .iter()
            .any(|op| op.finished_at.is_none()))
    }

    async fn send_agent_message(&self, message: &MessageRecord) -> Result<OperationId, RunsError> {
        let attempt = self
            .inner
            .store
            .get(&AttemptId::from_recorded(message.attempt_id.clone()))
            .await?;
        self.inner
            .harness
            .open_executor(
                &attempt.thread_id,
                TaskScope {
                    workflow: attempt.workflow_id.as_str().into(),
                    version: attempt.version,
                    task: attempt.task,
                },
                attempt.model.clone(),
            )
            .await
            .map_err(|e| RunsError::Unavailable(e.to_string()))?;
        let operation = self
            .inner
            .turns
            .send_with_default_effort(
                attempt.thread_id.clone(),
                SendTurn {
                    command_id: format!("message-{}", message.id),
                    prompt: format!(
                        "The Planner asks you, about task T{}, without changing it:\n{}\n\
                Answer in plain words. Change no file and make no commit for this.",
                        attempt.task, message.message
                    ),
                    attachments: vec![],
                    model: attempt.model,
                    mode: crate::start::MODE.into(),
                    effort: None,
                    focus: None,
                    plan: None,
                    client_tab: None,
                },
            )
            .await
            .map_err(|e| RunsError::Unavailable(e.to_string()))?;
        self.inner
            .store
            .set_message_sent(&message.id, &operation)
            .await?;
        Ok(operation)
    }

    async fn watch_message(&self, message: MessageRecord, operation: OperationId) {
        let thread = ThreadId::from_recorded(message.thread_id.clone());
        loop {
            match self.inner.turns.operations(&thread).await {
                Ok(operations) => {
                    if let Some(ended) = operations
                        .into_iter()
                        .find(|op| op.id == operation && op.finished_at.is_some())
                    {
                        let reply = if ended.status_kind == "Completed" {
                            self.last_message(&thread, &operation).await
                        } else {
                            None
                        };
                        let reason = if reply.is_none() {
                            Some(
                                ended
                                    .failure_reason
                                    .unwrap_or_else(|| "the agent gave no reply".into()),
                            )
                        } else {
                            None
                        };
                        if let Err(error) = self
                            .inner
                            .store
                            .answer_message(&message.id, reply, reason)
                            .await
                        {
                            tracing::warn!(id=%message.id,%error,"runs.reply_save_failed");
                            return;
                        }
                        break;
                    }
                }
                Err(error) => tracing::warn!(id=%message.id,%error,"runs.reply_read_failed"),
            }
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
        self.notify_reply_logged(&message.id).await;
        if let Err(error) = self.close_if_idle(&message).await {
            tracing::warn!(id=%message.id,%error,"runs.reply_close_failed");
        }
        self.pump(&thread).await;
    }

    pub(crate) async fn notify_reply_logged(&self, id: &str) {
        let result = async {
            let message = self.inner.store.message(id).await?;
            self.notify_reply(&message).await
        }
        .await;
        if let Err(error) = result {
            tracing::warn!(%id,%error,"runs.reply_notice_pending");
        }
    }

    async fn close_if_idle(&self, message: &MessageRecord) -> Result<(), RunsError> {
        let thread = ThreadId::from_recorded(message.thread_id.clone());
        let lock = self.message_lock(&thread).await;
        let _held = lock.lock().await;
        let attempt = self
            .inner
            .store
            .get(&AttemptId::from_recorded(message.attempt_id.clone()))
            .await?;
        if self.inner.store.dispatch_barrier(&thread).await?
            || self.thread_busy(&thread).await?
            || self.inner.store.next_message(&thread).await?.is_some()
        {
            return Ok(());
        }
        self.inner
            .harness
            .release_executor(&thread)
            .await
            .map_err(|e| RunsError::Unavailable(e.to_string()))?;
        let mark = self
            .inner
            .store
            .mark(&attempt.workflow_id, attempt.version, attempt.task)
            .await?;
        if mark.is_some_and(|m| m.0) || attempt.state != AttemptState::Committed {
            self.inner.store.release(&attempt.id).await?;
        }
        Ok(())
    }

    pub(crate) async fn release_after_mark(&self, attempt: &AttemptId, thread: &ThreadId) {
        let result = async {
            let lock = self.message_lock(thread).await;
            let _held = lock.lock().await;
            let current = self.inner.store.get(attempt).await?;
            if !self
                .inner
                .store
                .pending_for_task(&current.workflow_id, current.version, current.task)
                .await?
            {
                self.release_locked(attempt, thread).await?;
            }
            Ok::<_, RunsError>(())
        }
        .await;
        if let Err(error) = result {
            tracing::warn!(%attempt,%error,"runs.release_failed");
        }
    }

    pub(crate) async fn recover_messages(&self) -> Result<(), RunsError> {
        for message in self.inner.store.unfinished_messages().await? {
            let thread = ThreadId::from_recorded(message.thread_id.clone());
            match message.state.as_str() {
                "sent" => {
                    let operations = self
                        .inner
                        .turns
                        .operations(&thread)
                        .await
                        .map_err(|e| RunsError::Unavailable(e.to_string()))?;
                    let ended = operations
                        .into_iter()
                        .find(|op| Some(op.id.as_str()) == message.operation_id.as_deref());
                    let reply = match &ended {
                        Some(op) if op.status_kind == "Completed" && op.finished_at.is_some() => {
                            self.last_message(&thread, &op.id).await
                        }
                        _ => None,
                    };
                    let reason = if reply.is_none() {
                        Some(
                            ended
                                .and_then(|op| op.failure_reason)
                                .unwrap_or_else(|| "the reply turn was interrupted".into()),
                        )
                    } else {
                        None
                    };
                    self.inner
                        .store
                        .answer_message(&message.id, reply, reason)
                        .await?;
                    self.notify_reply_logged(&message.id).await;
                }
                "replied" | "failed" => self.notify_reply_logged(&message.id).await,
                "waiting" => self.pump(&thread).await,
                _ => {}
            }
            self.close_if_idle(&message).await?;
            self.pump(&thread).await;
        }
        Ok(())
    }
}
