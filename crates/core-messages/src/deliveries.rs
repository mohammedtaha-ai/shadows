use crate::{
    Messages, MessagesError, prompt,
    store::{Delivery, Store},
};
use core_turns::{SendTurn, Turns, TurnsError};
use std::{collections::HashMap, sync::Arc};

/// How many dispatches of one delivery may fail before it is refused.
const TRIES: u32 = 3;

/// The worker's count of failed starts per delivery id. It lives in memory
/// only: a restart gives a stuck delivery its tries again.
pub(super) type Failures = HashMap<String, u32>;

/// Where one dispatch attempt left its delivery.
enum Started {
    /// Nothing to link: refused, or left to retry (`busy` when its member is).
    Settled { busy: bool },
    /// Its turn runs as this operation and still needs the link.
    Turn(String),
}

/// What one dispatch pass leaves for the worker.
#[derive(Default)]
pub(super) struct Dispatched {
    /// Something is left to retry after a pause.
    pub busy: bool,
    /// A started turn is not linked yet, so its ending must not be consumed.
    pub unlinked: bool,
}

impl Messages {
    /// One delivery's failure never stops the others. A start that fails is
    /// counted and refused with a notice once it has failed `TRIES` times. A
    /// link that fails is retried without a bound, since its turn already runs
    /// and only the link lets its ending be recorded. Any failure reports the
    /// pass busy so the worker retries after a pause.
    pub(super) async fn dispatch_ready(
        &self,
        turns: &Turns,
        stopping: &tokio::sync::watch::Receiver<bool>,
        failures: &mut Failures,
    ) -> Result<Dispatched, MessagesError> {
        let mut conn = self.storage.reader().acquire().await?;
        let candidates = Store::candidates(&mut conn).await?;
        drop(conn);
        failures.retain(|id, _| candidates.iter().any(|delivery| &delivery.id == id));
        let mut pass = Dispatched::default();
        for delivery in candidates {
            if *stopping.borrow() {
                break;
            }
            let delivery = Arc::new(delivery);
            match self.start(turns, delivery.clone(), stopping).await {
                Ok(Started::Settled { busy: waiting }) => {
                    failures.remove(&delivery.id);
                    pass.busy |= waiting;
                }
                Ok(Started::Turn(operation)) => {
                    failures.remove(&delivery.id);
                    let id = delivery.id.clone();
                    let linked = self
                        .storage
                        .write_txn_as(move |conn| {
                            Box::pin(async move { Store::link(conn, &delivery, &operation).await })
                        })
                        .await;
                    if let Err(error) = linked {
                        pass.busy = true;
                        pass.unlinked = true;
                        tracing::warn!(delivery_id = %id, %error, "messages.link_failed");
                    }
                }
                Err(error) => {
                    pass.busy = true;
                    let tries = failures.entry(delivery.id.clone()).or_default();
                    *tries += 1;
                    tracing::warn!(
                        delivery_id = %delivery.id, tries = *tries, %error,
                        "messages.delivery_failed"
                    );
                    if *tries < TRIES {
                        continue;
                    }
                    let id = delivery.id.clone();
                    match self.refuse(delivery, "the turn could not be started").await {
                        Ok(()) => {
                            failures.remove(&id);
                        }
                        Err(error) => {
                            tracing::warn!(delivery_id = %id, %error, "messages.refusal_failed");
                        }
                    }
                }
            }
        }
        Ok(pass)
    }

    async fn start(
        &self,
        turns: &Turns,
        delivery: Arc<Delivery>,
        stopping: &tokio::sync::watch::Receiver<bool>,
    ) -> Result<Started, MessagesError> {
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
                            .map(|()| Started::Settled { busy: false });
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
                            let prompt = Self::delivery_prompt(
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
        let settled = |busy| Ok(Started::Settled { busy });
        match result {
            Ok(operation) => Ok(Started::Turn(operation.as_str().to_owned())),
            Err(_) if *stopping.borrow() => settled(false),
            Err(TurnsError::ThreadBusy) => settled(true),
            Err(TurnsError::RuntimeStopping) => settled(false),
            Err(TurnsError::Storage(error)) => Err(error.into()),
            Err(TurnsError::Start(core_turns::StartError::Storage(error))) => Err(error.into()),
            Err(TurnsError::Start(core_turns::StartError::RuntimeStopping)) => settled(false),
            Err(error) => {
                tracing::debug!(delivery_id = %delivery.id, "messages.delivery_refused");
                let reason = match error {
                    TurnsError::SettingNotOffered { .. } => {
                        "the saved model or mode is unavailable"
                    }
                    TurnsError::ModeNotAllowed(_) => "the project does not allow the mode",
                    _ => "the turn could not be started",
                };
                self.refuse(delivery, reason)
                    .await
                    .map(|()| Started::Settled { busy: false })
            }
        }
    }

    async fn refuse(
        &self,
        delivery: Arc<Delivery>,
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
