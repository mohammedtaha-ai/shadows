use crate::{
    Messages, MessagesError, prompt,
    store::{Delivery, Store},
};
use core_turns::{SendTurn, Turns, TurnsError};

impl Messages {
    pub(super) async fn dispatch_ready(
        &self,
        turns: &Turns,
        stopping: &tokio::sync::watch::Receiver<bool>,
    ) -> Result<bool, MessagesError> {
        let mut conn = self.storage.reader().acquire().await?;
        let candidates = Store::candidates(&mut conn).await?;
        drop(conn);
        let mut busy = false;
        for delivery in candidates {
            if *stopping.borrow() {
                break;
            }
            busy |= self.dispatch(turns, delivery, stopping).await?;
        }
        Ok(busy)
    }

    async fn dispatch(
        &self,
        turns: &Turns,
        delivery: Delivery,
        stopping: &tokio::sync::watch::Receiver<bool>,
    ) -> Result<bool, MessagesError> {
        let delivery = std::sync::Arc::new(delivery);
        let (prompt, model, mode) = match (&delivery.prompt, &delivery.model, &delivery.mode) {
            (Some(prompt), Some(model), Some(mode)) => {
                (prompt.clone(), model.clone(), mode.clone())
            }
            _ => {
                let agent = match self.agents.agent(&delivery.slug).await {
                    Ok(agent) if !agent.deleted => agent,
                    Ok(_) | Err(core_agents::AgentsError::NotFound(_)) => {
                        return self
                            .refuse(delivery, "the agent is no longer available")
                            .await
                            .map(|()| false);
                    }
                    Err(error) => return Err(error.into()),
                };
                let row = delivery.clone();
                self.storage
                    .write_txn_as(move |conn| {
                        Box::pin(async move {
                            let group = row.group();
                            let history = Store::messages_between(
                                conn,
                                &group,
                                row.delivered_seq,
                                row.trigger_seq,
                                prompt::MAX_MESSAGES,
                            )
                            .await?;
                            let prompt = prompt::build(
                                &Store::group(conn, &group).await?,
                                &history,
                                row.trigger_seq - row.delivered_seq,
                            );
                            let mode =
                                shadows_agent::policy::default_mode(shadows_agent::policy::AYLUL)
                                    .expect("AYLUL has a default mode")
                                    .to_owned();
                            Store::prepare(conn, &row, &prompt, &agent.default_model, &mode)
                                .await?;
                            Ok::<_, MessagesError>((prompt, agent.default_model, mode))
                        })
                    })
                    .await?
            }
        };
        let result = turns
            .send_with_default_effort(
                delivery.thread(),
                SendTurn {
                    command_id: format!("delivery:{}", delivery.id),
                    prompt,
                    model,
                    mode,
                    effort: None,
                    focus: None,
                    plan: None,
                    attachments: Vec::new(),
                    client_tab: None,
                },
            )
            .await;
        match result {
            Ok(operation) => self
                .storage
                .write_txn_as(move |conn| {
                    Box::pin(async move { Store::link(conn, &delivery, operation.as_str()).await })
                })
                .await
                .map(|()| false),
            Err(_) if *stopping.borrow() => Ok(false),
            Err(TurnsError::ThreadBusy) => Ok(true),
            Err(TurnsError::RuntimeStopping) => Ok(false),
            Err(TurnsError::Storage(error)) => Err(error.into()),
            Err(TurnsError::Start(core_turns::StartError::Storage(error))) => Err(error.into()),
            Err(TurnsError::Start(core_turns::StartError::RuntimeStopping)) => Ok(false),
            Err(error) => {
                tracing::debug!(delivery_id = %delivery.id, "messages.delivery_refused");
                let reason = match error {
                    TurnsError::SettingNotOffered { .. } => {
                        "the saved model or mode is unavailable"
                    }
                    TurnsError::ModeNotAllowed(_) => "the project does not allow the mode",
                    _ => "the turn could not be started",
                };
                self.refuse(delivery, reason).await.map(|()| false)
            }
        }
    }

    async fn refuse(
        &self,
        delivery: std::sync::Arc<Delivery>,
        reason: &'static str,
    ) -> Result<(), MessagesError> {
        self.storage
            .write_txn_as(move |conn| {
                Box::pin(async move {
                    let draft =
                        Store::notice(format!("{} could not answer: {reason}", delivery.name));
                    Store::append(
                        conn,
                        &delivery.group(),
                        draft,
                        Some(delivery.cause_seq),
                        None,
                    )
                    .await?;
                    Store::finish(conn, &delivery, false).await
                })
            })
            .await
    }
}
