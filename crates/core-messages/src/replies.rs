use crate::{
    MessageKind, Messages, MessagesError, Speaker,
    store::{Draft, Store},
};
use core_events::{EndedOutcome, EndedTurn, Events};
use core_threads::{ThreadEntryKind, Threads};
use core_turns::Turns;
use std::{sync::Arc, time::Duration};
use tokio::{
    sync::{Mutex, watch},
    task::JoinHandle,
};

pub struct MessagesWorker {
    stop: watch::Sender<bool>,
    task: Mutex<Option<JoinHandle<()>>>,
}

impl MessagesWorker {
    pub fn request_stop(&self) {
        self.stop.send_replace(true);
    }

    /// The runtime has finished its bounded turn shutdown. A delivery link
    /// still in flight can be reconstructed from its command on restart.
    pub async fn finish(&self) {
        if let Some(task) = self.task.lock().await.take() {
            task.abort();
            let _ = task.await;
        }
    }
}

impl Drop for MessagesWorker {
    fn drop(&mut self) {
        if let Some(task) = self.task.get_mut().take() {
            task.abort();
        }
    }
}

impl Messages {
    pub fn listen(
        self: &Arc<Self>,
        turns: Turns,
        threads: Arc<Threads>,
        events: Arc<Events>,
    ) -> MessagesWorker {
        let (stop, mut stopping) = watch::channel(false);
        let messages = self.clone();
        let mut committed = self.storage.watch_committed();
        let task = tokio::spawn(async move {
            while !*stopping.borrow() {
                match messages.work(&turns, &threads, &events, &stopping).await {
                    Ok((true, _)) => continue,
                    Ok((false, busy)) => {
                        tokio::select! {
                            _ = stopping.changed() => {},
                            _ = committed.changed() => {},
                            _ = messages.wake.notified() => {},
                            _ = tokio::time::sleep(Duration::from_millis(250)), if busy => {},
                        }
                    }
                    Err(error) => {
                        tracing::warn!(%error, "messages.worker_retry");
                        tokio::select! {
                            _ = stopping.changed() => {},
                            _ = tokio::time::sleep(Duration::from_millis(250)) => {},
                        }
                    }
                }
            }
        });
        MessagesWorker {
            stop,
            task: Mutex::new(Some(task)),
        }
    }

    async fn work(
        &self,
        turns: &Turns,
        threads: &Threads,
        events: &Events,
        stopping: &watch::Receiver<bool>,
    ) -> Result<(bool, bool), MessagesError> {
        // Reconcile starts before reading endings so a fast turn's event cannot outrun its link.
        let busy = self.dispatch_ready(turns, stopping).await?;
        if *stopping.borrow() {
            return Ok((false, false));
        }
        let mut conn = self.storage.reader().acquire().await?;
        let cursor = Store::cursor(&mut conn).await?;
        drop(conn);
        let ended = events.ended_turns_after(cursor, 100).await?;
        let had_work = !ended.is_empty();
        for end in ended {
            self.record_end(turns, threads, end).await?;
        }
        Ok((had_work, busy))
    }

    async fn record_end(
        &self,
        turns: &Turns,
        threads: &Threads,
        end: EndedTurn,
    ) -> Result<(), MessagesError> {
        let mut conn = self.storage.reader().acquire().await?;
        let delivery = Store::by_operation(&mut conn, end.operation_id.as_str()).await?;
        drop(conn);
        let draft = if let Some(delivery) = &delivery {
            let reason = match end.outcome {
                EndedOutcome::Completed => None,
                EndedOutcome::Failed => {
                    let operation = turns
                        .operations(&end.thread_id)
                        .await?
                        .into_iter()
                        .find(|operation| operation.id == end.operation_id);
                    Some(
                        match (
                            operation
                                .as_ref()
                                .and_then(|op| op.failure_stage.as_deref()),
                            operation
                                .as_ref()
                                .and_then(|op| op.failure_reason.as_deref()),
                        ) {
                            (_, Some("the harness exited during the turn")) => {
                                "the harness exited during the turn"
                            }
                            (_, Some("the harness cancelled the turn on its own")) => {
                                "the harness cancelled the turn"
                            }
                            (Some("Prepare"), _) => "the harness could not prepare the turn",
                            (Some("Spawn"), _) => "the harness could not start the turn",
                            _ => "the turn failed while running",
                        },
                    )
                }
                EndedOutcome::Cancelled => Some("the turn was cancelled"),
                EndedOutcome::Interrupted => Some("the turn was interrupted"),
            };
            if let Some(reason) = reason {
                Some(Store::notice(format!(
                    "{} could not answer: {reason}",
                    delivery.name
                )))
            } else {
                let body = threads
                    .entries(&end.thread_id)
                    .await?
                    .into_iter()
                    .filter(|entry| {
                        entry.kind == ThreadEntryKind::AgentMessage
                            && entry.operation_id.as_deref() == Some(end.operation_id.as_str())
                    })
                    .map(|entry| entry.body)
                    .collect::<Vec<_>>()
                    .join("\n\n");
                if body.trim().is_empty() {
                    Some(Store::notice(format!(
                        "{} finished without a reply",
                        delivery.name
                    )))
                } else {
                    Some(Draft {
                        speaker: Speaker::Agent {
                            slug: delivery.slug.clone(),
                            name: delivery.name.clone(),
                        },
                        to: Vec::new(),
                        kind: MessageKind::Chat,
                        body,
                    })
                }
            }
        } else {
            None
        };
        self.storage
            .write_txn_as(move |conn| {
                Box::pin(async move {
                    if let (Some(delivery), Some(mut draft)) = (delivery, draft) {
                        let group = delivery.group();
                        let answered = draft.kind == MessageKind::Chat;
                        if answered {
                            let members = Store::group(conn, &group).await?.members;
                            draft.to = crate::mentions::addresses(
                                &draft.body,
                                &members,
                                Some(&delivery.slug),
                            );
                        }
                        let message = Store::append(
                            conn,
                            &group,
                            draft,
                            Some(delivery.cause_seq),
                            Some(end.operation_id.as_str()),
                        )
                        .await?;
                        if answered {
                            crate::bounds::schedule(
                                conn,
                                &message,
                                delivery.cause_seq,
                                delivery.round + 1,
                            )
                            .await?;
                        }
                        Store::finish(conn, &delivery, answered).await?;
                    }
                    Store::advance(conn, end.seq).await
                })
            })
            .await
    }
}
